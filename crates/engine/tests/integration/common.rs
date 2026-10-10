use compitent_engine::config::EngineConfig;
use compitent_engine::engine::CompitentEngine;
use std::fs;
use std::path::PathBuf;

/// Helper to create an isolated temporary directory cleaned up on Drop.
pub struct TempTestDir {
    pub path: PathBuf,
}

impl TempTestDir {
    pub fn new(prefix: &str) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), nanos));
        fs::create_dir_all(&path).expect("Failed to create temp test dir");
        Self { path }
    }

    pub fn sub_dir(&self, name: &str) -> PathBuf {
        let sub = self.path.join(name);
        fs::create_dir_all(&sub).expect("Failed to create sub dir");
        sub
    }

    pub fn write_file(&self, filename: &str, content: &str) -> PathBuf {
        let file_path = self.path.join(filename);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(&file_path, content).expect("Failed to write test file");
        file_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Creates a test engine instance pointed to an isolated temp directory.
pub fn create_test_engine(temp_dir: &TempTestDir) -> CompitentEngine {
    let config = EngineConfig {
        llama_endpoint: "http://127.0.0.1:8080/v1/embeddings".to_string(),
        embedding_model: "test-embedding-model".to_string(),
        embedding_dimensions: 384,
        intent_dir: temp_dir.path.display().to_string(),
        lancedb_path: temp_dir.path.join("lancedb").display().to_string(),
        ..Default::default()
    };
    CompitentEngine::new(config)
}
