# ADR 003: Markdown-Aware Chunking Strategy

## Status
Accepted

## Context
Initial implementations read entire `.md` files and vectorized them as a single monolithic chunk. This caused severe vector dilution, leading to retrieval failures during the `ArchEval` benchmarks (e.g., retrieving an unrelated ADR instead of the specific rule needed).

To provide accurate semantic retrieval for AI agents, documents must be chunked. However, standard sliding window text chunkers (e.g., fixed token counts) blindly cut documents across structural boundaries, destroying context and mixing unrelated concerns.

## Decision
1. **Header-Based Chunking:** The `indexer` module MUST parse Markdown files based on Header hierarchy (`#`, `##`, `###`) using `pulldown_cmark` rather than arbitrary token counts.
2. **Context Preservation:** Each chunk MUST retain the top-level Title and the immediate section header in its metadata and context payload, ensuring the model understands what section the retrieved text belongs to.
3. **Payload Separation:** Structural metadata (like file names or YAML frontmatter) should be tracked separately in Apache Arrow columns rather than polluting the semantic vector payload.

## Consequences
- AI agents querying LanceDB receive highly targeted, specific sections of an ADR (e.g., just the "Decision" block) rather than an entire document.
- Slashing context payload down to ~491 tokens mitigates attention bleed and prevents out-of-memory errors on local edge hardware.
