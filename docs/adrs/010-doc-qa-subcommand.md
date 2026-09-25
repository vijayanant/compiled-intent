# ADR 010: Document Q&A Subcommand (`intent ask`)

## Status

Accepted

## Context

Addresses [REQ-02: Storage and Retrieval](../specs/02-REQ-STORAGE-AND-RETRIEVAL.md) and [REQ-05: CLI and Configuration](../specs/05-REQ-CLI-AND-CONFIG.md).

While automated coding agents access indexed project documentation and ADRs over the Model Context Protocol (`intent serve`), developers working in the terminal often want quick answers to questions about the codebase's design and documentation without searching manually (e.g. `intent ask "Why did we choose LanceDB over SQLite?"`).

## The Dilemma

How should question-answering over indexed markdown documents be delivered to developers? Should it be delegated to an external wrapper script, bundle an in-process LLM runtime into Rust, or be added as a native subcommand in the `intent` CLI?

## Options Evaluated

### Option 1: External Wrapper Script (Python / Shell / Plugin)

- **Pros**: Keeps the `intent` Rust binary strictly scoped to retrieval.
- **Cons**: Adds developer friction. Requires managing extra dependencies (Python, Node.js, `curl`/`jq`). Developers have to install and maintain multiple tools. Generic wrappers also easily miss AST parent chunk structure and dump raw chunks into generic prompts.

### Option 2: In-Process Model Bundling (Candle / ONNX Runtime)

- **Pros**: Fully self-contained local generation without external process dependencies.
- **Cons**: Extreme binary bloat (several gigabytes for generative LLMs). Violates [ADR-008](./008-decoupled-local-embedding-provider.md) and creates heavy cross-compilation and hardware acceleration maintenance overhead.

### Option 3: Native Subcommand with Decoupled HTTP Inference (`intent ask`)

- **Pros**:
  - **Single Binary Ergonomics**: Directly aligns with [ADR-006](./006-unified-binary-architecture.md). A developer installs one executable (`intent`) that handles indexing, raw search, headless MCP serving, and doc Q&A.
  - **Zero Binary Bloat**: Reuses the existing asynchronous `reqwest` and `tokio` runtime established in ADR-008. Adds zero new external crate dependencies and keeps the binary under 15MB.
  - **Accurate Grounding**: Uses the existing search pipeline (Parent Chunk Collapsing and Adaptive Score Tolerance $\tau = 0.15$) and sends the retrieved sections to a local (Ollama) or OpenAI-compatible endpoint.
  - **Real-Time Token Streaming**: Streams generated tokens directly to `stdout` in the terminal.
- **Cons**: Adds an extra subcommand and chat streaming parser to the CLI.

## Decision

We choose **Option 3: Native Subcommand with Decoupled HTTP Inference (`intent ask`)**.

The `intent` CLI exposes the `ask` subcommand:

```bash
intent ask <question> [OPTIONS]
```

### Execution Flow

1. **Retrieval**: Searches the index using hybrid search with Parent Chunk Collapsing and Adaptive Score Tolerance ($\tau = 0.15$).
2. **Relevance Gating**: If no chunks pass the score threshold, the command reports that no relevant documentation was found, avoiding ungrounded answers.
3. **Prompt Formatting**: Formats a clean prompt including the retrieved section headings, source paths, and the user's question, instructing the model to answer based strictly on the provided context.
4. **Decoupled Inference**: Sends the request to the configured HTTP LLM endpoint (defaulting to Ollama `/api/chat`) and streams tokens directly to `stdout`.
5. **Attribution Footer**: Optionally prints the sources consulted and their relevance scores.

## Consequences & Trade-offs

- **Positive**: Clean developer experience (`intent ask "<question>"`), zero extra runtime dependencies, preserves the <15MB binary size, and adheres to ADR-006 and ADR-008.
- **Accepted Trade-off**: Requires an accessible local inference daemon (Ollama) or compatible HTTP endpoint. When the endpoint is offline, `intent ask` emits a clear error with the target URL.
