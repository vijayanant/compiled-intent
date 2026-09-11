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

    pub fn remove(&mut self, filename: &str) {
        self.entries.remove(filename);
    }

    pub fn find_orphans<P: AsRef<Path>>(&self, adr_paths: &[P], discovered_files: &[PathBuf]) -> Vec<String> {
        let discovered_set: std::collections::HashSet<PathBuf> = discovered_files.iter().cloned().collect();

        self.entries
            .keys()
            .filter(|existing_str| {
                let existing_path = Path::new(existing_str);
                let under_scanned_dir = adr_paths.iter().any(|dir| existing_path.starts_with(dir.as_ref()));
                under_scanned_dir && !discovered_set.contains(existing_path)
            })
            .cloned()
            .collect()
    }

    pub fn compute_deltas(&self, files: &[PathBuf]) -> Result<Vec<DeltaFile>, CompitentError> {
        let mut deltas = Vec::new();
        for file_path in files {
            let filename = file_path.display().to_string();
            let content = std::fs::read_to_string(file_path)?;
            let current_hash = Self::hash_content(&content);

            let needs_update = self.entries.get(&filename) != Some(&current_hash);

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
        let search_slice = if let Some(stripped) = content.strip_prefix("---\r\n") {
            stripped
        } else if let Some(stripped) = content.strip_prefix("---\n") {
            stripped
        } else {
            return ("{}".to_string(), content.to_string());
        };

        if let Some(end_rel) = search_slice.find("\n---").or_else(|| search_slice.find("\r\n---")) {
            let yaml_str = &search_slice[..end_rel];
            let after_fence_start = &search_slice[end_rel..];
            let fence_marker = if let Some(stripped) = after_fence_start.strip_prefix("\r\n---") {
                stripped
            } else if let Some(stripped) = after_fence_start.strip_prefix("\n---") {
                stripped
            } else {
                after_fence_start
            };
            let body = if let Some(stripped) = fence_marker.strip_prefix("\r\n") {
                stripped
            } else if let Some(stripped) = fence_marker.strip_prefix('\n') {
                stripped
            } else {
                fence_marker
            };

            if let Ok(json_val) = serde_yaml::from_str::<serde_json::Value>(yaml_str) {
                return (json_val.to_string(), body.to_string());
            }
        }

        ("{}".to_string(), content.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_metadata_crlf_and_lf_parity() {
        let lf_content = "---\ntitle: ADR Test\nstatus: accepted\n---\n# Body\nSome text";
        let crlf_content = "---\r\ntitle: ADR Test\r\nstatus: accepted\r\n---\r\n# Body\r\nSome text";

        let (meta_lf, body_lf) = Manifest::extract_metadata(lf_content);
        let (meta_crlf, body_crlf) = Manifest::extract_metadata(crlf_content);

        assert_eq!(meta_lf, meta_crlf, "Metadata JSON must match regardless of CRLF or LF line endings");
        assert!(meta_lf.contains("\"title\":\"ADR Test\""));
        assert_eq!(body_lf, "# Body\nSome text");
        assert_eq!(body_crlf, "# Body\r\nSome text");
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

    #[test]
    fn test_extract_metadata_malformed_yaml() {
        let content = "---\n: invalid [yaml: {\n---\n# Body\nSome text";
        let (metadata, body) = Manifest::extract_metadata(content);
        assert_eq!(metadata, "{}");
        assert_eq!(body, content);
    }

    #[test]
    fn test_manifest_save_and_load() {
        let temp_dir = std::env::temp_dir().join(format!("intent_test_{}", std::process::id()));
        let manifest_path = temp_dir.join(".manifest.json");

        let mut manifest = Manifest::new();
        manifest.update("file1.md".to_string(), "hash123".to_string());
        manifest.save(&manifest_path).expect("Should save manifest");

        let loaded = Manifest::load(&manifest_path);
        assert!(loaded.contains_file("file1.md"));
        assert!(!loaded.contains_file("file2.md"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_orphans_and_remove() {
        let mut manifest = Manifest::new();
        manifest.update("docs/adrs/001.md".to_string(), "h1".to_string());
        manifest.update("docs/adrs/002.md".to_string(), "h2".to_string());
        manifest.update("other/path/003.md".to_string(), "h3".to_string());

        let adr_paths = vec![PathBuf::from("docs/adrs")];
        let discovered = vec![PathBuf::from("docs/adrs/001.md")];

        let orphans = manifest.find_orphans(&adr_paths, &discovered);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0], "docs/adrs/002.md");

        manifest.remove(&orphans[0]);
        assert!(!manifest.contains_file("docs/adrs/002.md"));
        assert!(manifest.contains_file("docs/adrs/001.md"));
    }

    #[test]
    fn test_corrupted_manifest_recovery_fallback() {
        let temp_dir = std::env::temp_dir().join(format!("intent_corrupt_manifest_{}", std::process::id()));
        let manifest_path = temp_dir.join(".manifest.json");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Write corrupt JSON (e.g. truncated file or git conflict markers)
        std::fs::write(&manifest_path, "<<<<<<< HEAD\n{\"file.md\": \"hash\"\n=======").unwrap();

        let recovered = Manifest::load(&manifest_path);
        assert!(!recovered.contains_file("file.md"), "Corrupted manifest must fall back to empty state without panic");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
