//! Compitent Engine
//! 
//! A Universal Semantic Memory Layer for AI Agents

pub const DEFAULT_TABLE_NAME: &str = "adrs";

pub mod telemetry;
pub mod config;
pub mod engine;
pub mod indexer;
pub mod mcp;

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
        #[error("Network error: {0}")]
        Network(String),
        #[error("Internal error: {0}")]
        Internal(String),
        #[error("Configuration error: {0}")]
        Config(String),
    }
}
