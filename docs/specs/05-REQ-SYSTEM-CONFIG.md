# 05-REQ-SYSTEM-CONFIG

## 1. Overview

The Intent Engine MUST operate not just as an MCP server, but as a standalone, configurable binary with robust observability and user tooling.

## 2. Configuration Management (`EngineConfig`)

**Description:** The system MUST support dynamic configuration for database paths, embedding models, and dimensions.
**Requirements:**

- The engine MUST attempt to load a configuration file (e.g., `intent.toml`) if a path is provided via the `--config` CLI argument.
- If no configuration is provided, it MUST gracefully fall back to sensible defaults (e.g., `.intent_db` in the local workspace).
- The `EngineConfig` struct MUST dictate the `embedding_dimensions` expected by the Apache Arrow schema.

**Acceptance Criteria:**

- **Given** a valid `intent.toml`, **When** the binary starts with `--config path/to/toml`, **Then** the LanceDB connection path and dimensions MUST reflect the file's contents.

## 3. Command Line Interface (CLI)

**Description:** A thin CLI wrapper (`intent`) MUST be maintained alongside the MCP binary for human operators.
**Requirements:**

- Uses the `clap` crate for declarative argument parsing.
- MUST support the following subcommands:
  - `index <path>`: Manually triggers the indexing of a directory.
  - `search <query> --limit <N>`: Manually tests the semantic search retrieval.

## 4. Observability and Telemetry

**Description:** The system MUST emit structured, leveled logs.
**Requirements:**

- MUST initialize a `tracing-subscriber` globally at application startup.
- Logs MUST default to `INFO` level, configurable via the `RUST_LOG` environment variable.
- MCP stdio communication MUST NOT be corrupted by standard log output (e.g., if MCP runs on `stdout`, logs MUST go to `stderr` or a file, to prevent JSON-RPC parsing failures on the client side).
