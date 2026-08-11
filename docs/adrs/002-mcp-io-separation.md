# ADR 002: MCP I/O Separation (Stdout vs Stderr)

## Status
Accepted

## Context
Chronicle acts as an MCP (Model Context Protocol) server communicating over Standard Input/Output (Stdio). The MCP specification strictly dictates that the communication channel between the AI IDE (the client) and Chronicle (the server) happens over `stdout`. The client expects *only* valid JSON-RPC messages on this channel.

If a developer accidentally prints a debug message, a loading bar, or a panic trace to `stdout`, the client's JSON parser will instantly crash, breaking the entire proxy connection.

## Decision
We enforce a strict, absolute boundary on process outputs:
1.  **`stdout` is for the Protocol ONLY:** The `println!` macro (or any write to `io::stdout`) is strictly reserved for serializing JSON-RPC responses in the `mcp_server.rs` module. It must never be used anywhere else in the codebase.
2.  **`stderr` is for Logging:** All debug messages, errors, warnings, and informational logs must be written to `stderr` using `eprintln!` (or a dedicated logging framework configured to output to `stderr`). The MCP client will safely route `stderr` to the developer's console without breaking the protocol.

## Consequences
- Developers must be hyper-vigilant not to use `println!` for debugging in the `engine.rs` module.
- We may eventually want to introduce a strict logging facade (like `tracing` or `log`) configured to write exclusively to `stderr` to programmatically enforce this rule.
