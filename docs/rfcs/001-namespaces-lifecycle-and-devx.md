# RFC 001: Directory Namespaces, Indexing Lifecycle, and Developer Experience

- **Status**: Proposed / Under Discussion
- **Related ADR**: [ADR-007: Directory-Based Namespaces & Attention Isolation](../adrs/007-directory-namespaces-inter-document-firewall.md)
- **Related Spec**: [06-REQ-NAMESPACES: Directory Namespaces](../specs/06-REQ-NAMESPACES.md)
- **Author(s)**: Core Architecture Working Group
- **Date**: 2026-09-25

---

## 1. Context & Motivation

In multi-document software repositories, different classes of documentation serve distinct operational roles:
1. **Normative Governance (ADRs)**: Authoritative constraints and architectural boundaries (e.g. "Do not use SQLite; only LanceDB is permitted").
2. **Descriptive Specifications (Specs)**: API shapes, payloads, schema structures, and technical protocols.
3. **Operational Context (Runbooks / Ops)**: Deployment steps, disaster recovery, runbooks.
4. **General Project Documentation (Global)**: High-level architectural overviews (`architecture.md`), contributor guidelines (`CONTRIBUTING.md`), and top-level READMEs.

When an AI coding agent or CI merge gate operates on an edge model context budget (~4k to 8k tokens), unpartitioned search across these heterogeneous document types causes **Cross-Domain Semantic Bleed**: descriptive syntax specs crowd out normative governance rules, leading to illegal architectural drift.

To solve this, [ADR-007](../adrs/007-directory-namespaces-inter-document-firewall.md) introduced the concept of the **Inter-Document Attention Firewall**, partitioning documentation by directory and tagging chunks with a `namespace: Utf8` column in a single unified LanceDB table.

This RFC captures the subsequent architectural discussions, edge-case evaluations, and developer experience (DevX) trade-offs required to take the namespace model from an abstract concept to a production-grade, frictionless implementation.

---

## 2. Key Architecture Discussions & Trade-off Analysis

### 2.1 The Dilemma of Unmapped / "Global" Documentation

#### Discussion
When a repository configures specific namespace directories:
```toml
[namespaces]
adrs  = "docs/adrs"
specs = "docs/specs"
```
What should happen to top-level or general documentation (e.g. `README.md`, `docs/architecture.md`)?

#### Approach A: Recursive Root Scanning with a Fallback Namespace (`'global'`)
The engine scans recursively from the repository root. Any Markdown file under a configured namespace folder inherits that namespace tag; any other Markdown file receives `namespace = 'global'`.

- **Pros**:
  - High-level files (`docs/architecture.md`, `README.md`) are never lost.
  - Zero-configuration indexing: even if `[namespaces]` is empty, every document is indexed under `'global'`.
- **Cons & Traps (Why this was rejected)**:
  - **The Blacklist Trap**: Scanning from the root requires maintaining an exclusion blacklist (`.git`, `target`, `node_modules`, `.venv`, `dist`, `build`, submodules, benchmark logs). Blacklists are inherently brittle, resulting in accidental indexing of vendored code, generated reports, or thousands of third-party markdown files.

#### Approach B: Explicit Whitelisting (The Consensus)
The engine scans **only** the directories explicitly declared in `config.toml`. 
- Zero guesswork.
- Zero blacklisting or filesystem crawler leaks.
- If a project wants general documentation indexed, it explicitly adds a namespace for it:
  ```toml
  [namespaces]
  adrs   = "docs/adrs"
  specs  = "docs/specs"
  core   = "docs" # or global = "docs"
  ```

---

### 2.2 CLI Arguments vs. Declarative Configuration

#### Discussion
Early CLI prototypes supported passing arbitrary paths on the command line:
```bash
intent index docs/adrs
intent index some/other/folder
```

#### The Problem: State Drift & Ghost Directories
Allowing arbitrary ad-hoc CLI paths introduces severe lifecycle bugs:
1. **Re-indexing Failure**: If a developer indexes `some/other/folder` via the CLI, that directory was never added to `config.toml`. Next week, when the developer or CI runs `intent index`, the engine only knows about `config.toml` and will never scan, update, or purge deleted files from `some/other/folder`. It becomes a stale "ghost" in the database.
2. **Ambiguous Namespace Tagging**: If `some/other/folder` is not defined in `config.toml`, what namespace should its chunks receive? Guessing or assigning a generic tag undermines the deterministic Attention Firewall.

#### The Consensus
`config.toml` must be the **single source of truth** for tracked documentation directories:
- `intent index`: Differential sync for **all** namespaces defined in `config.toml`.
- `intent index --namespace <name>`: Validates that `<name>` exists in `config.toml`. If found, syncs only that specific namespace. If not found, errors cleanly with a list of available namespaces.
- **Positional path arguments (`intent index <path>`) are eliminated entirely.**

