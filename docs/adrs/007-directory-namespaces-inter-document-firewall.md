# ADR 007: Directory-Based Namespaces and Inter-Document Attention Isolation

## Status

Accepted

## Context

Addresses [REQ-06: Namespaces](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/specs/06-REQ-NAMESPACES.md).

Real-world software repositories contain diverse documentation types: normative Architecture Decision Records (ADRs), descriptive API specifications, operational runbooks, and design proposals. These documents frequently share identical technical vocabularies (e.g., cryptographic keys, networking protocols, database models).

In our research paper benchmarks, we discovered that unpartitioned retrieval across mixed document types causes **Cross-Domain Semantic Bleed**: a pre-commit audit evaluating an adversarial cryptographic PR retrieved a descriptive API spec (which validated signature syntax) instead of the normative Governance ADR (which strictly restricted the key derivation branch). Because edge models operate on a strict ~500-token attention budget, descriptive specifications crowd out normative constraints, causing the model to approve illegal changes.

## The Dilemma

How should the engine partition heterogeneous documentation to provide an **Inter-Document Attention Firewall** without forcing developers to adopt proprietary YAML frontmatter schemas?

## Options Evaluated

### Option 1: Mandatory YAML Frontmatter Taxonomy

- **Pros**: Explicit document-level classification (`type: adr`, `type: spec`, `module: crypto`).
- **Cons**: Severe developer friction. Forces developers to manually maintain proprietary metadata headers on every Markdown document. Repositories with standard Markdown documentation fail to index out-of-the-box.

### Option 2: Separate Physical LanceDB Tables per Namespace (`adrs.lance`, `specs.lance`)

- **Pros**: Strict physical isolation at the filesystem level.
- **Cons**: Catastrophic impact on global search. Searching across all project documentation requires opening $N$ separate LanceDB tables, executing $N$ queries, and inventing complex custom re-ranking math to merge disparate RRF rank spaces in Rust. Multiplies manifest files and connection handles.

### Option 3: Unified Table with Configurable Directory Namespaces and Columnar Tagging

- **Pros**: Developers configure directory mappings in `.intent/config.toml` (e.g., `adrs = "docs/adrs"`, `specs = "docs/specs"`). The engine maintains a **single unified LanceDB table**, adding a dedicated `namespace: String` column to the Arrow schema. Scoped searches apply an exact SQL equality predicate (`namespace = 'adrs'`) with zero scan overhead. Global searches query the entire knowledge base in a single fast pass (~2ms) with unified RRF scoring.
- **Cons**: Requires adding a `namespace` field to the Arrow RecordBatch builder.

## Decision

We choose **Option 3: Unified Table with Configurable Directory Namespaces**.

Directory paths configured in `.intent/config.toml` define the namespace. Every chunk generated from that path inherits the namespace tag. Pre-commit firewall gates can deterministically restrict retrieval to `namespace = "adrs"` to guarantee zero spec pollution, while code generation agents can query `namespace = "specs"`.

## Rejected Alternatives

- **Mandatory Frontmatter**: Rejected to maintain zero friction for standard Markdown files.
- **Separate Physical Tables**: Rejected because multi-table fan-out severely degrades global search latency and distorts Reciprocal Rank Fusion scores.

## Consequences & Trade-offs

- **Positive**: Provides a deterministic **Inter-Document Attention Firewall**; global search remains a single fast query; zero proprietary document formatting required.
- **Accepted Trade-off**: Re-mapping an existing directory to a new namespace in `config.toml` requires a re-index pass to update the columnar tag.
