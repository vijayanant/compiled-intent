# Chronicle Architecture

## System Overview
Chronicle is an **Episodic Memory Coprocessor** designed for local AI agents. It solves the "Memory Wall" and "Contextual Amnesia" problems by providing a zero-copy vector database that agents can query on-demand, rather than relying on static prompt stuffing.

Because Chronicle is strictly a **System Domain** component (a persistent memory layer), its API and architecture must be completely domain-agnostic. It does not assume the outer layer is a blogging assistant or an architectural auditor. Instead, it provides flexible, low-level levers so the outer layers can construct their own business logic.

Chronicle operates in two primary modes:
1.  **Offline Indexer:** Scans directories, parses documents into semantic chunks, generates vector embeddings, and stores them in a local LanceDB instance.
2.  **Online Coprocessor (MCP):** Runs as a persistent `stdio` server using the Model Context Protocol (MCP). It exposes tools to any compatible AI IDE or Agent, allowing them to dynamically search the memory base.

## API Design Philosophy

The system API is built around six core capabilities designed to solve the inherent flaws of volatile LLM context windows:

### 1. Pluggable Chunking Strategies
An ADR might need AST-level Markdown chunking, while a raw log file might need a sliding window. The API shouldn't force a single strategy.
* **API Need:** When the outer layer sends a document to be indexed, it should be able to specify the chunking strategy (`strategy="ast"`, `strategy="sliding_window_200"`, etc.).

### 2. Schema-Agnostic Metadata Handling
The Engine shouldn't know what "tags," "series," or "draft" means. It should just know about "text," "vectors," and a generic JSON "metadata" blob.
* **API Need:** The outer layer passes the raw markdown and a parsed JSON metadata payload. The Engine stores the JSON blindly.
* **API Need:** The search API must allow the outer layer to pass pre-filters (e.g., `filter="metadata.draft == false AND metadata.year == 2024"`). LanceDB supports this natively; the API just needs to pass it through.

### 3. Contextual Expansion (Parent-Child Retrieval)
If the outer layer uses tiny 200-character chunks to get highly accurate search hits, the LLM will complain that the chunks lack surrounding context.
* **API Need:** The API must expose a way to request surrounding context. For example, `get_chunk(id="chunk_123", expand_context=True)` which returns the target chunk plus the chunk before and after it, or the entire parent Markdown heading.

### 4. Search Modalities and Dial Tuning
Different tasks require different retrieval math. Finding a conceptual idea ("How do teams work together?") requires Vector Search. Finding a specific function name (`SessionLedger`) requires Full-Text Search (FTS).
* **API Need:** The search endpoint must accept parameters for `mode="vector" | "fts" | "hybrid"`.
* **API Need:** The outer layer should be able to tune the dials (e.g., `limit=5`, or adjusting the Reciprocal Rank Fusion weights for hybrid search).

### 5. Namespaces / Table Isolation
A user might want to index their Blog, their ADRs, and their personal journal on the same machine.
* **API Need:** The API must accept a namespace or collection parameter (e.g., `index_document(file, namespace="blog")`) so that the LanceDB tables remain isolated.

### 6. The Cache Invalidation Endpoint
The outer layer needs to know if the memory is stale, and it needs a way to force an update.
* **API Need:** A `sync(directory, namespace)` endpoint that performs the differential hashing, or an `update_file(file_path)` endpoint so an IDE watcher can instantly push changes to the Engine the moment a file is saved.

## Core Modules

To maintain a clean separation of concerns, the Rust codebase is divided into four distinct modules:

### 1. `config` (Configuration & Orchestration)
Handles loading settings from `chronicle.toml` (or environment variables) and parsing CLI arguments. 
*   **Responsibilities:** Defines indexing paths, chunking parameters, and embedding model endpoints (e.g., local `llama.cpp` server URLs).

### 2. `indexer` (Parsing & Vectorization)
The ingestion engine of Chronicle.
*   **Responsibilities:** 
    *   Reads files from disk.
    *   Applies language-specific chunking strategies (e.g., Markdown header-based chunking).
    *   Separates structural metadata (titles, headers) from semantic payloads.
    *   Calls the embedding model API to generate vectors.
    *   Constructs Apache Arrow `RecordBatch` structures and persists them to LanceDB.

### 3. `engine` (The Core Logic & Retrieval)
The agnostic brain of the system that bridges storage and search.
*   **Responsibilities:** 
    *   Executes vector similarity searches against the LanceDB `.lancedb` directory.
    *   Formats the retrieved chunks into human/LLM readable context blocks.
    *   *Constraint:* Must not contain any MCP-specific serialization logic.

### 4. `mcp` (The Transport Layer)
The communication boundary between Chronicle and the outside world.
*   **Responsibilities:**
    *   Speaks the JSON-RPC Model Context Protocol over `stdin`/`stdout`.
    *   Registers the `recall` tool with connecting clients.
    *   Parses incoming tool calls, passes the query string to the `engine`, and formats the response back into MCP JSON.
    *   *Constraint:* Must redirect all internal logging to `stderr` to avoid corrupting the `stdout` JSON-RPC pipe.

## Data Flow

### Indexing Flow
`Markdown Document` → `Indexer (Header Chunker)` → `Llama.cpp Embedding API` → `Apache Arrow RecordBatch` → `LanceDB (.lancedb)`

### Retrieval Flow (MCP)
`Agent Tool Call (recall)` → `MCP Server` → `Engine (Get Query Embedding)` → `LanceDB (Vector Search)` → `Engine (Format String)` → `MCP Server (Tool Response JSON)`
