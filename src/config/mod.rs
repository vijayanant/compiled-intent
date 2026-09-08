use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EngineConfig {
    pub llama_endpoint: String,
    pub embedding_model: String,
    pub embedding_dimensions: i32,
    pub intent_dir: String,
    pub lancedb_path: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            llama_endpoint: "http://localhost:8089/v1/embeddings".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            embedding_dimensions: 768,
            intent_dir: ".intent".to_string(),
            lancedb_path: ".intent/lancedb".to_string(),
        }
    }
}

impl EngineConfig {
    pub fn load(custom_path: Option<String>) -> Self {
        let is_custom = custom_path.is_some();
        let config_path = custom_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".intent/config.toml"));

        match std::fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => return config,
                Err(e) if is_custom => {
                    tracing::warn!("Failed to parse custom config '{}': {}", config_path.display(), e);
                }
                _ => {}
            },
            Err(e) if is_custom => {
                tracing::warn!("Failed to read custom config '{}': {}", config_path.display(), e);
            }
            _ => {}
        }
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EngineConfig::default();
        assert_eq!(config.intent_dir, ".intent");
        assert_eq!(config.llama_endpoint, "http://localhost:8089/v1/embeddings");
    }

    #[test]
    fn test_load_fallback_to_default() {
        // If file doesn't exist, it should gracefully fall back to default
        let config = EngineConfig::load(None);
        assert_eq!(config.lancedb_path, ".intent/lancedb");
    }
}
