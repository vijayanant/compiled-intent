# Chronicle MCP Tools (API Reference)

This document defines the strict Model Context Protocol (MCP) tools exposed by the Chronicle Engine to external clients. 

---

## 1. `search` (The Retrieval Lever)

**The Problem:** Different tasks require entirely different retrieval math. Finding a conceptual idea ("How do teams work together?") requires Vector Search, while finding a specific struct name (`SessionLedger`) requires exact Full-Text Search (FTS). Furthermore, outer layers need to filter on their own custom metadata (e.g., "Only search drafts from 2023").

**The Solution:** The `search` tool exposes the search modality and raw SQL-like filters directly to the client, allowing the outer layer to tune the mathematical retrieval dials.

### Arguments (JSON Schema)
*   `query` (string, required): The search term or semantic concept to look for.
*   `namespace` (string, optional): The specific LanceDB collection/table to search within (e.g., `"blog"`, `"adrs"`).
*   `mode` (string, optional): Must be one of `["vector", "fts", "hybrid"]`. Defaults to `"hybrid"`.
*   `metadata_filter` (string, optional): A SQL-like filter string applied to the JSON metadata *before* the vector search executes (e.g., `"metadata.draft = false"`). 
*   `limit` (integer, optional): Maximum results. Defaults to 5.

---

## 2. `get_context` (The Context Expansion Lever)

**The Problem:** To get highly accurate search hits, the engine must chunk text into dense, tiny blocks (e.g., 200 characters). However, if we feed a 200-character chunk to an LLM, the LLM will complain that it lacks surrounding context (e.g., what heading is this under?).

**The Solution:** Implements **Parent-Child Retrieval** (Small-to-Big Retrieval). The Engine searches using tiny chunks, but provides this tool so the outer layer can "zoom out" and retrieve the full human-readable context block before reasoning over it.

### Arguments (JSON Schema)
*   `chunk_id` (string, required): The unique identifier of the micro-chunk returned by the `search` tool.
*   `expand_level` (string, optional): How much surrounding context to fetch. Options:
    *   `"parent"` (default): Returns the full Markdown AST heading block that contains the micro-chunk.
    *   `"siblings"`: Returns the target chunk plus the immediate preceding and succeeding chunks.
    *   `"document"`: Returns the entire source file.

---

## 3. `sync_directory` (The Cache Invalidation Lever)

**The Problem:** The LLM's context window is instantly stale the moment a file changes on disk. If the outer layer (like an IDE) modifies a file, the memory base must be updated instantly without the latency of a full database rebuild.

**The Solution:** Exposes the Engine's differential delta-hashing mechanism. The outer layer can trigger this endpoint to force the Engine to instantly index only the files that have changed.

### Arguments (JSON Schema)
*   `directory_path` (string, required): The path to the content directory.
*   `namespace` (string, required): The LanceDB table/collection to sync this directory into.
*   `chunking_strategy` (string, optional): The engine cannot assume the shape of the data. Must be one of `["ast", "sliding_window"]`. Defaults to `"ast"`.
