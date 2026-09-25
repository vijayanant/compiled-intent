# ADR 009: Unified Normalized Relevance Score Architecture

## Status

Accepted

## Context

Addresses [REQ-01: MCP Interface](../specs/01-REQ-MCP-INTERFACE.md) and [REQ-02: Storage and Retrieval](../specs/02-REQ-STORAGE-AND-RETRIEVAL.md).

Compiled Intent exposes three search execution modalities:
1. `vector`: Dense embedding nearest-neighbor search.
2. `fts`: Tantivy BM25 scalar inverted index search.
3. `hybrid`: Multi-pass vector + FTS retrieval fused via Reciprocal Rank Fusion (RRF).

Each retrieval mechanism computes relevance using mathematically disparate metrics:
- Vector search computes **Cosine Distance** ($d \in [0, 2]$), where **lower is better** ($0.0$ is identical).
- Full-text search computes **BM25 Relevance**, an unbounded positive float (e.g., $0.0$ to $25.0+$), where **higher is better**.
- Hybrid search computes **Reciprocal Rank Fusion (RRF)**, producing tiny fraction sums (e.g., $0.016$ to $0.032$), where **higher is better**.

## The Dilemma

How should search results report match quality across these three disparate modalities to client AI agents, IDE plugins, and pre-commit gates?

Should the engine return raw metrics with mode-specific field names (e.g., `"distance"` for vector, `"bm25"` for FTS), pass through raw numbers under a shared label, or normalize all outputs into a single, unified relevance metric?

## Options Evaluated

### Option 1: Mode-Specific Raw Metrics (`distance`, `bm25`, `rrf_score`)

- **Pros**: Preserves raw mathematical outputs without post-processing or scaling transformations.
- **Cons**: Polymorphic JSON schema forces client AI agents to branch their parsing logic based on the requested `mode`. Agents cannot compare results across different query runs or set cross-modality confidence thresholds.

### Option 2: Shared Field Name with Raw Metric Passthrough

- **Pros**: Uniform JSON key (e.g., `"metric"` or `"distance"`).
- **Cons**: Creates severe semantic contradictions and polarity inversion:
  - In `vector` mode, `0.0` is the best possible match (closest distance).
  - In `fts` mode, `0.0` is the worst possible match (zero keyword overlap).
  - Calling a BM25 score of `11.4` a `"distance"` is mathematically false.
  - In `hybrid` mode, mixing raw distance columns from sub-passes causes results ordered by RRF rank to appear out of order with respect to the printed metric.

### Option 3: Unified Normalized Relevance Score (`score: f32` $\in [0.0, 1.0]$) with Positive Polarity

- **Pros**:
  - **Single Invariant Schema**: Every search result returns `{ "chunk_id", "context", "score" }`.
  - **Universal Polarity**: **Higher is always better**. $1.0$ represents maximum relevance; $0.0$ represents zero relevance.
  - **Agent Decision Thresholds**: Client agents can enforce simple, universal confidence policies (e.g. `if result.score < 0.5 { skip }`) regardless of whether the search was vector, FTS, or hybrid.
  - **Monotonic Output**: Results are consistently ordered from highest score to lowest score.
- **Cons**: Requires mapping unbounded BM25 scores and fraction-scale RRF scores into $[0.0, 1.0]$ using continuous normalization functions.

## Decision

We choose **Option 3: Unified Normalized Relevance Score (`score: f32` $\in [0.0, 1.0]$)**.

We define the normalization formulas as follows:

1. **Vector Mode (Cosine Distance $d \in [0, 2]$)**:
   $$\text{score} = \max(0.0, 1.0 - d)$$
   - Distance $0.0 \to \text{score } 1.0$
   - Distance $1.0 \to \text{score } 0.0$

2. **FTS Mode (BM25 Relevance $S \ge 0$)**:
   $$\text{score} = \frac{S}{S + 10.0}$$
   - Continuously maps unbounded BM25 scores into $[0.0, 1.0)$ with smooth saturation ($S=10 \to 0.50$, $S=25 \to 0.71$).

3. **Hybrid Mode (Reciprocal Rank Fusion $RRF \ge 0$)**:
   $$\text{score} = \min\left(1.0, \frac{RRF}{RRF_{\max}}\right) \quad \text{where } RRF_{\max} = \frac{2}{61} \approx 0.032787$$
   - #1 rank in both passes $\to \text{score } 1.0$
   - #1 rank in single pass $\to \text{score } \approx 0.50$

## Consequences & Trade-offs

- **Positive**: Consistent, unambiguous interface contract across MCP, CLI, and internal engines. LLMs and developers immediately understand that higher scores mean stronger matches.
- **Accepted Trade-off**: Sigmoid saturation for BM25 compresses differences between extremely large scores (e.g., 30 vs 40). We accept this compression because relative ordering is already preserved by Tantivy's internal rank sort.
- **Compliance Invariant**: The engine MUST NOT emit the key `"distance"` in search result objects. All modalities MUST output `"score"` bounded within $[0.0, 1.0]$.
