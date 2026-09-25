# ADR 002: Markdown AST Parsing and Small-to-Big Retrieval Strategy

## Status

Accepted

## Context

Addresses [REQ-03: AST Chunking](../specs/03-REQ-AST-CHUNKING.md).

Architectural specifications, ADRs, and engineering documentation possess a rigid hierarchical structure: documents are divided into functional sections defined by Markdown headings (`#`, `##`, `###`), containing code blocks, tables, and paragraphs.

In edge-based LLM workflows, the model operates under a tight attention budget (~500 tokens). Slicing documents into arbitrary token chunks or feeding entire files into the prompt severely degrades model reasoning.

## The Dilemma

How should technical Markdown documents be segmented to maximize semantic search accuracy while providing coherent, actionable context to the LLM?

## Options Evaluated

### Option 1: Fixed-Token Sliding Windows (e.g., 512 tokens with 50-token overlap)

- **Pros**: Trivial to implement; language-agnostic.
- **Cons**: Cuts sentences and code blocks in half at arbitrary byte boundaries. Severely suffers from the "Pronoun Problem" (a child chunk reading *"It must not be called asynchronously"* loses the heading that identifies what *"It"* refers to). Bleeds text across unrelated sections, causing the LLM to hallucinate rules across boundary lines.

### Option 2: Whole-File Document Injection

- **Pros**: Complete document context preserved without chunking artifacts.
- **Cons**: Severe intra-document attention bleed. In benchmark experiments, feeding entire 2,160-token specification files caused model reasoning accuracy on adversarial tasks to drop from 71.4% down to 50.0%. Surrounding paragraphs act as attention distractors, diluting the model's focus away from the specific normative constraint.

### Option 3: AST-Guided Parent-Child Chunking (Small-to-Big Retrieval)

- **Pros**: Uses an AST parser (`pulldown_cmark`) to isolate text strictly by Markdown headings. Each heading defines an authoritative **Parent Section**. Inside each section, small dense child windows ($\le 300$ characters, 50-character overlap) are generated strictly for embedding math. When a child chunk matches during search, the engine expands and returns the entire intact Parent Block.
- **Cons**: Requires building and maintaining an AST event state machine; duplicate text stored across child vector rows.

## Decision

We choose **Option 3: AST-Guided Parent-Child Chunking**.

By using `pulldown_cmark`, heading text is explicitly preserved and prepended to all parent payloads (`# Heading\n\n...`), completely eliminating the Pronoun Problem. Generating dense 300-character child windows maximizes vector similarity sharpness, while returning the full parent section provides the LLM with complete, unbroken contextual logic.

## Rejected Alternatives

- **Fixed-Token Sliding Windows**: Rejected because arbitrary token boundaries sever syntax and destroy heading association.
- **Whole-File Retrieval**: Rejected because empirical research proves that intra-document peripheral sections create catastrophic attention distractors on edge models.

## Consequences & Trade-offs

- **Positive**: High vector precision from dense 300-char children; zero pronoun ambiguity in retrieved parent contexts; strict structural isolation between headings.
- **Positive**: Parent Chunk Collapsing automatically deduplicates sibling child chunks that resolve to the same parent AST block (`parent_id`), eliminating sibling crowding in top-$k$ search windows and maximizing information diversity.
- **Accepted Trade-off**: Documents with zero markdown headings fall back to a single default `"Document Root"` parent block.

