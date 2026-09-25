# ADR 008: Decoupled Local Embedding Provider Architecture

## Status

Accepted

## Context

Addresses [REQ-02: Storage and Retrieval](../specs/02-REQ-STORAGE-AND-RETRIEVAL.md) and [REQ-05: CLI and Configuration](../specs/05-REQ-CLI-AND-CONFIG.md).

Generating dense semantic embeddings for Markdown AST child chunks and search queries requires a high-dimensional transformer encoder (such as 768-D `nomic-embed-text` or 384-D `all-minilm-l6-v2`).

As an edge architectural coprocessor and pre-commit gate, `compiled-intent` must operate with minimal latency, zero unnecessary operational friction, and a predictable deployment footprint across diverse developer machines.

## The Dilemma

How should `compiled-intent` execute embedding inference? Should it bundle model weights and an in-process inference runtime directly into the compiled Rust binary, rely on a remote cloud API, or decouple execution to a local inference provider via HTTP (Ollama)?

## Options Evaluated

### Option 1: Bundled In-Process Model & Runtime (ONNX Runtime / Candle / FastEmbed)

- **Pros**: Produces a fully self-contained single binary with zero external process dependencies. In-process SIMD tensor evaluation delivers ~2–5ms embedding generation without HTTP socket overhead.
- **Cons**: 
  - **Severe Binary Bloat**: High-quality embedding models like `nomic-embed-text` require 140MB–280MB of weight data. Embedding weights directly expands the lean ~15MB Rust binary to 300+ MB.
  - **Packaging & Cross-Compilation Friction**: Statically linking C++ ONNX Runtime or libtorch introduces severe build friction across target triples (`aarch64-apple-darwin`, `x86_64-unknown-linux-musl`, etc.).
  - **Loss of Model Agency**: The binary either locks the user into a single hardcoded model architecture or must implement a complex, bespoke weight downloader and cache manager in Rust.
  - **Hardware Acceleration Complexity**: Supporting Apple Silicon Metal, CUDA, and CPU SIMD in-process requires fragile platform-specific driver bindings.

### Option 2: Cloud-Only Embeddings API (OpenAI / Cohere / Voyage AI)

- **Pros**: Zero local compute or VRAM usage; lightweight binary.
- **Cons**: Completely violates the core offline/air-gapped design requirement for local CI/CD pre-commit hooks. Introduces recurring API costs, external network latency, token rate limits, and leaks proprietary codebase architecture over public networks.

### Option 3: Decoupled Local Inference Provider via HTTP (Ollama)

- **Pros**:
  - **Ultra-Lean Binary Footprint**: Keeps the compiled `intent` binary under 15MB with pure, safe Rust dependencies.
  - **User Freedom & Model Evolution**: Users configure their preferred embedding model (`nomic-embed-text`, `bge-m3`, `mxbai-embed-large`, etc.) and dimensionality via simple `config.toml` keys.
  - **Offloaded Hardware Optimization**: Delegates GPU VRAM management, Metal shader compilation, and quantized weight execution to Ollama, which is already optimized for local developer workstations.
  - **Standardized Local API**: Standard HTTP/JSON interface (`/api/embeddings`) allows easy swapping or mocking in integration tests.
- **Cons**: Requires the user to have Ollama (or a compatible local endpoint) running. Incurs ~30–45ms of HTTP serialization and IPC overhead per batch.

## Decision

We choose **Option 3: Decoupled Local Inference Provider via HTTP (Ollama)**.

We explicitly prioritize **binary ergonomics, user model choice, and cross-platform maintainability** over saving 30ms of local inference latency. For an agent coprocessor where downstream LLM token generation takes 1,000ms–3,000ms, a 35ms embedding call is virtually imperceptible, whereas a 300MB binary bloat and rigid model lock-in represent severe developer friction.

## Rejected Alternatives

- **In-Process ONNX / Candle Bundling**: Rejected due to binary bloat (300+ MB), loss of user model configurability, and cross-compilation complexity.
- **Cloud API Only**: Rejected due to offline/air-gapped pre-commit requirements and code privacy.

## Consequences & Trade-offs

- **Positive**: Single lean ~15MB binary, rapid compile times, zero C++ runtime dependencies, and complete user freedom to select embedding models.
- **Accepted Trade-off**: The user must install and start Ollama locally.
- **Compliance Invariant**: When the configured endpoint is unreachable, the engine MUST emit an actionable, user-friendly diagnostic on `stderr` specifying the exact URL and failing model, rather than panicking or failing silently.
