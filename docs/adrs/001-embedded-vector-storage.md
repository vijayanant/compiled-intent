# ADR 001: Embedded Vector and Scalar Storage Engine

## Status

Accepted

## Context

Addresses [REQ-02: Storage and Retrieval](../specs/02-REQ-STORAGE-AND-RETRIEVAL.md).

Compiled Intent is designed to operate as an autonomous, zero-friction architectural coprocessor in local developer environments and CI/CD pre-commit hooks. It must store high-dimensional embeddings (384-D / 768-D) for semantic similarity search alongside raw document strings for exact code symbol retrieval (e.g., `SessionLedger`, custom error types).

Pre-commit interceptors have a hard latency budget (<3 seconds total execution) and must run headlessly on developer laptops without requiring multi-tenant cloud credentials, background system daemons, or open network ports.

## The Dilemma

How should the engine store, index, and query vector embeddings and scalar full-text metadata locally?

## Options Evaluated

### Option 1: External Vector Database Daemon (Qdrant / Milvus)

- **Pros**: Mature vector clustering, battle-tested HNSW indexing, rich metadata filtering.
- **Cons**: Requires running an external service (Docker container or system daemon). If the daemon is stopped, pre-commit hooks and IDE tools fail immediately. Managing background process lifecycles, network sockets, and port collisions on developer workstations introduces unacceptable operational friction.

### Option 2: Relational Database with Vector Extension (SQLite + `sqlite-vss` / `sqlite-vec` + FTS5)

- **Pros**: Ubiquitous, single-file embedded database with mature ACID guarantees.
- **Cons**: Vector extensions require dynamic C library compilation and platform-dependent shared object linking. SQLite's write-lock contention hinders concurrent ingestion. Most critically, combining vector similarity search with FTS5 BM25 text queries requires custom SQL ranking heuristics that perform poorly across varying corpus sizes.

### Option 3: Embedded Columnar Storage (LanceDB via Lance Format)

- **Pros**: Pure embedded, in-process library running directly inside the host process with zero background daemons. Built on Apache Arrow columnar memory format, enabling zero-copy memory-mapped (`mmap`) reads. Provides native out-of-the-box Tantivy scalar full-text search and built-in Reciprocal Rank Fusion (RRF) hybrid search. Opens in 2–5ms with negligible memory overhead.
- **Cons**: Younger Rust SDK; schema migrations require rewriting record batches.

## Decision

We choose **Option 3: LanceDB**.

LanceDB provides the optimal balance for an edge coprocessor: in-process zero-daemon deployment, sub-5ms table opens, native Apache Arrow columnar serialization, and native Tantivy full-text search integration.

## Rejected Alternatives

- **Qdrant / Milvus**: Rejected because external daemons violate the zero-dependency, self-contained pre-commit hook requirement.
- **SQLite-VSS**: Rejected due to platform-specific dynamic C library dependencies, write-lock contention, and lack of native vector-FTS hybrid fusion.

## Consequences & Trade-offs

- **Positive**: Single zero-dependency binary with zero network port conflicts and instantaneous cold-start.
- **Accepted Trade-off**: Storing full parent markdown payloads directly in Arrow string columns introduces data redundancy across child chunks. We explicitly accept this storage overhead to eliminate dual-database write inconsistency (e.g., storing vectors in LanceDB while maintaining a separate SQLite database for text payloads).
