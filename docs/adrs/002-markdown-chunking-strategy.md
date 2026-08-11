# ADR 002: Markdown-Aware Chunking Strategy

## Status
Accepted

## Context
Our initial LanceDB implementation (`engine.rs`) read entire `.md` files and vectorized them as a single chunk. This caused severe vector dilution, leading to retrieval failures during the `ArchEval` benchmarks (e.g., retrieving an unrelated crypto ADR instead of the Event Sourcing ADR).

To provide accurate semantic retrieval for AI Agents, documents must be chunked. However, sliding window text chunkers (e.g., 500 tokens) blindly cut documents in half, destroying context.

## Decision
1. **Header-Based Chunking:** The `indexer` module MUST parse Markdown files based on Header hierarchy (`#`, `##`, `###`) rather than arbitrary token counts.
2. **Context Preservation:** Each chunk MUST retain the top-level Title and the immediate parent header in its metadata or payload, ensuring the embedding model understands what the section belongs to.
3. **Payload Separation:** Structural metadata (like file names or YAML frontmatter) should be tracked separately from the semantic vector payload to prevent noise.

## Consequences
- AI Agents querying LanceDB will receive highly targeted, specific sections of an ADR (e.g., just the "Decision" block) rather than a massive 2,000-word file.
- We will need to pull in a lightweight Markdown parser or write a regex-based header scanner in Rust.
