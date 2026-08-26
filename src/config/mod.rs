#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub lancedb_path: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            lancedb_path: ".lancedb".to_string(),
        }
    }
}
