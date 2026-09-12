# ADR 005: Headless Stdio MCP Protocol and I/O Isolation

## Status

Accepted

## Context

Addresses [REQ-01: MCP Interface](file:///Users/vj/workspace/blogging-assistant/compiled-intent/docs/specs/01-REQ-MCP-INTERFACE.md).

AI coding assistants (Claude Desktop, Cursor, Antigravity, VS Code Copilot) require real-time access to the local architectural memory layer. The communication protocol must be cross-platform, secure, instantaneous, and zero-configuration.

## The Dilemma

What protocol transport should be used to connect IDE agents to Compiled Intent without introducing network friction or process lifecycle hazards?

## Options Evaluated

### Option 1: Local HTTP/REST Server with Server-Sent Events (SSE)

- **Pros**: Standard web tooling; easy to debug via curl or browser.
- **Cons**: Severe operational friction. Requires binding to a local TCP port (e.g. `localhost:8080`), causing port collisions if multiple workspaces or instances run concurrently. Prompts macOS/Windows firewall permission popups. When the parent editor exits, background HTTP server daemons are frequently orphaned as zombie processes consuming memory.

### Option 2: Unix Domain Sockets (UDS) / Named Pipes

- **Pros**: Fast IPC with filesystem permission security.
- **Cons**: Non-portable. Named pipes on Windows behave fundamentally differently from POSIX domain sockets on macOS/Linux. Socket file cleanup requires complex crash recovery logic to avoid stale socket errors.

### Option 3: Model Context Protocol (MCP) over Stdio

- **Pros**: The industry standard for AI tool execution. Uses standard input/output (`stdin`/`stdout`) passing newline-delimited JSON-RPC 2.0 messages. Zero network configuration; zero port collisions. Process lifetime is automatically bound to the parent IDE (when the editor closes the pipe, the engine exits cleanly with zero zombie processes).
- **Cons**: Requires absolute, unyielding I/O discipline: any rogue print to `stdout` breaks the JSON stream.

## Decision

We choose **Option 3: Model Context Protocol (MCP) over Stdio**.

The engine runs headlessly, processing JSON-RPC 2.0 requests over `stdin` and emitting responses strictly over `stdout`.

### Non-Negotiable Invariant: Stdio Isolation

Because standard output is the dedicated JSON-RPC transport medium:

- **`stdout` is strictly reserved for valid JSON-RPC 2.0 frames**. Under no circumstances may application code, debug statements, third-party libraries, or panic hooks write to `stdout`.
- **`stderr` is the dedicated diagnostic channel**. All telemetry, `tracing` subscribers, informational logs, and error traces MUST write exclusively to `stderr`.

## Rejected Alternatives

- **Local HTTP / SSE**: Rejected because TCP port collisions, firewall prompts, and zombie daemon lifecycles create unacceptable developer friction.
- **Unix Domain Sockets**: Rejected due to Windows named-pipe incompatibilities and socket file cleanup overhead.

## Consequences & Trade-offs

- **Positive**: Seamless integration into Claude Desktop, Cursor, and Antigravity with zero configuration; automatic process lifecycle management.
- **Enforced Invariant**: Code changes must never write diagnostic output to `stdout`. All logging must be verified via `tracing` directed to `stderr`.
