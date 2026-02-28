use std::path::Path;
use std::process::Command;

#[test]
fn runtime_invokes_sample_lifecycle_callbacks() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/sample-game");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-int-saves");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--saves-root")
        .arg(&saves_root)
        .arg("--sdk-root")
        .arg(&sdk_root)
        .output()
        .expect("runtime should execute");

    assert!(output.status.success(), "runtime must succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Loaded cartridge: Sample Game"));
    assert!(stdout.contains("Invoked lifecycle callback: on_boot() [python]"));
    assert!(stdout.contains("Loop callbacks invoked: on_update=3 on_render=3"));
    assert!(stdout.contains("Invoked lifecycle callback: on_shutdown() [python]"));
}
