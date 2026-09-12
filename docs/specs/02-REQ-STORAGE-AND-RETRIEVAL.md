# 02-REQ-STORAGE-AND-RETRIEVAL: Storage Schema and Search Modalities

> [!NOTE] **Architectural Rationale**
>
> - For storage engine trade-offs (LanceDB vs SQLite-VSS vs Qdrant), see [ADR-001: Embedded Vector Storage](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/001-embedded-vector-storage.md).
> - For hybrid search fusion rationale (RRF vs Convex Combination), see [ADR-003: Tri-Modal Retrieval and Reciprocal Rank Fusion](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/003-tri-modal-search-and-rrf.md).

## 1. Apache Arrow Columnar Storage Schema

All indexed documents MUST be serialized into an Apache Arrow `RecordBatch` conforming strictly to the following 7-column schema:

| Column Name | Arrow Data Type | Nullable | Description |
| :--- | :--- | :---: | :--- |
| `chunk_id` | `Utf8` | No | Unique composite identifier (`<filename>_b<parent>_c<child>`). |
| `parent_id` | `Utf8` | Yes | Parent block identifier (`<filename>_b<parent>`). |
| `filename` | `Utf8` | No | Normalized relative path of the source document. |
| `vector_text` | `Utf8` | No | Dense child text ($\le 300$ chars) used for embedding generation. |
| `context_payload` | `Utf8` | No | Full parent Markdown AST section returned to the LLM. |
| `metadata` | `Utf8` | No | Serialized JSON string of frontmatter attributes (or `"{}"`). |
| `vector` | `FixedSizeList<Float32>` | No | Dense vector embedding matching configured dimensions (384/768). |

## 2. In-Process Storage Engine

- **Database**: Embedded LanceDB connection initialized via `lancedb::connect(&path)`.
- **Table Name**: Configurable via `EngineConfig.table_name` (default: `"adrs"`).
- **Full-Text Index**: The engine MUST build an inverted scalar index over the `context_payload` column using LanceDB's built-in Tantivy integration (`LanceStorage::build_fts_index`).

## 3. Search Modalities

The retrieval engine MUST support three first-class search modes:

### 3.1 `fts` (Full-Text Search)

- Executes an inverted index BM25 search over `context_payload`.
- Used for exact identifier matching (struct names, error codes, CLI flags) without vector semantic dilution.

### 3.2 `vector` (Dense Nearest-Neighbor)

- Computes cosine distance between the query embedding and the `vector` column.
- Used for high-level conceptual, architectural, and synonym queries.

### 3.3 `hybrid` (Reciprocal Rank Fusion - Default)

- Executes both dense vector search and Tantivy FTS concurrently.
- Fuses rank orders using Reciprocal Rank Fusion ($RRF(d) = \sum \frac{1}{k + r_i}$) with rank constant $k=60$.
- Guarantees exact symbol hits rank at the top while semantically related rules are captured seamlessly.

## 4. Return Structure

All search modes MUST project and return JSON objects with:

```json
{
  "chunk_id": "docs/specs/01-mcp.md_b0_c0",
  "context": "Source: docs/specs/01-mcp.md\nSection: Overview\n\n# Overview\n...",
  "distance": 0.0
}
```

Retrieved child matches resolve to their full parent AST `context_payload`, preventing isolated, ungrounded token fragments from reaching the evaluator.
