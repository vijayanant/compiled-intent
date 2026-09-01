# ADR 001: Compitent Modular Architecture and Dogfooding

## Status
Accepted

## Context
Compitent currently has a flat, monolithic Rust structure (`main.rs`, `engine.rs`, `mcp_server.rs`). As we introduce configuration files, Markdown-specific chunking, and incremental LanceDB indexing, `engine.rs` will become an unmaintainable God Object.

Furthermore, as a tool designed to enforce Architectural Decision Records (ADRs) via MCP, we must "dogfood" our own product. The engine must be governed by its own ADRs, and the Compitent MCP server must be used during its own development.

## Decision
1. **Directory Structure:** We will refactor the codebase into a modular Library/Binary structure.
   - `src/lib.rs`: The core public API.
   - `src/main.rs`: The thin CLI entrypoint.
   - `src/config/`: Handles `intent.toml` parsing and environment variables.
   - `src/indexer/`: Contains `markdown_chunker.rs` and LanceDB storage logic.
   - `src/mcp/`: Contains the `mcp_server.rs` stdio transport and tool definitions.

2. **Separation of Concerns:** 
   - The `indexer` module MUST NOT know about the Model Context Protocol (MCP).
   - The `mcp` module MUST NOT directly interact with LanceDB; it must route requests through the core `engine` or `indexer` interfaces.

3. **Dogfooding Mandate:** All future AI-assisted development on this repository must run with the Compitent MCP server attached, and agents must query the `recall` tool before refactoring.

## Consequences
- The codebase will be highly testable (we can test the chunker without spinning up the MCP server).
- Adding new chunkers (e.g., Tree-sitter for code) later will simply require adding a new module under `src/indexer/`.
- We can immediately begin indexing this `docs/adrs/` folder to govern our own agentic workflow.
