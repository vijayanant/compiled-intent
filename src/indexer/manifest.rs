use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use crate::error::CompitentError;

pub const MANIFEST_FILENAME: &str = ".manifest.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaFile {
    pub path: PathBuf,
    pub filename: String,
    pub content: String,
    pub hash: String,
}

#[derive(Debug, Default, Clone)]
pub struct Manifest {
    entries: HashMap<String, String>,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> Self {
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(path) {
                if let Ok(entries) = serde_json::from_str(&data) {
                    return Self { entries };
                }
            }
        }
        Self::new()
    }

    pub fn save(&self, path: &Path) -> Result<(), CompitentError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn contains_file(&self, filename: &str) -> bool {
        self.entries.contains_key(filename)
    }

    pub fn update(&mut self, filename: String, hash: String) {
        self.entries.insert(filename, hash);
    }

    pub fn compute_deltas(&self, files: &[PathBuf]) -> Result<Vec<DeltaFile>, CompitentError> {
        let mut deltas = Vec::new();
        for file_path in files {
            let filename = file_path.display().to_string();
            let content = std::fs::read_to_string(file_path)?;
            let current_hash = Self::hash_content(&content);

            let needs_update = match self.entries.get(&filename) {
                Some(stored_hash) => stored_hash != &current_hash,
                None => true,
            };

            if needs_update {
                deltas.push(DeltaFile {
                    path: file_path.clone(),
                    filename,
                    content,
                    hash: current_hash,
                });
            }
        }
        Ok(deltas)
    }

    pub fn hash_content(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for byte in hash {
            use std::fmt::Write;
            write!(&mut hex, "{:02x}", byte).unwrap();
        }
        hex
    }

    pub fn extract_metadata(content: &str) -> (String, String) {
        if content.starts_with("---\n") || content.starts_with("---\r\n") {
            if let Some(end_idx) = content[4..].find("---\n").map(|i| i + 4) {
                let yaml_str = &content[4..end_idx];
                if let Ok(json_val) = serde_yaml::from_str::<serde_json::Value>(yaml_str) {
                    let rest = &content[end_idx + 4..];
                    return (json_val.to_string(), rest.to_string());
                }
            }
        }
        ("{}".to_string(), content.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_content_determinism() {
        let text = "Hello world";
        let h1 = Manifest::hash_content(text);
        let h2 = Manifest::hash_content(text);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        // Known SHA256 of "Hello world"
        assert_eq!(h1, "64ec88ca00b268e5ba1a35678a1b5316d212f4f366b2477232534a8aeca37f3c");
    }

    #[test]
    fn test_extract_metadata_with_frontmatter() {
        let content = "---\ntitle: ADR Test\nstatus: accepted\n---\n# Body\nSome text";
        let (metadata, body) = Manifest::extract_metadata(content);
        assert!(metadata.contains("\"title\":\"ADR Test\""));
        assert!(metadata.contains("\"status\":\"accepted\""));
        assert_eq!(body, "# Body\nSome text");
    }

    #[test]
    fn test_extract_metadata_without_frontmatter() {
        let content = "# Body\nSome text";
        let (metadata, body) = Manifest::extract_metadata(content);
        assert_eq!(metadata, "{}");
        assert_eq!(body, content);
    }
}
