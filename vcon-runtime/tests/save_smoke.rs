use std::path::Path;
use std::process::Command;

#[test]
fn save_smoke_persists_slot_data() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/save-smoke");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-save-smoke");
    let _ = std::fs::remove_dir_all(&saves_root);

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--saves-root")
        .arg(&saves_root)
        .arg("--sdk-root")
        .arg(&sdk_root)
        .arg("--input-source")
        .arg("none")
        .arg("--frames")
        .arg("3")
        .output()
        .expect("runtime should execute save-smoke cartridge");

    assert!(
        output.status.success(),
        "save-smoke run should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let slot = saves_root
        .join("com.vcon.save_smoke")
        .join("state.json");
    let text = std::fs::read_to_string(&slot).expect("saved slot should exist");
    let value: serde_json::Value = serde_json::from_str(&text).expect("slot should be json");
    assert_eq!(value["counter"], serde_json::json!(3));
}

#[test]
fn save_quota_violation_fails_run() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/save-quota");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-save-quota");
    let _ = std::fs::remove_dir_all(&saves_root);

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--saves-root")
        .arg(&saves_root)
        .arg("--sdk-root")
        .arg(&sdk_root)
        .arg("--input-source")
        .arg("none")
        .arg("--frames")
        .arg("1")
        .output()
        .expect("runtime should execute save-quota cartridge");

    assert!(!output.status.success(), "quota run must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("save quota exceeded"),
        "unexpected stderr: {stderr}"
    );
}
