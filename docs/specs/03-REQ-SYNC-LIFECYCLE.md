# 03-REQ-SYNC-LIFECYCLE

## 1. Overview

Indexing large repositories is computationally expensive (due to embedding generation). The engine MUST implement a strict Differential Sync to ensure only modified files are processed, and deleted files are purged.

## 2. The Manifest File

**Description:** The state of the file system MUST be tracked via a JSON manifest file.
**Location:** `[LANCEDB_PATH]/.manifest_[NAMESPACE].json`
**Format:** A key-value map of `{"/absolute/path/to/file.md": "sha256_hash_string"}`.

## 3. Sync Algorithm

When `index_directory(path, namespace)` is invoked, the engine MUST execute the following pipeline:

### 3.1 File Discovery

- Recursively walk `path` and collect all files with the `.md` extension.
- Compute the SHA-256 hash of the raw string content of each file.

### 3.2 Upsert Logic (Insert/Update)

- Compare the computed hash against the state in `.manifest_[NAMESPACE].json`.
- **IF** the file is missing from the manifest, OR the hash differs:
  - Push to `files_to_process`.
  - Issue a DELETE command to LanceDB: `DELETE FROM [namespace] WHERE filename = 'path'`.
  - Chunk, embed, and insert the new vectors.
  - Update the manifest in memory.

### 3.3 Deletion Handling (Critical)

- **Problem Statement:** If a file is deleted from disk, `WalkDir` will not find it, meaning it bypasses the Upsert logic.
- **Requirement:** The engine MUST identify orphans.
- **Algorithm:**
  1. Iterate over all keys (filenames) currently existing in `.manifest_[NAMESPACE].json`.
  2. If a key from the manifest is NOT found in the newly discovered `md_files` list, it is an orphan.
  3. Issue a DELETE command to LanceDB: `DELETE FROM [namespace] WHERE filename = 'orphan_path'`.
  4. Remove the key from the manifest in memory.

### 3.4 Transaction Commit

- The updated manifest MUST be flushed to disk ONLY after the LanceDB transaction successfully commits.

**Acceptance Criteria:**

- **Given** an already indexed file `A.md`, **When** `A.md` is deleted from the filesystem and `index_directory` is called, **Then** querying LanceDB for `filename = 'A.md'` MUST yield 0 results, and `.manifest.json` MUST NOT contain `A.md`.
