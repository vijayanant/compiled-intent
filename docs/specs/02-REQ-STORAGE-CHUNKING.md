# 02-REQ-STORAGE-CHUNKING

## 1. Overview

The engine relies on Apache Arrow and LanceDB for high-performance vector retrieval, and `pulldown-cmark` for Markdown parsing.

## 2. Database Schema

Every LanceDB table (namespace) MUST adhere strictly to the following Apache Arrow schema:

```rust
Schema::new(vec![
    Field::new("chunk_id", DataType::Utf8, false),
    Field::new("parent_id", DataType::Utf8, true),
    Field::new("filename", DataType::Utf8, false),
    Field::new("vector_text", DataType::Utf8, false),      // The exact text that was embedded
    Field::new("context_payload", DataType::Utf8, false),  // The expanded parent text
    Field::new("metadata", DataType::Utf8, false),         // JSON serialized YAML frontmatter
    Field::new("vector", DataType::FixedSizeList(
        Arc::new(Field::new("item", DataType::Float32, true)), 
        engine.config.embedding_dimensions
    ), false),
])
```

**Acceptance Criteria:**

- The schema MUST NOT contain nullable strings for `chunk_id`, `filename`, `vector_text`, or `context_payload`.
- A Tantivy FTS index MUST be explicitly generated on the `context_payload` column after every successful indexing operation.

## 3. Markdown Parsing & Chunking

**Description:** The system MUST parse raw markdown files and split them logically.
**Requirements:**

1. **Metadata Extraction:** The system MUST check for YAML frontmatter starting exactly with `---` on line 1. If found, it MUST strip it from the body and parse it into a valid JSON string for the `metadata` column.
2. **AST Traversal:** The `pulldown-cmark` parser MUST group text by Markdown headers (`<h1>` to `<h6>`).
3. **Parent-Child Logic:**
   - A Parent block is the Header + its entire section text.
   - Child blocks are the individual paragraphs within that section.
   - Embeddings MUST be generated solely on the Child block (`vector_text`), but the retrieved `context_payload` MUST contain the Parent block.

**Acceptance Criteria:**

- **Given** a markdown file with 1 Header and 3 Paragraphs.
- **When** chunked.
- **Then** the output MUST yield 3 rows in LanceDB. Each row shares the same `parent_id` and `context_payload`, but has a unique `chunk_id`, `vector_text`, and `vector`.
