# ADR 006: Unified Binary Architecture and CLI Topology

## Status

Accepted

## Context

Addresses [REQ-05: CLI and Config](../specs/05-REQ-CLI-AND-CONFIG.md).

Compiled Intent is used in two primary environments:

1. **Interactive Human & CI/CD Tooling**: Developers running ad-hoc searches (`intent search`) or pre-commit scripts forcing differential indexing (`intent index`).
2. **Headless IDE Agent Integration**: AI tools launching an automated daemon over standard I/O to serve MCP requests.

## The Dilemma

How should the codebase be packaged and executed? As separate purpose-built binaries or as a consolidated canonical tool?

## Options Evaluated

### Option 1: Split Binaries (`chronicle` CLI and `compiled-intent` MCP Server)

- **Pros**: Clear separation of entrypoints; smaller individual binary concerns.
- **Cons**: Severe packaging and mental overhead. End users and package managers must install two distinct executables. Documentation becomes fragmented between two binary names, and internal dependency sharing inflates compilation times.

### Option 2: Daemon-Client IPC Architecture

- **Pros**: LanceDB table remains held in memory across commands.
- **Cons**: Excessive complexity. Requires a background daemon (`intentd`) that must be started, stopped, monitored, and recovered if killed. LanceDB is already an embedded zero-copy `mmap` engine that opens tables in 2–5ms; introducing daemon IPC overhead provides negligible latency gain while adding high operational risk.

### Option 3: Unified Canonical Binary (`intent`) with Subcommand Routing

- **Pros**: Single executable (`intent`) with zero external runtime dependencies. Supports explicit subcommands (`intent index <path>`, `intent search <query>`, `intent serve`). Running `intent` with zero arguments defaults automatically to `serve` on `stdio`, allowing IDEs to point directly to `intent` without configuring subcommand flags.
- **Cons**: Single `main.rs` binary must handle both command-line argument routing and long-running MCP event loops.

## Decision

We choose **Option 3: Unified Canonical Binary (`intent`)**.

A single binary named `intent` serves all use cases:

- `intent index <path>`: Runs offline differential indexing.
- `intent search <query>`: Runs CLI vector/FTS/hybrid queries.
- `intent serve`: Runs the headless MCP server daemon.
- `intent` (zero arguments): Automatically falls back to `serve`, making IDE MCP configuration dead simple.

## Rejected Alternatives

- **Split Binaries**: Discarded to eliminate naming confusion and streamline cargo packaging.
- **Daemon-Client Architecture**: Discarded because embedded LanceDB cold-start latency (<5ms) makes a persistent background daemon unnecessary.

## Consequences & Trade-offs

- **Positive**: Single binary to build, test, package, and deploy; zero background daemon management; zero-configuration IDE setup.
- **Accepted Trade-off**: Global configuration flags (`--config <path>`) must be parsed at the root CLI router before delegating to subcommands.
