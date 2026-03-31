use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub source: String,
    pub installed_at: String,
    pub binary: String,
    pub description: String,
    pub installed_by_pup: String,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let manifest: Manifest = serde_json::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(manifest)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> Manifest {
        Manifest {
            name: "hello".to_string(),
            version: "0.1.0".to_string(),
            source: "local:/tmp/pup-hello".to_string(),
            installed_at: "2026-03-29T00:00:00Z".to_string(),
            binary: "pup-hello".to_string(),
            description: "A hello world extension".to_string(),
            installed_by_pup: "0.39.0".to_string(),
        }
    }

    #[test]
    fn test_manifest_roundtrip() {
        let dir = std::env::temp_dir().join("pup-test-manifest-roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.json");

        let original = sample_manifest();
        original.save(&path).unwrap();

        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded.name, original.name);
        assert_eq!(loaded.version, original.version);
        assert_eq!(loaded.source, original.source);
        assert_eq!(loaded.binary, original.binary);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_manifest_load_missing() {
        let result = Manifest::load(Path::new("/nonexistent/manifest.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_manifest_load_invalid_json() {
        let dir = std::env::temp_dir().join("pup-test-manifest-invalid");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("manifest.json");
        std::fs::write(&path, "not json").unwrap();

        let result = Manifest::load(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
