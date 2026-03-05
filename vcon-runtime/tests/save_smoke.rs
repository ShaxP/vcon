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

    let slot = saves_root.join("com.vcon.save_smoke").join("state.json");
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

#[test]
fn save_corruption_is_recovered_by_quarantine_and_rewrite() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/save-recovery");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-save-recovery");
    let _ = std::fs::remove_dir_all(&saves_root);

    let run = || {
        Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
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
            .expect("runtime should execute save-recovery cartridge")
    };

    let first = run();
    assert!(
        first.status.success(),
        "first recovery run should succeed: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let slot_dir = saves_root.join("com.vcon.save_recovery");
    let slot = slot_dir.join("state.json");
    std::fs::write(&slot, b"{invalid-json").expect("should corrupt slot");

    let second = run();
    assert!(
        second.status.success(),
        "second recovery run should succeed: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    let text = std::fs::read_to_string(&slot).expect("recovered slot should exist");
    let value: serde_json::Value = serde_json::from_str(&text).expect("slot should be valid json");
    assert_eq!(value["counter"], serde_json::json!(1));

    let quarantined = std::fs::read_dir(&slot_dir)
        .expect("slot dir should exist")
        .filter_map(Result::ok)
        .any(|entry| {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            name.starts_with("state.corrupt.") && name.ends_with(".json")
        });
    assert!(quarantined, "corrupt slot should be quarantined");
}
