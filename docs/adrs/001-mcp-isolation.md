# ADR 001: MCP Protocol Isolation

## Status
Accepted

## Context
Chronicle acts as an MCP (Model Context Protocol) server. However, the MCP specification is relatively new and may evolve, or we may eventually want to run Chronicle as a standalone CLI tool without an AI IDE. 

## Decision
The core intelligence of Chronicle (RAG logic, vector database querying, embedding generation) must remain entirely ignorant of the MCP protocol. 
1. The MCP JSON-RPC handling logic must be strictly isolated (e.g., in an `mcp_server.rs` module or a dedicated `chronicle-mcp` crate).
2. The core logic must live in a separate `engine` module that exposes pure Rust structs and traits. The MCP layer simply acts as a dumb Stdio router that calls the `engine`.

## Consequences
- We can easily swap out the MCP implementation if a better Rust SDK is released.
- We can run Chronicle from the command line (`chronicle audit`) by simply bypassing the MCP layer and calling the core Rust engine directly.
