use std::path::{Path, PathBuf};

use crate::Manifest;

#[derive(Debug, Clone)]
pub struct SaveNamespace {
    pub game_id: String,
    pub quota_mb: u32,
    pub root: PathBuf,
}

impl SaveNamespace {
    pub fn from_manifest(base_dir: &Path, manifest: &Manifest) -> Result<Self, StorageError> {
        let game_id = sanitize_game_id(&manifest.id)?;
        let root = base_dir.join(&game_id);

        Ok(Self {
            game_id,
            quota_mb: manifest.save_quota_mb,
            root,
        })
    }

    pub fn slot_path(&self, slot: &str) -> Result<PathBuf, StorageError> {
        if slot.trim().is_empty() {
            return Err(StorageError::InvalidSlot(
                "slot name must be non-empty".to_owned(),
            ));
        }
        if slot.contains('/') || slot.contains("..") {
            return Err(StorageError::InvalidSlot(
                "slot name must not contain path traversal components".to_owned(),
            ));
        }

        Ok(self.root.join(format!("{slot}.json")))
    }
}

fn sanitize_game_id(value: &str) -> Result<String, StorageError> {
    if value.is_empty() {
        return Err(StorageError::InvalidGameId(
            "game id must not be empty".to_owned(),
        ));
    }

    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(StorageError::InvalidGameId(
            "game id may only include [a-zA-Z0-9._-]".to_owned(),
        ));
    }

    Ok(value.to_owned())
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("invalid game id: {0}")]
    InvalidGameId(String),
    #[error("invalid slot: {0}")]
    InvalidSlot(String),
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::SaveNamespace;
    use crate::Manifest;

    fn manifest() -> Manifest {
        Manifest {
            id: "com.example.demo".to_owned(),
            name: "Demo".to_owned(),
            version: "0.1.0".to_owned(),
            entrypoint: "src/main.py".to_owned(),
            sdk_version: "1".to_owned(),
            assets_path: "assets".to_owned(),
            save_quota_mb: 8,
            permissions: vec!["storage".to_owned()],
        }
    }

    #[test]
    fn builds_namespaced_slot_path() {
        let namespace = SaveNamespace::from_manifest(Path::new("/tmp/vcon/saves"), &manifest())
            .expect("namespace should be created");
        let slot = namespace
            .slot_path("slot_a")
            .expect("slot path should be created");
        assert_eq!(slot, Path::new("/tmp/vcon/saves/com.example.demo/slot_a.json"));
    }

    #[test]
    fn rejects_slot_traversal() {
        let namespace = SaveNamespace::from_manifest(Path::new("/tmp/vcon/saves"), &manifest())
            .expect("namespace should be created");
        let err = namespace
            .slot_path("../bad")
            .expect_err("traversal should be blocked");
        assert!(err.to_string().contains("path traversal"));
    }
}
