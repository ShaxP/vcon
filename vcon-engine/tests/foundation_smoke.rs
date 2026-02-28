use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use vcon_engine::boot_cartridge;

#[test]
fn boots_sample_cartridge_from_workspace() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge_dir = workspace.join("cartridges/sample-game");
    let saves_root = std::env::temp_dir().join("vcon-int-saves");

    let report = boot_cartridge(&cartridge_dir, &saves_root).expect("sample cartridge should boot");

    assert_eq!(report.manifest.id, "com.vcon.sample_game");
    assert!(report.lifecycle.on_boot);
    assert!(report.lifecycle.on_shutdown);
}

#[test]
fn rejects_manifest_with_blocked_network_permission() {
    let root = make_temp_dir("vcon-engine-int-bad");
    let src = root.join("src");
    fs::create_dir_all(&src).expect("src should be created");

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
    .expect("manifest should be written");
    fs::write(src.join("main.py"), "import vcon\n").expect("entrypoint should be written");

    let err = boot_cartridge(&root, Path::new("/tmp/vcon-int-saves"))
        .expect_err("cartridge with network permission should fail");
    assert!(err.to_string().contains("permission `network` is blocked"));

    let _ = fs::remove_dir_all(root);
}

fn make_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("{prefix}-{stamp}"));
    fs::create_dir_all(&root).expect("temp dir should be created");
    root
}
