# ADR 005: Tri-Modal Hybrid Search Architecture

## Status
Accepted

## Context
In an architectural enforcement coprocessor, retrieval demands span two distinct querying requirements:
1. **Exact Identifier Lookups:** When an agent or pre-commit hook audits specific code symbols (e.g., `SessionLedger`, custom error types, API trait names), pure dense vector search fails catastrophically due to semantic embedding averaging and sub-word tokenization artifacts.
2. **Conceptual and Pattern Lookups:** When querying high-level architectural rules (e.g., "immutable state transition", "database layer isolation"), exact string matching fails when different terminology is used.
3. **Diff Interception:** At pre-commit and CI/CD merge gates, intercepted changes contain both concrete symbol references and conceptual design changes.

## Decision
We implement a **Tri-Modal Search Architecture** within the Compitent Engine:

1. **Three First-Class Modalities:**
   - `vector`: Cosine distance nearest-neighbor search over 384-dimensional dense child chunk embeddings stored in LanceDB.
   - `fts`: Full-text scalar indexing and BM25 ranking via Tantivy over `context_payload` and `vector_text`.
   - `hybrid`: LanceDB Reciprocal Rank Fusion (RRF) combining Tantivy scalar full-text search and dense vector search into an integrated ranking score.

2. **Configurable Execution:**
   - The engine defaults to `hybrid` search for optimal general retrieval.
   - Outer middleware, pre-commit hooks, and CLI invocations (`intent search --mode <vector|fts|hybrid>`) can explicitly dictate the retrieval mode depending on the audit context.

3. **Contextual Payload Return:**
   - Regardless of search modality, retrieved child matches resolve to their full parent AST `context_payload`, preventing isolated token fragments from reaching the evaluator.

## Consequences
- **Positive:** Exact code identifiers (`SessionLedger`) are resolved with 100% accuracy via FTS or hybrid search without vector dilution.
- **Positive:** Abstract architectural intent remains retrievable via semantic vector similarity.
- **Requirement:** LanceDB tables must maintain both dense vector indices and scalar Tantivy full-text indices over `context_payload`.
