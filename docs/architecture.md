# Compiled Intent: System Architecture

## 1. System Overview

**Compiled Intent** is an episodic memory coprocessor and architectural enforcement engine written in Rust. It solves the problems of **Contextual Amnesia** and **Context Poisoning** in AI coding workflows by replacing volatile prompt stuffing with deterministic, compiler-grade retrieval.

### The Two Domains

- **System Domain (Compiled Intent)**: A deterministic, headless Rust engine (`intent`) that maintains a synchronized Apache Arrow / LanceDB vector database of local project intent (ADRs, Specs, Architecture Guides). It is completely domain-agnostic.
- **Business Domain (AI Agents & Merge Gates)**: Outer layers (Claude Desktop, Cursor, Antigravity, CI/CD pre-commit hooks) that query the engine via the **Model Context Protocol (MCP)** or CLI, evaluating code diffs against retrieved architectural constraints.

```mermaid
flowchart TD
    subgraph BusinessDomain ["Business Domain (Reasoning & Evaluation)"]
        IDE["AI IDE (Claude / Cursor / Antigravity)"]
        CI["Pre-Commit Interceptor / CI Gate"]
    end

    subgraph SystemDomain ["System Domain (Compiled Intent Engine)"]
        CLI["CLI Router (intent)"]
        MCP["MCP Server (JSON-RPC 2.0 over Stdio)"]
        Engine["Core Retrieval Engine"]
        Indexer["AST Ingestion & Sync Pipeline"]
        Storage["Embedded LanceDB + Tantivy FTS"]
        Manifest["SHA-256 State Manifest"]
    end

    IDE <-->|Stdio JSON-RPC 2.0| MCP
    CI <-->|Subcommand Execution| CLI
    CLI --> Engine
    CLI --> Indexer
    MCP --> Engine
    MCP --> Indexer
    Indexer -->|Parse & Slice| AST["pulldown_cmark AST"]
    Indexer -->|Content Hashing| Manifest
    Indexer -->|Arrow RecordBatch| Storage
    Engine -->|Hybrid RRF Query| Storage
```

---

## 2. Core Architectural Pillars

### 2.1 Intra-Document Attention Firewall (AST Heading Chunking)

- Technical documents have strict semantic hierarchies.
- The engine parses Markdown using `pulldown_cmark`, using headings (`#`, `##`) as authoritative **Parent Block** boundaries.
- Slices dense child chunks ($\le 300$ characters) strictly *within* heading boundaries.
- Matches resolve back to full parent sections (`context_payload`), completely eliminating pronoun ambiguity while preventing intra-document attention bleed.

### 2.2 Inter-Document Attention Firewall (Directory Namespaces)

- Configured in `.intent/config.toml` (e.g. `adrs = "docs/adrs"`, `specs = "docs/specs"`).
- Slices from each directory inherit an exact `namespace` column in the unified LanceDB Arrow table.
- Pre-commit audit gates restrict queries to `namespace = "adrs"` to mathematically prevent descriptive specs from polluting normative architectural audits.

### 2.3 Tri-Modal Hybrid Search & Reciprocal Rank Fusion (RRF)

- Dense vector search fails on exact code symbols (`SessionLedger`, `CompitentError`); lexical search fails on abstract conceptual rules.
- LanceDB executes concurrent vector cosine search and Tantivy BM25 scalar search, fusing rank positions using scale-invariant Reciprocal Rank Fusion ($k=60$).

### 2.4 Differential State Synchronization

- Content-addressed SHA-256 manifest (`.intent/manifest.json`) detects modified, added, and deleted files.
- Exits in <10ms when the cache is warm.
- Scoped orphan purging ensures deleted files are evicted from LanceDB storage without cross-directory data loss.

### 2.5 Stdio Wire Isolation

- When running as an MCP server daemon, `stdout` is reserved 100% for valid JSON-RPC 2.0 frames.
- All telemetry, logging, and diagnostics are strictly emitted to `stderr`.

---

## 3. Module Hierarchy & Separation of Concerns

The Rust codebase is partitioned into distinct submodules:

```
src/
├── lib.rs              <-- Public library API and re-exports
├── main.rs             <-- Unified canonical binary (intent)
├── config/             <-- Configuration loading (config.toml) and CLI argument parsing
├── engine/             <-- Core storage orchestration and hybrid search execution
├── indexer/            <-- Markdown AST chunking, Arrow RecordBatch building, and manifest sync
│   ├── arrow_builder.rs
│   ├── manifest.rs
│   ├── markdown_chunker.rs
│   └── storage.rs
├── mcp/                <-- Headless JSON-RPC 2.0 protocol handling and tool dispatch
│   ├── mod.rs
│   └── tools.rs
└── telemetry.rs        <-- Stderr-isolated tracing subscriber initialization
```

### Separation Invariants

1. **The Indexer MUST NOT know about MCP**.
2. **The MCP layer MUST NOT directly access LanceDB or filesystem tables**; it must route all calls through the core `engine` or `indexer`.
3. **The Core Engine exposes pure Rust data structures** independent of JSON-RPC or CLI representations.

---

## 4. Documentation Architecture

The `docs/` directory follows the industry standard separating **Contracts (WHAT)** from **Decisions (WHY)**:

- [docs/specs/](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/specs): Living functional specifications defining schemas, CLI flags, protocols, and invariants.
- [docs/adrs/](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs): Immutable chronological decision records evaluating competing options, trade-offs, and consequences.
