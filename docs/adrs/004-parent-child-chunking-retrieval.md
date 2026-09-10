# ADR 004: Parent-Child Chunking (Small-to-Big Retrieval)

## Status
Accepted (Refines ADR 003)

## Context
ADR 002 established that we must chunk by Markdown AST Headers to preserve semantic context and avoid the "blind cuts" of sliding window chunkers. 

However, testing revealed that AST-based chunking suffers heavily from **Vector Dilution**. If a single Markdown heading contains three paragraphs covering three distinct concepts, the resulting vector is mathematically averaged. A specific query for one of those concepts will yield a low similarity score, causing retrieval failures. 

We need the semantic precision of a dense sliding window, but the structural context of an AST header.

## Decision
We will implement **Parent-Child Chunking (Small-to-Big Retrieval)**.

1. **Child Chunks (The Search Vector):** The `indexer` will split document sections into dense, small chunks (e.g., 200 characters). These child chunks will be vectorized. This ensures maximum semantic density and prevents vector averaging.
2. **Parent Context (The Payload):** The `indexer` will store the full AST heading block (the parent) in LanceDB alongside the child chunk.
3. **Retrieval Mapping:** When an MCP search matches a child vector, the engine will NOT return the isolated 200-character chunk to the LLM. Instead, it resolves the child to its `parent_id` and returns the complete, human-readable AST block.

## Consequences
- **Positive:** We eliminate vector dilution without losing structural context. The LLM gets the full paragraph/heading it needs.
- **Negative:** LanceDB storage size will increase slightly due to duplicating the parent text across multiple child rows. 
- **Technical Requirement:** The Arrow schema must be updated to support relational IDs (e.g., `parent_id`, `chunk_id`) or store the full payload alongside the micro-vector.
