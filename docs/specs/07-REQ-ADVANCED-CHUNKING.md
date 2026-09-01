# 07-REQ-ADVANCED-CHUNKING

## 1. Overview

While AST-based chunking (`pulldown-cmark`) is ideal for structured Markdown, the MCP `index_directory` tool schema explicitly permits passing a `chunking_strategy` parameter (`"ast"` or `"sliding_window"`). The engine MUST implement the `sliding_window` fallback.

## 2. Sliding Window Strategy

**Description:** A naive, character/token-based chunking strategy for unstructured text or poorly formatted markdown.
**Requirements:**

- **Input Parameters:** MUST accept `chunk_size` (e.g., 1000 characters) and `chunk_overlap` (e.g., 200 characters).
- **Behavior:** The raw text is split into sequential chunks of `chunk_size`, where the end of Chunk N overlaps with the beginning of Chunk N+1 by `chunk_overlap` characters.

**Acceptance Criteria:**

- **Given** a 2000-character document lacking any markdown headers.
- **When** `index_directory` is called with `chunking_strategy: "sliding_window"`.
- **Then** the chunker MUST split the document sequentially, ensuring the boundaries overlap to preserve semantic context across chunk splits.
