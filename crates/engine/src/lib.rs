//! Compitent Engine Library
//!
//! A Universal Semantic Memory Layer for AI Agents

pub const DEFAULT_TABLE_NAME: &str = "adrs";

pub mod config;
pub mod error;
pub mod indexer;
pub mod retrieval;
pub mod telemetry;

// Re-export for seamless backward compatibility and ergonomics
pub use error::CompitentError;
pub use retrieval as engine;
pub use retrieval::{CompitentEngine, SearchFilterOptions};
