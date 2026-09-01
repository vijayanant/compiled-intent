# Intent Engine: Requirements Index

This directory contains the detailed engineering specifications for the Intent Engine (compiled-intent). These specifications are designed to be testable, verifiable, and implementable by both human developers and autonomous AI agents.

## Specification Documents

1. **[01-REQ-MCP-INTERFACE.md](./01-REQ-MCP-INTERFACE.md)**
   - Covers the Model Context Protocol (MCP) server requirements.
   - Defines exact JSON-RPC schemas for tools (`recall`, `get_context`, `index_directory`) and resources.
   - *Note: Incorporates the mandate from ADR-001 to standardize on the `recall` tool name.*

2. **[02-REQ-STORAGE-CHUNKING.md](./02-REQ-STORAGE-CHUNKING.md)**
   - Covers the LanceDB and Apache Arrow schema definitions.
   - Defines the exact AST-based markdown chunking rules and YAML frontmatter extraction logic.

3. **[03-REQ-SYNC-LIFECYCLE.md](./03-REQ-SYNC-LIFECYCLE.md)**
   - Covers the differential sync engine requirements (`.manifest.json`).
   - Details the exact hashing logic and the required procedure for handling deleted/orphaned files to prevent "phantom memory".

4. **[04-REQ-NAMESPACES-FILTERING.md](./04-REQ-NAMESPACES-FILTERING.md)**
   - Covers the transition from a monolithic hardcoded database to isolated, multi-tenant LanceDB tables (Namespaces).
   - Details the SQL pushdown logic for metadata filtering.

5. **[05-REQ-SYSTEM-CONFIG.md](./05-REQ-SYSTEM-CONFIG.md)**
   - Covers existing configuration parsing (`EngineConfig`), telemetry (`tracing`), and the `clap`-based CLI wrapper for manual testing.

6. **[06-REQ-BACKGROUND-SYNC.md](./06-REQ-BACKGROUND-SYNC.md)**
   - Covers the future implementation of the `notify` file system watcher for ambient, debounce-protected memory updates.

7. **[07-REQ-ADVANCED-CHUNKING.md](./07-REQ-ADVANCED-CHUNKING.md)**
   - Fulfills the `chunking_strategy` enum parameter by detailing the requirements for the fallback Sliding Window chunking algorithm.
