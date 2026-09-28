# ADR 012: Engine-Tooling Separation and Cargo Workspace Topology

## Status

Accepted

## Context

Addresses [REQ-02: Storage and Retrieval](../specs/02-REQ-STORAGE-AND-RETRIEVAL.md), [REQ-05: CLI and Configuration](../specs/05-REQ-CLI-AND-CONFIG.md), and [ADR-006: Unified Binary Architecture](./006-unified-binary-architecture.md). Grounded in the *Compiled Intent* research paper (Figures 1 & 2).

During early development, `compiled-intent` was organized as a single crate (`src/lib.rs` and `src/main.rs`). Over successive milestones, the codebase evolved from a pure offline indexing and retrieval engine into a comprehensive suite supporting:
1. **Shift-Left Proactive Synthesis** (`intent ask`): Generating compliant code guidance from intent context using generative LLM streams.
2. **Shift-Right Reactive Auditing** (`intent audit`): Intercepting Git working tree diffs and evaluating architectural compliance using LLM structured JSON verdicts.
3. **Headless IDE Agent Integration** (`intent serve`): Exposing an MCP server over standard I/O.

As a result, deterministic storage and vector retrieval logic (LanceDB, Tantivy, AST parent-child chunking) currently share the same module hierarchy with generative LLM prompt harnesses, Git subprocess wrappers, and terminal formatting logic.

## The Dilemma

How should we separate the core deterministic **Compitent Engine** from the developer-facing **Intent Tooling** without breaking the single-binary distribution model mandated by [ADR-006](./006-unified-binary-architecture.md)?

## Options Evaluated

### Option 1: Status Quo (Monolithic Single Crate)
- **Pros**: Zero package management overhead; all modules live under `src/`.
- **Cons**: Severe boundary erosion. Storage and indexer modules can inadvertently acquire dependencies on LLM client structs, Git subprocess calls, or CLI formatting code. The core retrieval engine cannot be embedded by external Rust applications without pulling in `clap`, Git wrappers, and ANSI terminal dependencies.

### Option 2: Split Independent Repositories
- **Pros**: Complete physical separation of repositories.
- **Cons**: Massive coordination overhead. Version synchronization, cross-repository PRs, CI duplication, and developer friction. Violates the core development velocity of the project.

### Option 3: Two-Crate Cargo Workspace (`crates/engine` and `crates/cli`)
- **Pros**:
  - **Enforced Architectural Direction**: The engine crate (`compitent-engine`) has zero knowledge of Git, terminal formatting, or generative LLMs. Dependency flows strictly one-way: `cli -> engine`.
  - **Preserves ADR-006**: The workspace produces the single canonical `intent` binary via the `cli` crate. End-user installation, pre-commit scripts, and IDE MCP setups remain completely unchanged.
  - **Embeddability**: Any third-party Rust service or background daemon can embed `compitent-engine` as a pure, lightweight library dependency.
  - **Atomic Refactoring**: Code changes, migrations, and integration tests stay together in one monorepo with unified `make verify` CI checks.
- **Cons**: Requires restructuring the root `Cargo.toml` into a workspace configuration and updating internal import paths.

## Decision

We choose **Option 3: Two-Crate Cargo Workspace (`crates/engine` and `crates/cli`)**.

### Target Project Layout

