# ADR 004: Differential State Manifest and Scoped Orphan Purging

## Status

Accepted

## Context

Addresses [REQ-04: Sync Lifecycle](../specs/04-REQ-SYNC-LIFECYCLE.md).

Embedding generation is computationally expensive: embedding 50 Markdown files via a local LLM or API takes several seconds. Pre-commit hooks run on every local `git commit` and must complete in under 2 seconds.

Furthermore, when an architectural spec or ADR is deleted or renamed, stale vectors in LanceDB will continue to be retrieved and enforced by AI agents unless explicitly invalidated.

## The Dilemma

How should the engine detect modified, added, and deleted files to guarantee near-instantaneous incremental indexing and deterministic cache invalidation?

## Options Evaluated

### Option 1: Full Re-index on Every Invocation

- **Pros**: Trivial logic; zero state synchronization bugs.
- **Cons**: Completely unacceptable latency. Re-embedding an entire documentation tree on every commit wastes CPU/GPU compute, stalls developer commits, and burns API tokens.

### Option 2: Git Diff and Status Tracking (`git diff` / `git status`)

- **Pros**: Leverages existing version control state.
- **Cons**: Fragile and non-portable. Fails completely when documentation lives outside a Git repository, in Git submodules, or in untracked draft files. `git diff` cannot detect unstaged changes reliably when invoked from disparate tooling hooks.

### Option 3: Content-Addressed SHA-256 Manifest with Scoped Orphan Invalidation

- **Pros**: Completely independent of Git. Maintains a persistent local JSON manifest (`.intent/manifest.json`) mapping absolute/relative file paths to SHA-256 content hashes. On each run, hashes are computed in <5ms. Only files with hash deltas are chunked and embedded. Deleted files are detected as orphans and purged via atomic SQL delete filters (`LanceStorage::invalidate_stale_files`).
- **Cons**: Requires maintaining and saving an on-disk manifest file alongside the database.

## Decision

We choose **Option 3: Content-Addressed SHA-256 Manifest**.

The engine operates on a strict 5-stage lifecycle:

1. **Delta Scan**: Compares scanned markdown file hashes against `manifest.json`.
2. **Early Exit**: If zero deltas and zero orphans exist, exits in <10ms ("Cache is warm").
3. **Scoped Orphan Invalidation**: Purges vector rows for deleted files strictly scoped within the scanned directories.
4. **Stale Replacement**: Before writing updated chunks, purges previous vectors for modified files.
5. **Atomic Write & Manifest Commit**: Writes new Arrow record batches, rebuilds the Tantivy FTS index, and commits the updated manifest to disk.

## Rejected Alternatives

- **Full Re-indexing**: Rejected because re-embedding hundreds of files on every commit violates the <2s pre-commit latency contract.
- **Git-Based Tracking**: Rejected because the coprocessor must operate reliably across non-git directories, ephemeral CI runners, and unstaged editor buffers.

## Consequences & Trade-offs

- **Positive**: Sub-10ms warm cache verification; zero stale vector leakage upon file deletion; resilient cross-platform operation.
- **Accepted Trade-off**: Orphan detection must be strictly scoped to the directories currently being scanned. Unscanned directories present in the manifest are preserved to avoid cross-directory data loss.
