# ADR 003: Tri-Modal Retrieval and Reciprocal Rank Fusion (RRF)

## Status

Accepted

## Context

Addresses [REQ-02: Storage and Retrieval](../specs/02-REQ-STORAGE-AND-RETRIEVAL.md).

In an architectural coprocessor, retrieval demands span two conflicting query archetypes:

1. **Exact Identifier Lookups**: When an agent or pre-commit hook audits specific code symbols (e.g., `SessionLedger`, custom error types, API trait names), pure dense vector search fails catastrophically due to semantic embedding averaging and sub-word tokenization.
2. **Conceptual & Behavioral Rules**: When querying high-level architectural mandates (e.g., *"immutable state transition"*, *"database layer isolation"*), exact keyword matching fails when the code diff uses different terminology.

## The Dilemma

How should the retrieval engine combine dense vector semantics with exact lexical matching to deliver high-precision results across both query archetypes?

## Options Evaluated

### Option 1: Pure Dense Vector Search

- **Pros**: Handles synonyms, conceptual queries, and broad architectural inquiries naturally.
- **Cons**: Completely washes out exact code symbols. A query for `SessionLedger` embeds into a general "cryptography/accounting" neighborhood and frequently misses the exact struct declaration.

### Option 2: Pure Lexical Search (Tantivy BM25)

- **Pros**: Guaranteed exact matching for proprietary symbol names and keywords.
- **Cons**: Completely blind to conceptual meaning, synonyms, or abstracted architectural rules.

### Option 3: Linear Convex Score Combination ($\alpha \cdot S_{\text{vector}} + (1-\alpha) \cdot S_{\text{bm25}}$)

- **Pros**: Simple mathematical formulation.
- **Cons**: Cosine distance (bounded $[0, 2]$) and BM25 scores (unbounded $[0, \infty)$) have completely different, non-standardized numerical distributions. Normalizing these scores requires fine-tuning arbitrary scaling weights ($\alpha$) that degrade across different document collections.

### Option 4: Tri-Modal Retrieval with Reciprocal Rank Fusion (RRF)

- **Pros**: Combines dense vector search and Tantivy scalar FTS using reciprocal rank positions ($RRF(d) = \sum \frac{1}{k + r_i}$) rather than raw scores. RRF is scale-invariant and requires zero corpus-specific weight tuning. The engine exposes three explicit execution modalities: `vector`, `fts`, and `hybrid` (default).
- **Cons**: Requires executing two parallel searches in hybrid mode before performing the rank fusion pass.

## Decision

We choose **Option 4: Tri-Modal Retrieval with Reciprocal Rank Fusion (RRF)**.

LanceDB natively executes RRF combining Tantivy BM25 and dense vector search. We configure `hybrid` as the default search mode, while providing explicit `--mode fts` for deterministic symbol lookup and `--mode vector` for pure conceptual queries.

## Rejected Alternatives

- **Pure Vector Search**: Rejected because exact code symbol lookup is a mandatory capability for pre-commit code audits.
- **Linear Convex Scoring**: Rejected because score normalization across disparate mathematical distributions is fragile and unmaintainable across diverse document corpuses.

## Consequences & Trade-offs

- **Positive**: Exact code symbols rank #1 via Tantivy FTS while conceptual queries resolve via vector cosine similarity; zero parameter tuning needed.
- **Accepted Trade-off**: Hybrid search incurs the computational latency of both vector distance calculation and Tantivy inverted index traversal (~40ms total, dominated by external embedding generation).
