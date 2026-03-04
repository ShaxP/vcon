use std::path::Path;
use std::process::Command;

#[test]
fn runtime_executes_physics_demo_and_dispatches_collision_events() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/physics-demo");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-physics-saves");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--saves-root")
        .arg(&saves_root)
        .arg("--sdk-root")
        .arg(&sdk_root)
        .arg("--frames")
        .arg("30")
        .output()
        .expect("runtime should execute");

    assert!(output.status.success(), "runtime must succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Loaded cartridge: Physics Demo"));
    assert!(stdout.contains("Loop callbacks invoked: on_update=30 on_render=30"));
    assert!(stdout.contains("Event callbacks invoked: on_event="));
    assert!(stdout.contains("physics events:"));
}
