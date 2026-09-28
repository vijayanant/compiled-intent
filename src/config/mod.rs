use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LlmConfig {
    pub endpoint: String,
    pub model: String,
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434/api/chat".to_string(),
            model: "gemma3:12b".to_string(),
            temperature: 0.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EngineConfig {
    pub llama_endpoint: String,
    pub embedding_model: String,
    pub embedding_dimensions: i32,
    pub intent_dir: String,
    pub lancedb_path: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub llm: LlmConfig,
}

fn default_log_level() -> String {
    "warn".to_string()
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            llama_endpoint: "http://localhost:8089/v1/embeddings".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            embedding_dimensions: 768,
            intent_dir: ".intent".to_string(),
            lancedb_path: ".intent/lancedb".to_string(),
            log_level: default_log_level(),
            llm: LlmConfig::default(),
        }
    }
}

impl EngineConfig {
    pub fn load(custom_path: Option<String>) -> Self {
        let is_custom = custom_path.is_some();
        let config_path = custom_path.map(PathBuf::from).unwrap_or_else(|| {
            if let Ok(mut cur) = std::env::current_dir() {
                loop {
                    let candidate = cur.join(".intent/config.toml");
                    if candidate.is_file() {
                        return candidate;
                    }
                    if !cur.pop() {
                        break;
                    }
                }
            }
            PathBuf::from(".intent/config.toml")
        });

        match std::fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => return config,
                Err(e) if is_custom => {
                    tracing::warn!(
                        "Failed to parse custom config '{}': {}",
                        config_path.display(),
                        e
                    );
                }
                _ => {}
            },
            Err(e) if is_custom => {
                tracing::warn!(
                    "Failed to read custom config '{}': {}",
                    config_path.display(),
                    e
                );
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
    fn test_custom_config_override_and_malformed_fallback() {
        let temp_dir = std::env::temp_dir().join(format!("intent_cfg_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. Valid custom config overrides defaults
        let valid_path = temp_dir.join("valid.toml");
        let valid_toml = r#"
            llama_endpoint = "http://custom-host:9999/v1/embeddings"
            embedding_model = "bge-large"
            embedding_dimensions = 1024
            intent_dir = ".custom_intent"
            lancedb_path = ".custom_intent/lancedb"
        "#;
        std::fs::write(&valid_path, valid_toml).unwrap();

        let cfg = EngineConfig::load(Some(valid_path.display().to_string()));
        assert_eq!(cfg.llama_endpoint, "http://custom-host:9999/v1/embeddings");
        assert_eq!(cfg.embedding_model, "bge-large");
        assert_eq!(cfg.embedding_dimensions, 1024);
        assert_eq!(cfg.intent_dir, ".custom_intent");

        // 2. Malformed config logs warning and falls back to default safely
        let malformed_path = temp_dir.join("malformed.toml");
        std::fs::write(&malformed_path, "invalid_toml = [[ broken syntax").unwrap();

        let fallback_cfg = EngineConfig::load(Some(malformed_path.display().to_string()));
        assert_eq!(fallback_cfg.intent_dir, ".intent");
        assert_eq!(fallback_cfg.embedding_dimensions, 768);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_load_fallback_to_default_when_none_provided() {
        let config = EngineConfig::load(None);
        assert_eq!(config.lancedb_path, ".intent/lancedb");
        assert_eq!(config.llm.model, "gemma3:12b");
        assert_eq!(config.llm.endpoint, "http://localhost:11434/api/chat");
    }

    #[test]
    fn test_llm_config_custom_override() {
        let temp_dir = std::env::temp_dir().join(format!("intent_llm_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let path = temp_dir.join("llm.toml");
        let toml_str = r#"
            llama_endpoint = "http://localhost:11434/api/embeddings"
            embedding_model = "nomic-embed-text"
            embedding_dimensions = 768
            intent_dir = ".intent"
            lancedb_path = ".intent/lancedb"

            [llm]
            endpoint = "http://remote:11434/api/chat"
            model = "qwen2.5-coder:7b"
            temperature = 0.2
        "#;
        std::fs::write(&path, toml_str).unwrap();

        let cfg = EngineConfig::load(Some(path.display().to_string()));
        assert_eq!(cfg.llm.endpoint, "http://remote:11434/api/chat");
        assert_eq!(cfg.llm.model, "qwen2.5-coder:7b");
        assert!((cfg.llm.temperature - 0.2).abs() < 1e-5);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
