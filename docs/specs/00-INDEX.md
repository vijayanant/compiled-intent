# Compiled Intent: System Specification Index

## 1. Overview

Compiled Intent is a high-performance, system-level context engine written in Rust. It serves as a universal semantic memory layer and architectural coprocessor for AI coding assistants and CI/CD merge gates.

This directory contains the **living functional specifications** defining the public contracts, interfaces, data formats, and behavioral invariants of the engine.

## 2. Specification Map & Architecture Traceability

Each specification defines the operational contract (**WHAT**) and links to its corresponding Architectural Decision Record (**WHY**):

| Specification | Scope & Contract | Authoritative ADR Rationale |
| :--- | :--- | :--- |
| [01-REQ-MCP-INTERFACE](./01-REQ-MCP-INTERFACE.md) | JSON-RPC 2.0 wire protocol, Stdio I/O isolation, tool definitions (`search`, `get_context`, `index_directory`), and error code mapping. | [ADR-005: Headless Stdio MCP Protocol](../adrs/005-headless-stdio-mcp-protocol.md)<br/>[ADR-009: Unified Normalized Relevance Score](../adrs/009-unified-normalized-relevance-score.md) |
| [02-REQ-STORAGE-AND-RETRIEVAL](./02-REQ-STORAGE-AND-RETRIEVAL.md) | 7-column Apache Arrow RecordBatch schema, 384/768-D vector indexing, Tantivy scalar FTS, and Reciprocal Rank Fusion (RRF). | [ADR-001: Embedded Vector Storage](../adrs/001-embedded-vector-storage.md)<br/>[ADR-003: Tri-Modal Search & RRF](../adrs/003-tri-modal-search-and-rrf.md)<br/>[ADR-009: Unified Normalized Relevance Score](../adrs/009-unified-normalized-relevance-score.md) |
| [03-REQ-AST-CHUNKING](./03-REQ-AST-CHUNKING.md) | Markdown AST heading parsing via `pulldown_cmark`, 300-char dense child windows (50 overlap), and parent payload context return. | [ADR-002: Markdown AST Parsing & Small-to-Big Retrieval](../adrs/002-ast-guided-parent-chunking.md) |
| [04-REQ-SYNC-LIFECYCLE](./04-REQ-SYNC-LIFECYCLE.md) | Content-addressed SHA-256 manifest, 5-stage sync lifecycle, and scoped orphan vector invalidation. | [ADR-004: Differential State Manifest & Scoped Invalidation](../adrs/004-differential-manifest-sync.md) |
| [05-REQ-CLI-AND-CONFIG](./05-REQ-CLI-AND-CONFIG.md) | Canonical `intent` binary CLI subcommands (`serve`, `index`, `search`, `ask`), `--config` precedence, and TOML schema. | [ADR-006: Unified Binary Architecture](../adrs/006-unified-binary-architecture.md)<br/>[ADR-008: Decoupled Local Embedding Provider Architecture](../adrs/008-decoupled-local-embedding-provider.md)<br/>[ADR-010: Document Q&A Subcommand](../adrs/010-doc-qa-subcommand.md) |
| [06-REQ-NAMESPACES](./06-REQ-NAMESPACES.md) | Directory-based namespace mapping, unified Arrow column tagging, and DataFusion SQL pushdown filtering. | [ADR-007: Directory-Based Namespaces & Attention Isolation](../adrs/007-directory-namespaces-inter-document-firewall.md) |
