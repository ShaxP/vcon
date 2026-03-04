use std::path::Path;
use std::process::Command;

fn run_and_dump(path: &std::path::Path) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/sample-game");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-determinism-saves");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--saves-root")
        .arg(&saves_root)
        .arg("--sdk-root")
        .arg(&sdk_root)
        .arg("--input-source")
        .arg("scripted")
        .arg("--frames")
        .arg("10")
        .arg("--dt-fixed")
        .arg("0.0166666667")
        .arg("--dump-frame")
        .arg(path)
        .output()
        .expect("runtime should execute");

    assert!(
        output.status.success(),
        "determinism run should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn deterministic_replay_produces_identical_frame_dump() {
    let path_a = std::env::temp_dir().join("vcon-determinism-a.ppm");
    let path_b = std::env::temp_dir().join("vcon-determinism-b.ppm");
    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);

    run_and_dump(&path_a);
    run_and_dump(&path_b);

    let a = std::fs::read(&path_a).expect("first dump should exist");
    let b = std::fs::read(&path_b).expect("second dump should exist");
    assert_eq!(a, b, "frame dumps must match for deterministic replay");
}
