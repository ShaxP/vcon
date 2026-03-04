use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn validate_accepts_sample_cartridge() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/sample-game");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-pack"))
        .arg("validate")
        .arg("--cartridge")
        .arg(&cartridge)
        .output()
        .expect("vcon-pack should execute");

    assert!(output.status.success(), "validate must succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Manifest valid for cartridge com.vcon.sample_game"));
}

#[test]
fn validate_rejects_unsupported_sdk_version() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vcon-pack-sdk-test-{stamp}"));
    let src = root.join("src");
    std::fs::create_dir_all(&src).expect("src should be created");

    std::fs::write(
        root.join("vcon.toml"),
        r#"id = "com.vcon.bad_sdk"
name = "Bad SDK"
version = "0.1.0"
entrypoint = "src/main.py"
sdk_version = "2"
assets_path = "assets"
save_quota_mb = 8
permissions = ["storage"]
"#,
    )
    .expect("manifest write should succeed");
    std::fs::write(src.join("main.py"), "import vcon\n").expect("entrypoint write should succeed");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-pack"))
        .arg("validate")
        .arg("--cartridge")
        .arg(&root)
        .output()
        .expect("vcon-pack should execute");

    assert!(!output.status.success(), "validate must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("sdk_version"), "unexpected stderr: {stderr}");

    let _ = std::fs::remove_dir_all(root);
}
