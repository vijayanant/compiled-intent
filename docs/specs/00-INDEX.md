# Compiled Intent: System Specification Index

## 1. Overview

Compiled Intent is a high-performance, system-level context engine written in Rust. It serves as a universal semantic memory layer and architectural coprocessor for AI coding assistants and CI/CD merge gates.

This directory contains the **living functional specifications** defining the public contracts, interfaces, data formats, and behavioral invariants of the engine.

## 2. Specification Map & Architecture Traceability

Each specification defines the operational contract (**WHAT**) and links to its corresponding Architectural Decision Record (**WHY**):

| Specification | Scope & Contract | Authoritative ADR Rationale |
| :--- | :--- | :--- |
| [01-REQ-MCP-INTERFACE](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/specs/01-REQ-MCP-INTERFACE.md) | JSON-RPC 2.0 wire protocol, Stdio I/O isolation, tool definitions (`search`, `get_context`, `index_directory`), and error code mapping. | [ADR-005: Headless Stdio MCP Protocol](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/005-headless-stdio-mcp-protocol.md) |
| [02-REQ-STORAGE-AND-RETRIEVAL](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/specs/02-REQ-STORAGE-AND-RETRIEVAL.md) | 7-column Apache Arrow RecordBatch schema, 384/768-D vector indexing, Tantivy scalar FTS, and Reciprocal Rank Fusion (RRF). | [ADR-001: Embedded Vector Storage](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/001-embedded-vector-storage.md)<br/>[ADR-003: Tri-Modal Search & RRF](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/003-tri-modal-search-and-rrf.md) |
| [03-REQ-AST-CHUNKING](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/specs/03-REQ-AST-CHUNKING.md) | Markdown AST heading parsing via `pulldown_cmark`, 300-char dense child windows (50 overlap), and parent payload context return. | [ADR-002: Markdown AST Parsing & Small-to-Big Retrieval](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/002-ast-guided-parent-chunking.md) |
| [04-REQ-SYNC-LIFECYCLE](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/specs/04-REQ-SYNC-LIFECYCLE.md) | Content-addressed SHA-256 manifest, 5-stage sync lifecycle, and scoped orphan vector invalidation. | [ADR-004: Differential State Manifest & Scoped Invalidation](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/004-differential-manifest-sync.md) |
| [05-REQ-CLI-AND-CONFIG](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/specs/05-REQ-CLI-AND-CONFIG.md) | Canonical `intent` binary CLI subcommands (`serve`, `index`, `search`), `--config` precedence, and TOML schema. | [ADR-006: Unified Binary Architecture](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/006-unified-binary-architecture.md) |
| [06-REQ-NAMESPACES](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/specs/06-REQ-NAMESPACES.md) | Directory-based namespace mapping, unified Arrow column tagging, and DataFusion SQL pushdown filtering. | [ADR-007: Directory-Based Namespaces & Attention Isolation](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/007-directory-namespaces-inter-document-firewall.md) |