```text
compiled-intent/
├── Cargo.toml                    # Workspace root Cargo.toml
├── Makefile                      # make verify, fmt, clippy, test
├── docs/                         # ADRs, Specs, RFCs
├── tests/                        # Top-level workspace integration tests
│
├── crates/
│   ├── engine/                   # [CRATE: compitent-engine (Library)]
│   │   ├── Cargo.toml            # Storage, AST chunking, embeddings, hybrid retrieval
│   │   └── src/
│   │       ├── lib.rs            # Public API: SearchEngine, IndexerPipeline, Storage
│   │       ├── error.rs          # EngineError (LanceDB, Arrow, IO, Indexing)
│   │       ├── config.rs         # EngineConfig (storage paths, embedding providers)
│   │       ├── telemetry.rs      # Engine metrics & tracing
│   │       ├── indexer/          # AST Parent-Child Chunker, FastEmbed/Ollama, Manifest
│   │       │   ├── mod.rs
│   │       │   ├── chunker.rs
│   │       │   ├── embedder.rs
│   │       │   ├── manifest.rs
│   │       │   └── pipeline.rs
│   │       └── retrieval/        # LanceDB schema, Tantivy FTS, Hybrid Search & Scoring
│   │           ├── mod.rs
│   │           ├── schema.rs
│   │           ├── lancedb.rs
│   │           └── search.rs
│   │
│   └── cli/                      # [CRATE: intent (Binary)]
│       ├── Cargo.toml            # Depends on `compitent-engine`, Clap, Reqwest, Stdio
│       └── src/
│           ├── main.rs           # Entrypoint: Clap root router & default fallback to `serve`
│           ├── config.rs         # CLI / User config (LLM configs, API keys, profiles)
│           ├── llm/              # Generative LLM clients (Ollama, OpenAI streaming & chat)
│           │   ├── mod.rs
│           │   ├── client.rs
│           │   └── types.rs
│           ├── mcp/              # MCP JSON-RPC protocol daemon over stdio
│           │   ├── mod.rs
│           │   ├── server.rs
│           │   └── protocol.rs
│           ├── audit/            # Shift-Right Reactive Auditing & Git diff inspection
│           │   ├── mod.rs
│           │   ├── git.rs
│           │   └── reporter.rs
│           └── commands/         # Subcommand handlers
│               ├── mod.rs
│               ├── index.rs
│               ├── search.rs
│               ├── ask.rs
│               ├── audit.rs
│               └── serve.rs
```

## The Five Architectural Boundary Invariants

1. **System Coprocessor vs. Application Domain (Paper Figures 1 & 2)**
   - The *Compitent Engine* is the architectural coprocessor: its sole responsibility is parsing, indexing, and serving AST-bounded constraint slices.
   - The *Intent Tooling* is the application layer that integrates the coprocessor into developer workflows: proactive generation (`ask`), reactive validation (`audit`), and IDE integration (`serve`).

2. **Determinism vs. Stochasticity (The LLM Boundary)**
   - The Engine is 100% deterministic and reproducible. Given a document and a query, vector embeddings and BM25 scores are deterministic. It contains **zero generative LLM dependencies**.
   - Generative LLM prompts, token streaming, temperature, and stochastic JSON completion parsing belong strictly in the Tooling layer.

3. **Subprocess & System Coupling (The Git Boundary)**
   - The Engine has zero coupling to Git, commits, working trees, or terminal ANSI colors. It operates exclusively on memory buffers, file paths, Arrow batches, and math.
   - Git subprocess invocation (`git diff`), commit message extraction, and user-facing terminal UI live exclusively in the Tooling layer.

4. **Reusability & Embeddability (Library First)**
   - `compitent-engine` is a pure Rust library crate. External systems can embed it directly without bringing along CLI or subprocess dependencies.

5. **Preservation of ADR-006 (Unified Binary Experience)**
   - The workspace compiles a single binary: `intent`. For end users, IDEs, and CI pipelines, the CLI ergonomics, subcommand flags, and zero-config `serve` fallback remain completely identical.

## Refactoring Guardrails & Constraints

- **No Logic Rewrites**: Code refactoring must proceed exclusively by *relocation* of exact tested logic, never reimplementation from memory.
- **Zero Clippy Bypasses**: No `#[allow(clippy::...)]` attributes. All code must naturally satisfy clippy (`-D warnings`), including function length (<=50 lines) and argument counts (<=7).
- **Continuous Test Verification**: All 69 automated tests must remain green across every migration step.