---

### 2.3 Accidental Namespace Overlap & Directory Collision

#### Discussion
What happens if a developer configures overlapping or nested directory mappings?
```toml
# Accidental overlap example:
[namespaces]
docs = "docs"
adrs = "docs/adrs" # Nested inside docs/!
```
Or duplicate mappings:
```toml
[namespaces]
team_a = "docs/specs"
team_b = "docs/specs"
```

#### The Hazards of Overlap
1. **Duplicate Chunk Overhead**: Every file in `docs/adrs` would be indexed twice (once as `docs` and once as `adrs`), doubling embedding compute, storage, and search latency.
2. **Manifest Race Conditions**: The differential sync manifest (`.manifest.json`) tracks files by relative path (`docs/adrs/001.md`). Having two namespaces claim the same file creates hash collisions and inconsistent deletion tracking.
3. **Breach of the Attention Firewall**: Overlapping directories allow descriptive specifications to leak into normative ADR queries, defeating the fundamental purpose of ADR-007.

#### The Consensus: Strict Defensive Validation
The engine must detect and reject overlapping directory configurations on startup before performing any file scanning or database operations:
- **Exact Duplicate Check**: Reject configurations where `canonicalize(dir_a) == canonicalize(dir_b)`.
- **Directory Nesting Check**: Reject configurations where `canonicalize(dir_a).starts_with(canonicalize(dir_b))`.
- **Actionable Error Diagnostic**:
  ```text
  Error: Overlapping namespaces detected in .intent/config.toml:
    - 'adrs' ("docs/adrs") is a subdirectory of 'docs' ("docs").
  Namespaces must point to mutually disjoint directories to prevent document duplication.
  ```

---

### 2.4 Developer Experience (DevX) vs. Configuration Rigor

#### The DevX Tension
Requiring `[namespaces]` in `config.toml` creates a potential friction point for new users:
- If a first-time user installs `intent` and runs `intent index`, failing immediately with an error asking them to manually create `.intent/config.toml` and look up TOML syntax feels overly bureaucratic.

#### Solutions Evaluated

#### Solution 1: `intent init` Scaffolding (Recommended)
Provide an automated initialization command modeled after `cargo init` or `git init`:
```bash
intent init
```
- Inspects the local workspace.
- Detects common documentation directory layouts (e.g. `docs/adrs`, `docs/specs`, `docs/`, `rfcs/`).
- Generates a fully annotated, commented `.intent/config.toml` with pre-filled namespaces.
- Enables zero-to-running in two keystrokes: `intent init && intent index`.

#### Solution 2: Interactive First-Run Prompt
If `intent index` is executed in a repository with no `.intent/config.toml`:
- Instead of exiting with an error, the CLI checks if standard directories like `docs/` exist.
- Prompts the user:
  ```text
  No .intent/config.toml found.
  Discovered 'docs/' directory. Initialize configuration and index as namespace 'default'? [Y/n]:
  ```
- If confirmed, creates the config file and proceeds with indexing.

#### Solution 3: CLI Namespace Management (`intent namespace add`)
Allows users to manage namespaces without manual TOML editing:
```bash
intent namespace add adrs docs/adrs
intent namespace add specs docs/specs
```
- Performs path validation and collision detection before updating the file.

---

## 3. Summary of Agreed Principles

| Topic | Decision | Rationale |
| :--- | :--- | :--- |
| **Indexing Scope** | Explicit namespaces only (No arbitrary root recursion). | Prevents crawler leaks into build directories (`target`, `node_modules`). |
| **CLI Grammar** | `intent index` or `intent index --namespace <name>`. | Eliminates unmanaged "ghost" directories and ensures declarative reproducibility. |
| **Directory Disjointness** | Strict enforcement of mutually disjoint directory paths. | Prevents vector duplication, manifest race conditions, and firewall leaks. |
| **Onboarding DevX** | `intent init` command and interactive first-run scaffolding. | Eliminates manual TOML syntax friction for first-time adopters. |
| **Storage Schema** | Unified single table with `namespace: Utf8` column. | Columnar SQL pushdown filtering with single-pass hybrid search (~2ms). |

---

## 4. Next Steps

1. Review and refine this RFC with stakeholders.
2. Formalize an ADR update (or companion ADR to ADR-007) ratifying:
   - Disjoint directory validation.
   - Declarative CLI grammar (`intent index [--namespace <name>]`).
   - The `intent init` scaffolding command.
3. Implement the ratified architecture in modular phases:
   - Phase 1: Config schema with `namespaces: HashMap<String, String>` and disjointness validation.
   - Phase 2: Arrow schema `namespace: Utf8` column and LanceDB pushdown filtering.
   - Phase 3: CLI grammar updates and `intent init` scaffolding.
