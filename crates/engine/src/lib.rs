//! Compitent Engine Library
//!
//! A Universal Semantic Memory Layer for AI Agents

pub const DEFAULT_TABLE_NAME: &str = "adrs";

pub mod config;
pub mod engine;
pub mod error;
pub mod indexer;
pub mod telemetry;

pub use error::CompitentError;
