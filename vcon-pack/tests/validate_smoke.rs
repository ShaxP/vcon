use std::path::Path;
use std::process::Command;

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
