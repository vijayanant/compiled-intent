# 05-REQ-CLI-AND-CONFIG: Command-Line Interface and Configuration

> [!NOTE] **Architectural Rationale**
> - For the decision to consolidate the CLI and MCP daemon into a single unified binary, see [ADR-006: Unified Binary Architecture and CLI Topology](../adrs/006-unified-binary-architecture.md).
> - For the decision to decouple embedding inference to an external local daemon (Ollama) rather than bundling model weights, see [ADR-008: Decoupled Local Embedding Provider Architecture](../adrs/008-decoupled-local-embedding-provider.md).
> - For the decision to provide a native Q&A subcommand (`intent ask`) with decoupled streaming generation, see [ADR-010: Document Q&A Subcommand](../adrs/010-doc-qa-subcommand.md).
> - For the decision to provide a native pre-commit and CI verification gate (`intent audit`), see [ADR-011: Native Architectural Guardian Subcommand](../adrs/011-native-architectural-guardian-subcommand.md).

## 1. Canonical Binary Topology

The software MUST be packaged as a single standalone executable named **`intent`**.

### 1.1 Global Configuration, Flags, and Project Anchoring

- `--config <path>`: Optional path to a custom configuration file.
- `-v`, `--verbose`: Enables verbose diagnostic logging on `stderr`.
- `-V`, `--version`: Prints the canonical binary version.
- **Upward Config Discovery**: If `--config` is omitted, the engine MUST walk upward from the current working directory searching for `.intent/config.toml`. If found, that directory containing `.intent/` becomes the active project root.
- **Project Root Anchoring**: When `--config <path>` is supplied, the project root is anchored to the parent directory of `.intent/`. The process working directory and all relative paths in configuration (`lancedb_path`, `intent_dir`) resolve relative to this project root.
- If no configuration file is found, the engine MUST initialize using safe internal defaults.

### 1.2 Subcommands

#### `intent index <path>...`

- Scans one or more directories and executes the 5-stage differential indexing pipeline.
- Example: `intent index docs/adrs docs/specs`

#### `intent search <query>`

- Executes a search query from the terminal and prints the results as formatted JSON to `stdout`.
- **Flags**:
  - `--mode <hybrid|fts|vector>` (default: `hybrid`)
  - `--limit <N>` (default: `5`)
  - `--namespace <name>` (optional)

#### `intent ask <question>`

- Retrieves relevant documentation using adaptive hybrid search and queries a local or remote LLM to answer questions at the terminal.
- Emits real-time streamed tokens to `stdout` followed by an attribution summary of consulted documents.
- **Flags**:
  - `--model <name>`: Target model identifier (default: from config `llm.model` or `gemma3:12b`).
  - `--endpoint <url>`: Target HTTP completion endpoint (default: from config `llm.endpoint` or `http://localhost:11434/api/chat`).
  - `--limit <N>`: Candidate over-fetch limit before parent collapsing (default: `10`).
  - `--score-tolerance <tau>`: Relative drop cutoff percentage below top hit (default: `0.15`).
  - `--min-score <s>`: Absolute relevance floor (default: `0.0`).
  - `--namespace <name>`: Restrict retrieval to a specific directory namespace (optional).
  - `--no-stream`: Disable real-time token streaming and emit the full response at completion.
  - `--no-sources`: Omit the citation attribution footer.

#### `intent audit [OPTIONS]`

- Audits staged code changes or specific commits against governing architectural invariants using a local or remote LLM merge gate.
- Emits formatted `[PASS]` or `[VETO]` results with exit code `0` on success and exit code `1` on invariant violation or policy failure.
- **Flags**:
  - `-m, --message <intent>`: Explicit commit message or intent rationale.
  - `-c, --commit <rev>`: Specific commit or git revision to audit (e.g. `HEAD`, `HEAD~1..HEAD`).
  - `-f, --file <path>`: Path to a commit message file (e.g. `.git/COMMIT_EDITMSG`).
  - `--model <name>`: Target model identifier (default: from config `llm.model` or `qwen2.5-coder:14b`).
  - `--endpoint <url>`: Target HTTP completion endpoint (default: from config `llm.endpoint` or `http://localhost:11434/api/chat`).
  - `--limit <N>`: Candidate over-fetch limit before parent collapsing (default: `3`).
  - `--namespace <name>`: Restrict retrieval to a specific directory namespace (optional).
  - `--strict`: Terminate with exit code `1` if the LLM inference server is unreachable.

#### `intent serve`

- Starts the headless Model Context Protocol (MCP) server daemon on `stdin`/`stdout`.

#### Zero-Argument Invocation Invariant

- Executing `intent` with zero arguments MUST automatically default to `intent serve`. This enables IDEs and MCP hosts to configure `command = "intent"` without requiring subcommands.

## 2. Configuration Schema (`config.toml`)

The configuration file format is TOML:

```toml
# Database Storage
lancedb_path = ".intent/lancedb"
table_name = "adrs"
log_level = "warn" # Options: error, warn, info, debug

# External Embedding Model (ADR-008)
llama_endpoint = "http://localhost:11434/api/embeddings"
embedding_model = "nomic-embed-text"
embedding_dimensions = 768

# LLM Generation for `intent ask` (ADR-010)
[llm]
endpoint = "http://localhost:11434/api/chat"
model = "gemma3:12b"
temperature = 0.0

# Configurable Namespaces
[namespaces]
adrs = "docs/adrs"
specs = "docs/specs"
```

### 2.1 Configuration Precedence

1. Explicit CLI arguments (e.g. `--mode vector`, `--limit 10`, `--model gemma3:12b`).
2. Values specified in the loaded `config.toml`.
3. Internal engine defaults.

