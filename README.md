# Compiled Intent (Chronicle Engine)

**A Universal, Persistent Semantic Memory Layer for AI Agents**

*Compiled Intent* is a high-performance, system-level context engine written in Rust. It solves the problem of **Contextual Amnesia** and **Context Poisoning** in Cloud LLMs by decoupling persistent memory storage from volatile LLM reasoning.

## The Architecture: System Domain vs. Business Domain

Modern AI workflows often make the mistake of relying on the LLM's ephemeral context window as the system of record. This leads to latency, token bloat, and stale context. 

*Compiled Intent* acts strictly as the **System Domain**. It is a headless proxy that maintains a persistent, perfectly synchronized vector database (LanceDB) of your local files (ADRs, Blog Posts, Documentation). 

All "fuzzy" LLM reasoning (agents, linters, auditing) is offloaded to the **Business Domain** (outer layers like Cursor, Claude, or Python scripts), which query this engine via the **Model Context Protocol (MCP)**.

### Core Philosophy
1. **The LLM is not the system of record.** The local filesystem + LanceDB is the ground truth.
2. **Instant Boot, Zero Overhead:** Written in Rust, the engine starts instantly as a background MCP process.
3. **Domain Agnostic:** The engine does not care if it is indexing a technical blog or a complex software architecture. It strictly handles bytes, vectors, and schema-agnostic JSON metadata.

## Core Capabilities (Roadmap)

To fully empower the outer business layers, the engine's API is designed around extreme flexibility:

*   **Pluggable Chunking Strategies:** Supports everything from AST-level Markdown heading extraction to sliding-window semantic chunking.
*   **Schema-Agnostic Metadata:** Attach arbitrary, deeply nested JSON to any indexed document for surgical pre-filtering.
*   **Small-to-Big Retrieval (Parent-Child):** Index dense, highly-specific chunks to prevent "vector dilution", but retrieve the full parent context (e.g., surrounding paragraphs or full AST headings) at search time.
*   **Hybrid Search Modalities:** Exposes `vector`, `fts` (Full-Text Search), and `hybrid` (RRF) search endpoints.
*   **Differential Syncing:** Fast, delta-based hashing to ensure the vector index remains perfectly in sync with the local filesystem without full rebuilds.

## Usage

*Compiled Intent* operates primarily as an MCP (Model Context Protocol) server communicating over `stdio`. 

```bash
# Start the MCP server for an IDE or Agent
cargo run --release
```

```bash
# Force an offline, differential index of a directory
cargo run --release -- --index /path/to/content
```
