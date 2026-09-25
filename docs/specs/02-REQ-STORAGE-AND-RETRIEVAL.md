# 02-REQ-STORAGE-AND-RETRIEVAL: Storage Schema and Search Modalities

> [!NOTE] **Architectural Rationale**
>
> - For storage engine trade-offs (LanceDB vs SQLite-VSS vs Qdrant), see [ADR-001: Embedded Vector Storage](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/001-embedded-vector-storage.md).
> - For hybrid search fusion rationale (RRF vs Convex Combination), see [ADR-003: Tri-Modal Retrieval and Reciprocal Rank Fusion](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/003-tri-modal-search-and-rrf.md).
> - For the unified normalized relevance score schema and formulas, see [ADR-009: Unified Normalized Relevance Score Architecture](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/009-unified-normalized-relevance-score.md).

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

### 3.1 `fts` (Tantivy Scalar BM25)

- Evaluates query tokens against the Tantivy inverted index on `context_payload`.
- Used for exact identifier matching (struct names, error codes, CLI flags) without vector semantic dilution.
- Normalizes raw BM25 score $S$ to $[0.0, 1.0]$ via continuous sigmoid saturation: $\text{score} = \frac{S}{S + 10.0}$.

### 3.2 `vector` (Dense Nearest-Neighbor)

- Computes cosine distance $d \in [0, 2]$ between the query embedding and the `vector` column.
- Used for high-level conceptual, architectural, and synonym queries.
- Normalizes cosine distance to similarity score: $\text{score} = \max(0.0, 1.0 - d)$.

### 3.3 `hybrid` (Reciprocal Rank Fusion - Default)

- Executes both dense vector search and Tantivy FTS concurrently.
- Fuses rank orders using Reciprocal Rank Fusion ($RRF(d) = \sum \frac{1}{k + r_i}$) with rank constant $k=60$.
- Guarantees exact symbol hits rank at the top while semantically related rules are captured seamlessly.
- Normalizes RRF score relative to theoretical maximum ($RRF_{\max} = \frac{2}{61} \approx 0.032787$): $\text{score} = \min(1.0, \frac{RRF}{RRF_{\max}})$.

## 4. Return Structure

All search modes MUST project and return JSON objects with a unified, normalized `score` key in range `[0.0, 1.0]`:

```json
{
  "chunk_id": "docs/specs/01-mcp.md_b0_c0",
  "context": "Source: docs/specs/01-mcp.md\nSection: Overview\n\n# Overview\n...",
  "score": 0.95
}
```

- Higher `score` strictly represents greater relevance.
- The legacy `"distance"` key is prohibited.
- Retrieved child matches resolve to their full parent AST `context_payload`, preventing isolated, ungrounded token fragments from reaching the evaluator.

### 4.1 Parent Chunk Collapsing (Sibling Deduplication)

To prevent sibling child chunks from the same parent section from crowding out distinct parent documents in the top-$k$ results:
- **Candidate Over-fetching**: The search pipeline over-fetches candidate rows from LanceDB ($\max(k \times 4, 20)$).
- **Stream Preservation**: Candidates stream natively from LanceDB in authoritative rank order (RRF fused order for hybrid, BM25 for FTS, vector distance for vector), guaranteeing optimal multi-modal fusion.
- **Parent Collapsing Invariant**: Sibling child chunks sharing the same `parent_id` (or derived `<filename>_b<parent>` identity) MUST collapse into a single result entry, retaining the child needle with the highest similarity score.
- **Top-$k$ Truncation**: Results are truncated to the requested `limit` of unique parent context payloads.

This invariant guarantees that every slot in the top-$k$ window provides distinct, non-redundant contextual logic to the agent while eliminating redundant token consumption.

### 4.2 Adaptive Retrieval Windows (Score Tolerance Gating)

To eliminate the "Precipice Problem" and prevent marginal or distractor chunks from diluting the LLM's attention budget during developer workflows:
- **Absolute Floor ($S_{\text{floor}}$)**: If configured via `min_score`, any candidate whose normalized relevance score $S < S_{\text{floor}}$ is pruned. If the top match fails this floor, 0 chunks are returned to prevent false-positive prompt pollution on generic queries.
- **Score Tolerance ($\tau \in [0.0, 1.0]$)**: If configured via `score_tolerance`, candidates whose score falls more than fraction $\tau$ below the top score are pruned (i.e. $S < (1.0 - \tau) \cdot S_{\text{top}}$). For example, `score_tolerance = 0.15` retains matches within 15% of the top-ranked hit.
- **Compound Gating Invariant**: A candidate chunk $i$ is retained if and only if:
  $$S_i \ge \max\left(S_{\text{floor}},\, (1.0 - \tau) \cdot S_{\text{top}}\right)$$
- When neither threshold is specified, the engine returns the standard top-$k$ parent results, preserving fixed-window IR benchmark comparability.



