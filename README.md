# Compiled Intent (`intent`)

**A Local Semantic Memory Layer and Architectural Coprocessor for AI Agents**

[![Engine Version](https://img.shields.io/badge/Version-v0.1.0-orange.svg)](Cargo.toml)
[![Research Paper](https://img.shields.io/badge/Research-Report%20(PDF)-blue.svg)](https://github.com/vijayanant/compiled-intent-research)

*Compiled Intent* is a fast, local semantic context engine written in Rust. It serves as an offline architectural memory gate and Model Context Protocol (MCP) server for coding agents. Instead of stuffing thousands of lines of architectural specifications into LLM prompts, `intent` compiles design rules, RFCs, and Architecture Decision Records (ADRs) into an embedded LanceDB store and serves AST-bounded constraint chunks on demand.

Read the empirical research report and evaluation findings in [compiled-intent-research](https://github.com/vijayanant/compiled-intent-research).

---

## Why Compiled Intent?

When teams attempt to enforce architectural constraints with AI coding assistants, they usually rely on **prompt stuffing**—dumping large design specifications, wiki pages, or ADRs into the agent's context window. 

At scale, this breaks down:
* **The Ammunition Effect:** Large prompts contain non-normative context, testing mocks, and deprecation notes that LLMs seize upon to rationalize bad code changes.
* **Token Bloat & Latency:** Holding 15,000–20,000 tokens of documentation in active KV cache degrades local machine responsiveness and inflates cloud API costs.
* **Contextual Amnesia:** Coding models only see what happens to be in their current window; they have no persistent awareness of project boundaries.

*Compiled Intent* treats architectural rules the way compilers treat code: parsing them into discrete AST invariant blocks, indexing them locally in LanceDB, and surfacing only the governing rules when an agent proposes or reviews a diff.

---

## Installation

Install the canonical `intent` binary directly via Cargo:

```bash
cargo install --git https://github.com/vijayanant/compiled-intent.git
```

Verify the installation:

```bash
intent --help
```

---

## Quickstart

### 1. Prerequisites (Local Embeddings)

`intent` decouples vector embedding generation to an external local model server (such as [Ollama](https://ollama.com/)):

```bash
# Pull the default 768-dimension embedding model
ollama pull nomic-embed-text
```

### 2. Index Documentation

Recursively index any directory containing Markdown files (ADRs, design specs, guidelines):

```bash
intent index docs/
```

Indexing uses content-addressed SHA-256 hashing. Re-running `intent index` only processes modified or newly added files, automatically purging stale vectors for deleted or edited documents.

### 3. Search Intent from the CLI

Run ad-hoc queries across your indexed specifications:

```bash
# Hybrid Search (Vector + BM25 Full-Text Search via RRF) - Default
intent search "How are authentication tokens derived?"

# Exact Keyword Search (Tantivy Full-Text Search)
intent search "SessionLedger" --mode fts

# Pure Vector Search
intent search "Cryptographic integrity guarantees" --mode vector --limit 5
```

Search output is returned as formatted JSON with normalized relevance scores `[0.0, 1.0]`:

```json
[
  {
    "chunk_id": "docs/specs/auth.md_b2_c0",
    "context": "Source: docs/specs/auth.md\nSection: Token Derivation\n\n# Token Derivation\n...",
    "score": 0.8842
  }
]
```

---

## Model Context Protocol (MCP) Integration

`intent` natively implements the Model Context Protocol (JSON-RPC 2.0 over `stdio`). When launched with no arguments or with `intent serve`, it acts as a headless MCP server daemon.

### Client Configuration

Add `intent` to your MCP client configuration (Claude Desktop, Cursor, or Antigravity):

```json
{
  "mcpServers": {
    "compiled-intent": {
      "command": "intent",
      "args": ["serve"]
    }
  }
}
```

If you use a custom configuration file:

```json
{
  "mcpServers": {
    "compiled-intent": {
      "command": "intent",
      "args": ["--config", "/path/to/custom_config.toml", "serve"]
    }
  }
}
```

### Exposed MCP Tools

| Tool | Purpose | Key Arguments |
| :--- | :--- | :--- |
| `search` | Queries the semantic index across vector, FTS, or hybrid modalities. Returns top-$k$ relevant AST chunks. | `query` (required), `mode` (`hybrid`, `vector`, `fts`), `limit` |
| `get_context` | Small-to-big retrieval: resolves a `chunk_id` back to the full source document on disk. | `chunk_id` (required) |
| `index_directory` | Triggers differential indexing on a target directory directly from the agent. | `directory_path` (required) |

---

## Configuration

`intent` reads configuration from `.intent/config.toml` by default, or from the path specified by the global `--config` flag:

```toml
# Embedding endpoint (OpenAI-compatible)
llama_endpoint = "http://localhost:11434/v1/embeddings"
embedding_model = "nomic-embed-text:latest"
embedding_dimensions = 768

# Local storage paths
intent_dir = ".intent"
lancedb_path = ".intent/lancedb"
```

---

## Documentation & Architecture

The engine is engineered around formal system specifications and immutable Architectural Decision Records:

* **[Architectural Decision Records (ADRs)](docs/adrs/)**: Documented design decisions covering embedded vector storage ([ADR-001](docs/adrs/001-embedded-vector-storage.md)), AST parent-child chunking ([ADR-002](docs/adrs/002-ast-guided-parent-chunking.md)), tri-modal hybrid search ([ADR-003](docs/adrs/003-tri-modal-search-and-rrf.md)), differential sync manifests ([ADR-004](docs/adrs/004-differential-manifest-sync.md)), headless Stdio MCP ([ADR-005](docs/adrs/005-headless-stdio-mcp-protocol.md)), and unified relevance scoring ([ADR-009](docs/adrs/009-unified-normalized-relevance-score.md)).
* **[System Specifications](docs/specs/)**: Formal wire protocols, Arrow storage schemas, AST parsing contracts, and CLI interfaces.
* **[Research Report & Benchmarks](https://github.com/vijayanant/compiled-intent-research)**: Empirical evaluation across 117 benchmark tasks in ArchEval, demonstrating a 64% reduction in context payload against single-file oracle retrieval.
