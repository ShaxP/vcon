use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::{Manifest, ManifestError};
use crate::sandbox::{scan_entrypoint_source, validate_manifest_permissions};
use crate::storage::{SaveNamespace, StorageError};

#[derive(Debug, Clone)]
pub struct LifecycleAvailability {
    pub on_boot: bool,
    pub on_shutdown: bool,
}

#[derive(Debug, Clone)]
pub struct BootReport {
    pub manifest: Manifest,
    pub entrypoint_path: PathBuf,
    pub lifecycle: LifecycleAvailability,
    pub save_namespace: SaveNamespace,
}

pub fn boot_cartridge(cartridge_dir: &Path, saves_root: &Path) -> Result<BootReport, EngineError> {
    let manifest_path = cartridge_dir.join("vcon.toml");
    let manifest_source =
        fs::read_to_string(&manifest_path).map_err(|source| EngineError::ReadManifest {
            path: manifest_path.clone(),
            source,
        })?;
    let manifest = Manifest::parse(&manifest_source)?;

    let permission_violations = validate_manifest_permissions(&manifest);
    if !permission_violations.is_empty() {
        let messages = permission_violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(EngineError::Policy(messages));
    }

    let entrypoint_path = cartridge_dir.join(&manifest.entrypoint);
    let entrypoint_source =
        fs::read_to_string(&entrypoint_path).map_err(|source| EngineError::ReadEntrypoint {
            path: entrypoint_path.clone(),
            source,
        })?;

    let import_violations = scan_entrypoint_source(&entrypoint_source);
    if !import_violations.is_empty() {
        let messages = import_violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(EngineError::Policy(messages));
    }

    let lifecycle = LifecycleAvailability {
        on_boot: entrypoint_source.contains("def on_boot("),
        on_shutdown: entrypoint_source.contains("def on_shutdown("),
    };

    let save_namespace = SaveNamespace::from_manifest(saves_root, &manifest)?;

    Ok(BootReport {
        manifest,
        entrypoint_path,
        lifecycle,
        save_namespace,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("failed to read manifest at {path}: {source}")]
    ReadManifest { path: PathBuf, source: std::io::Error },
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),
    #[error("sandbox policy violation: {0}")]
    Policy(String),
    #[error("failed to read entrypoint at {path}: {source}")]
    ReadEntrypoint { path: PathBuf, source: std::io::Error },
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::boot_cartridge;

    #[test]
    fn boots_sample_cartridge() {
        let cartridge_dir = Path::new("../cartridges/sample-game");
        let saves_dir = Path::new("/tmp/vcon-test-saves");
        let report = boot_cartridge(cartridge_dir, saves_dir).expect("sample should boot");

        assert_eq!(report.manifest.id, "com.vcon.sample_game");
        assert!(report.lifecycle.on_boot);
        assert!(report.lifecycle.on_shutdown);
        assert_eq!(report.save_namespace.quota_mb, 16);
    }

    #[test]
    fn rejects_policy_violating_cartridge() {
        let root = Path::new("/tmp/vcon-policy-violation");
        let src_dir = root.join("src");
        let _ = fs::remove_dir_all(root);
        fs::create_dir_all(&src_dir).expect("create src dir");

        fs::write(
            root.join("vcon.toml"),
            r#"id = "com.vcon.bad"
name = "Bad"
version = "0.1.0"
entrypoint = "src/main.py"
sdk_version = "1"
assets_path = "assets"
save_quota_mb = 8
permissions = ["network"]
"#,
        )
        .expect("write manifest");
        fs::write(src_dir.join("main.py"), "import vcon\n").expect("write entrypoint");

        let err = boot_cartridge(root, Path::new("/tmp/vcon-test-saves"))
            .expect_err("policy violation should fail");
        assert!(err.to_string().contains("permission `network` is blocked"));

        let _ = fs::remove_dir_all(root);
    }
}
