//! Integration Test Suite for Intent Tooling
//!
//! Subsystems:
//! - `common`: Shared test directory lifecycle and mock engine initialization
//! - `mcp_protocol`: JSON-RPC 2.0 handshake, stdio purity, and tool dispatch
//! - `llm_client`: Streaming and complete chat completion with `Ollama` / `OpenAI`

mod integration {
    pub mod common;
    pub mod llm_client;
    pub mod mcp_protocol;
}
