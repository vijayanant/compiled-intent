# ADR 006: Differential Sync and Cache Invalidation

## Status
Accepted

## Context
Traditional prompt stuffing and naive vector databases suffer from the "Immutability Trap":
1. **Phantom Memory:** Once an architectural specification is embedded, modifying or deleting a rule leaves obsolete vectors in the index unless an explicit invalidation occurs. If an architect modifies an ADR mid-sprint or deletes an obsolete constraint, the LLM will continue hallucinating or enforcing stale rules.
2. **Re-indexing Overhead:** Rebuilding vector tables from scratch on every commit or file save is computationally prohibitive and destroys the responsiveness required for sub-two-second pre-commit hooks on local edge hardware.

## Decision
We implement **Differential Sync and Cache Invalidation** via manifest hashing:

1. **SHA-256 Manifest Tracking:**
   - The indexer maintains a `.manifest.json` file in the database root mapping file paths to their SHA-256 content hashes.
   - During indexing passes, only new files or files with mismatched SHA-256 hashes are parsed, chunked, and embedded.
   - Unmodified files are skipped completely, ensuring idempotent zero-delta passes.

2. **Atomic Replacement on Update:**
   - When an existing file's hash changes, the storage layer purges all prior rows for that filename (`DELETE WHERE filename = '...'`) prior to appending updated chunk vectors.

3. **Orphan Purging on Deletion:**
   - Files recorded in the manifest that no longer exist on disk are identified as orphans.
   - The indexer deletes all vector entries associated with orphaned files from LanceDB and evicts their keys from `.manifest.json`.

## Consequences
- **Positive:** Guarantees zero "phantom memory"—deleted or modified rules are immediately evicted from the retrieval index.
- **Positive:** Idempotent re-indexing completes in milliseconds when files are unchanged, making pre-commit gates viable.
- **Requirement:** LanceDB tables must support deletion queries by filename, and `.manifest.json` must be kept in sync with on-disk state.
