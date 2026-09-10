# ADR 001: Modular Architecture and MCP Protocol Isolation

## Status
Accepted

## Context
Compitent originally had a flat, monolithic Rust structure (`main.rs`, `engine.rs`, `mcp_server.rs`). As configuration files, markdown AST chunking, and incremental LanceDB indexing were introduced, `engine.rs` risked becoming an unmaintainable God Object.

Furthermore, Compitent acts as an MCP (Model Context Protocol) server over standard input/output (`stdio`). However, the core intelligence (semantic retrieval, embedding generation, vector database querying) must remain completely independent of the MCP wire format so the engine can be used as a standalone library, CLI tool (`intent`), or pre-commit interceptor.

Finally, as a tool designed to enforce Architectural Decision Records (ADRs), we must "dogfood" our own product: the engine must be governed by its own ADRs.

## Decision
1. **Directory Structure:** Refactor the codebase into a modular Library/Binary structure:
   - `src/lib.rs`: The public library API and re-exports.
   - `src/main.rs`: The headless MCP server daemon entrypoint.
   - `src/bin/cli.rs`: The human-friendly and CI/CD CLI tool (`intent`).
   - `src/config/`: Engine configuration loading and defaults.
   - `src/engine/`: Core retrieval and storage orchestration.
   - `src/indexer/`: Markdown AST chunking, differential manifest tracking, and Arrow/LanceDB storage.
   - `src/mcp/`: JSON-RPC protocol transport and tool dispatch.

2. **Separation of Concerns:**
   - The `indexer` module MUST NOT know about the Model Context Protocol (MCP).
   - The `mcp` module MUST NOT directly interact with LanceDB or filesystem storage; it must route requests through the core `engine` or `indexer` interfaces.
   - The core engine exposes pure Rust structs and traits independent of the wire protocol.

3. **Dogfooding Mandate:** All future AI-assisted development on this repository must be governed by its own indexed ADRs.

## Consequences
- The codebase is highly testable without spinning up MCP daemons or external IDEs.
- The MCP protocol layer can be updated or swapped without touching retrieval or storage algorithms.
- Outer scripts (pre-commit hooks, CI/CD gates, and benchmarks) can call the engine directly via CLI or library APIs.
