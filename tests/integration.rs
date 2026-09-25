//! Integration Test Suite for Compiled Intent
//!
//! Subsystems:
//! - `common`: Shared test directory lifecycle and mock engine initialization
//! - `indexer_pipeline`: Full ingestion, chunking, LanceDB write, and differential sync
//! - `mcp_protocol`: JSON-RPC 2.0 handshake, stdio purity, and tool dispatch
//! - `retrieval_engine`: Tantivy FTS exact symbol retrieval and `get_context` expansion

mod integration {
    pub mod common;
    pub mod indexer_pipeline;
    pub mod llm_client;
    pub mod mcp_protocol;
    pub mod retrieval_engine;
}
