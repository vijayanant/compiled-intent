# 05-REQ-CLI-AND-CONFIG: Command-Line Interface and Configuration

> [!NOTE] **Architectural Rationale**
> For the decision to consolidate the CLI and MCP daemon into a single unified binary, see [ADR-006: Unified Binary Architecture and CLI Topology](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/adrs/006-unified-binary-architecture.md).

## 1. Canonical Binary Topology

The software MUST be packaged as a single standalone executable named **`intent`**.

### 1.1 Global Configuration Resolution

- The CLI MUST accept an optional global flag `--config <path>` preceding any subcommand.
- If `--config` is omitted, the engine MUST look for `.intent/config.toml` in the current working directory.
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

# External Embedding Model
llama_endpoint = "http://localhost:11434/api/embeddings"
embedding_model = "nomic-embed-text"
embedding_dimensions = 768

# Configurable Namespaces
[namespaces]
adrs = "docs/adrs"
specs = "docs/specs"
```

### 2.1 Configuration Precedence

1. Explicit CLI arguments (e.g. `--mode vector`, `--limit 10`).
2. Values specified in the loaded `config.toml`.
3. Internal engine defaults.
