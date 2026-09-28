//! Integration Test Suite for Compitent Engine
//!
//! Subsystems:
//! - `common`: Shared test directory lifecycle and mock engine initialization
//! - `indexer_pipeline`: Full ingestion, chunking, LanceDB write, and differential sync
//! - `retrieval_engine`: Tantivy FTS exact symbol retrieval and `get_context` expansion

mod integration {
    pub mod common;
    pub mod indexer_pipeline;
    pub mod retrieval_engine;
}
