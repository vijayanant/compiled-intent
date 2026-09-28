# ADR 011: Native Architectural Guardian Subcommand (`intent audit`)

## Status

Accepted

## Context

Addresses [REQ-02: Storage and Retrieval](../specs/02-REQ-STORAGE-AND-RETRIEVAL.md), [REQ-05: CLI and Configuration](../specs/05-REQ-CLI-AND-CONFIG.md), and [REQ-06: Namespaces](../specs/06-REQ-NAMESPACES.md).

The central thesis of the *Compiled Intent* research paper is bridging the **Intent Gap**—the divergence between high-level architectural invariants (recorded in ADRs, specs, and architectural RFCs) and everyday code changes introduced via commits and pull requests.

While `compiled-intent` provides indexing, semantic retrieval (`intent search`), IDE agent integration via MCP (`intent serve`), and interactive document Q&A (`intent ask`), automated pre-commit and CI/CD verification gates currently require developers to maintain external scripts (e.g. Python wrappers invoking `intent search` and calling an LLM daemon).

Relying on external scripts introduces installation friction, Python/runtime dependencies, fragile git diff parsing, and inconsistent prompt schemas across teams.

## The Dilemma

How should pre-commit and CI/CD architectural compliance auditing be integrated into the `compiled-intent` toolchain?

## Options Evaluated

### Option 1: External Wrapper Scripts / Plugins
- **Pros**: Keeps the `intent` Rust binary strictly focused on retrieval.
- **Cons**: High friction for developers. Requires Python (`uv`, virtualenvs), shell utilities, or third-party wrappers in every repository. Fragile pre-commit setup and maintenance overhead.

### Option 2: In-Process Bundled LLM Inference (Candle / ONNX Runtime)
- **Pros**: Standalone binary with zero external daemon requirements.
- **Cons**: Extreme binary bloat (multi-gigabyte weights), massive compile times, and heavy GPU/Metal hardware acceleration maintenance overhead. Violates [ADR-008](./008-decoupled-local-embedding-provider.md).

### Option 3: Native CLI Subcommand with Decoupled HTTP Inference (`intent audit`)
- **Pros**:
  - **Single Binary Ergonomics**: Aligns directly with [ADR-006: Unified Binary Architecture](./006-unified-binary-architecture.md). Installing the `intent` binary gives teams an immediate, out-of-the-box pre-commit and CI merge gate without any external runtime dependencies.
  - **Zero Binary Bloat**: Reuses the asynchronous `reqwest` and `tokio` runtime established in [ADR-008](./008-decoupled-local-embedding-provider.md) and [ADR-010](./010-doc-qa-subcommand.md). Keeps the compiled binary under 15MB.
  - **Precise Invariant Slicing**: Queries the local LanceDB index and Tantivy FTS engine directly with parent chunk collapsing and namespace filtering ([ADR-007](./007-directory-namespaces-inter-document-firewall.md)), delivering high-density ~500-token context slices directly to local LLMs (Ollama `qwen2.5-coder:14b`, `gemma3:12b`, etc.).
  - **Deterministic Merge Gate**: Enforces `temperature: 0.0`, structured JSON verdict evaluation, and emits standard UNIX exit codes: `0` for `[PASS]` and `1` for `[VETO]`.
- **Cons**: Adds a new subcommand and Git diff extraction logic to the CLI.

## Decision

We choose **Option 3: Native CLI Subcommand with Decoupled HTTP Inference (`intent audit`)**.

The `intent` CLI exposes the `audit` subcommand:

```bash
intent audit [OPTIONS]
```

### CLI Grammar & Options

- `-m, --message <intent>`: Explicit commit message or intent rationale.
- `-c, --commit <rev>`: Specific commit or git revision to audit (e.g. `HEAD`, `HEAD~1..HEAD`).
- `-f, --file <path>`: Path to a commit message file (e.g. `.git/COMMIT_EDITMSG`).
- `--model <name>`: LLM model identifier (overrides `[llm].model` in `config.toml` or defaults to `qwen2.5-coder:14b`).
- `--endpoint <url>`: LLM HTTP completion endpoint (overrides `[llm].endpoint` in `config.toml`).
- `--namespace <name>`: Restrict invariant retrieval to a specific namespace (e.g. `adrs`).
- `--limit <N>`: Candidate over-fetch limit before parent collapsing (default: `3`).
- `--min-score <s>`: Minimum normalized confidence score floor in `[0.0, 1.0]`.
- `--strict`: In CI/pre-commit, fail with exit code `1` if the LLM inference server is unreachable.

### Execution Flow

1. **Diff & Message Resolution**:
   - If `--commit <rev>` is supplied, extracts the commit message via `git log -1 --pretty=%B <rev>` and diff via `git diff <rev>~1..<rev>`.
   - Otherwise, extracts staged git changes via `git diff --cached` (falling back to `git diff HEAD` if no changes are staged).
   - Resolves commit message from `-m`, `--file`, standard input (if piped), or interactive prompt.
   - If no code changes exist in the diff, outputs a notice and exits cleanly with code `0`.
2. **Invariant Retrieval**:
   - Queries the local LanceDB / Tantivy index using hybrid search with parent chunk collapsing.
   - If no invariants match the intent stimulus, outputs `[PASS] Unconstrained by known invariants.` and exits with code `0`.
3. **Decoupled Evaluation**:
   - Formulates a system prompt defining the auditor role and JSON response schema (`{"verdict": "PASS"|"VETO", "violated_invariant": ..., "rationale": ...}`).
   - Sends the payload to the local or remote LLM endpoint at `temperature: 0.0`.
4. **Exit Codes & Output**:
   - If `verdict == "PASS"`: emits clean `[PASS]` summary with rationale to `stdout` and exits with code `0`.
   - If `verdict == "VETO"`: emits `[VETO]` with the violated invariant and rationale to `stderr` and exits with code `1`.

## Consequences & Trade-offs

- **Positive**: Native pre-commit and CI/CD guardian workflow with zero Python or external tooling dependencies. Instant setup in `.git/hooks/pre-commit` or GitHub Actions.
- **Accepted Trade-off**: Requires access to a local (Ollama) or remote OpenAI-compatible inference endpoint. If unreachable, emits a clear actionable error on `stderr` and respects the `--strict` flag.
