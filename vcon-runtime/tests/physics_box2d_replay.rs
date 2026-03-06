use std::path::Path;
use std::process::Command;

fn run_physics_demo_dump(path: &std::path::Path, frames: &str) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/physics-demo");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join(format!("vcon-runtime-physics-box2d-saves-{frames}"));

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--saves-root")
        .arg(&saves_root)
        .arg("--sdk-root")
        .arg(&sdk_root)
        .arg("--frames")
        .arg(frames)
        .arg("--dump-frame")
        .arg(path)
        .env("VCON_PHYSICS_BACKEND", "box2d")
        .output()
        .expect("runtime should execute physics demo");

    assert!(
        output.status.success(),
        "physics replay run should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn box2d_physics_demo_replay_is_deterministic() {
    let path_a = std::env::temp_dir().join("vcon-physics-box2d-replay-a.ppm");
    let path_b = std::env::temp_dir().join("vcon-physics-box2d-replay-b.ppm");
    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);

    run_physics_demo_dump(&path_a, "90");
    run_physics_demo_dump(&path_b, "90");

    let a = std::fs::read(&path_a).expect("first physics replay dump should exist");
    let b = std::fs::read(&path_b).expect("second physics replay dump should exist");
    assert_eq!(a, b, "box2d physics replay must be deterministic");
}
