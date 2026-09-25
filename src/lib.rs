//! Compitent Engine
//!
//! A Universal Semantic Memory Layer for AI Agents

pub const DEFAULT_TABLE_NAME: &str = "adrs";

pub mod config;
pub mod engine;
pub mod indexer;
pub mod llm;
pub mod mcp;
pub mod telemetry;

// Re-export core types
pub use error::CompitentError;

pub mod error {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum CompitentError {
        #[error("IO error: {0}")]
        Io(#[from] std::io::Error),
        #[error("Database error: {0}")]
        Database(String),
        #[error("LanceDB error: {0}")]
        LanceDb(#[from] lancedb::Error),
        #[error("Arrow error: {0}")]
        Arrow(#[from] arrow::error::ArrowError),
        #[error("JSON error: {0}")]
        Json(#[from] serde_json::Error),
        #[error("HTTP error: {0}")]
        Http(#[from] reqwest::Error),
        #[error("Network error: {0}")]
        Network(String),
        #[error("Internal error: {0}")]
        Internal(String),
        #[error("Configuration error: {0}")]
        Config(String),
    }
}
