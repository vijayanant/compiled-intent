use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EngineConfig {
    pub llama_endpoint: String,
    pub embedding_model: String,
    pub embedding_dimensions: i32,
    pub chronicle_dir: String,
    pub lancedb_path: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            llama_endpoint: "http://localhost:8089/v1/embeddings".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            embedding_dimensions: 768,
            chronicle_dir: ".chronicle".to_string(),
            lancedb_path: ".chronicle/lancedb".to_string(),
        }
    }
}

impl EngineConfig {
    pub fn load(custom_path: Option<String>) -> Self {
        let config_path = custom_path.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".chronicle/config.toml"));
        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str(&contents) {
                return config;
            }
        }
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_config() {
        let config = EngineConfig::default();
        assert_eq!(config.chronicle_dir, ".chronicle");
        assert_eq!(config.llama_endpoint, "http://localhost:8089/v1/embeddings");
    }

    #[test]
    fn test_load_fallback_to_default() {
        // If file doesn't exist, it should gracefully fall back to default
        let config = EngineConfig::load(None);
        assert_eq!(config.lancedb_path, ".chronicle/lancedb");
    }
}
