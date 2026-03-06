use std::path::Path;
use std::process::Command;

fn run_diag(input_source: &str) -> String {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/input-diagnostics");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-diag-saves");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--saves-root")
        .arg(&saves_root)
        .arg("--sdk-root")
        .arg(&sdk_root)
        .arg("--input-source")
        .arg(input_source)
        .arg("--frames")
        .arg("1")
        .output()
        .expect("runtime should execute diagnostics cartridge");

    assert!(
        output.status.success(),
        "runtime diagnostics run should succeed (status: {:?}, stderr: {})",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn diagnostics_renders_scripted_input_state() {
    let stdout = run_diag("scripted");

    assert!(stdout.contains("Loaded cartridge: Input Diagnostics"));
    assert!(stdout.contains("Loop callbacks invoked: on_update=1 on_render=1"));
    assert!(stdout.contains("Draw commands submitted: 22"));
    assert!(stdout.contains("Draw commands rendered: 22 (unsupported: 0)"));
}

#[test]
fn diagnostics_renders_neutral_when_input_is_none() {
    let stdout = run_diag("none");

    assert!(stdout.contains("Loaded cartridge: Input Diagnostics"));
    assert!(stdout.contains("Loop callbacks invoked: on_update=1 on_render=1"));
    assert!(stdout.contains("Draw commands submitted: 21"));
    assert!(stdout.contains("Draw commands rendered: 21 (unsupported: 0)"));
}

#[test]
fn diagnostics_gamepad_source_runs() {
    let stdout = run_diag("gamepad");
    assert!(stdout.contains("Loaded cartridge: Input Diagnostics"));
    assert!(stdout.contains("Loop callbacks invoked: on_update=1 on_render=1"));
}
