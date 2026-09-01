# 06-REQ-BACKGROUND-SYNC

## 1. Overview

Currently, the AI agent (or user) must manually invoke `index_directory` to update the LanceDB vectors. To achieve true "ambient memory", the system MUST support an automated background file watcher.

## 2. File System Watcher

**Description:** The engine MUST optionally run a background Tokio task that listens for filesystem events.
**Requirements:**

- Leverage the `notify` (or `notify-debouncer-mini`) crate to watch registered namespace directories.
- MUST detect `Create`, `Modify`, and `Remove` events for `.md` files.

## 3. Debouncing and Execution

**Description:** Rapid file saves (e.g., typing in an IDE) MUST NOT spam the embedding model or lock the database.
**Requirements:**

- Filesystem events MUST be debounced (e.g., a 2-second sliding window of inactivity before triggering).
- Upon triggering, the system MUST invoke the standard Differential Sync pipeline defined in `03-REQ-SYNC-LIFECYCLE.md` for the affected namespace.

**Acceptance Criteria:**

- **Given** the watcher is active on `/docs/adrs`, **When** a user hits "Save" on `004-new-adr.md` 5 times in 2 seconds, **Then** the system MUST wait until the saves stop, and execute exactly ONE embedding update transaction for that file.
