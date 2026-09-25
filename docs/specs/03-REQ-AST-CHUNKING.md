# 03-REQ-AST-CHUNKING: Markdown AST-Guided Small-to-Big Chunking

> [!NOTE] **Architectural Rationale**
> For the experimental evidence proving why AST heading isolation outperforms whole-file stuffing (71.4% vs 50.0%), see [ADR-002: Markdown AST Parsing and Small-to-Big Retrieval Strategy](../adrs/002-ast-guided-parent-chunking.md).

## 1. AST Structural Parsing

The engine MUST parse Markdown documents using an Abstract Syntax Tree (AST) event stream via `pulldown_cmark`.

### 1.1 Heading Isolation

- Markdown headings (`#`, `##`, `###`) define authoritative **Parent Block** boundaries.
- When a new heading event (`Event::Start(Tag::Heading)`) is encountered, the current parent block MUST be flushed immediately.
- The heading text MUST be trimmed and prepended to the parent block's content:

  ```markdown
  # <Heading Text>

  <Section Content>
  ```

- If a document contains zero headings, all content MUST be captured under a single default parent block named `"Document Root"`.

### 1.2 Syntax Preservation

- **Code Blocks**: Code block fences (```` ``` ````) and code block content containing `#` characters MUST NOT be misinterpreted as markdown headings.
- **Structural Spacing**: Paragraph ends, list items, and code blocks MUST retain double newlines (`\n\n`) to preserve document formatting.

## 2. Dense Child Sliding Windows

Inside each isolated Parent Block, the engine MUST generate dense child chunks strictly for high-precision embedding math:

| Parameter | Value | Invariant Contract |
| :--- | :---: | :--- |
| **Window Size** | `300` chars | Child text MUST NOT exceed 300 UTF-8 characters. |
| **Overlap Size** | `50` chars | Sequential child windows overlap by 50 characters. |
| **Step Size** | `250` chars | Advance step = `max(1, 300 - 50)`. Zero-step loops are strictly prohibited. |
| **Multibyte Safety** | Enforced | Slicing MUST align with valid UTF-8 character boundaries (`char_indices`). |

## 3. Small-to-Big Context Payload

When a child vector matches a search query:

- The embedding vector matches against the 300-char `vector_text`.
- The engine projects and returns the full, intact `context_payload` (the parent section) with metadata headers:

  ```markdown
  Source: docs/adrs/001-storage.md
  Section: Decision

  # Decision
  We choose Option 3: LanceDB...
  ```
