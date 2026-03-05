use std::path::Path;
use std::process::Command;

fn run_and_dump(path: &std::path::Path, dt_fixed: &str, frames: &str, input_seed: &str) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/sample-game");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join(format!(
        "vcon-runtime-determinism-saves-dt-{dt_fixed}-f-{frames}-seed-{input_seed}"
    ));

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
        .arg(frames)
        .arg("--dt-fixed")
        .arg(dt_fixed)
        .arg("--input-seed")
        .arg(input_seed)
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

    run_and_dump(&path_a, "0.0166666667", "10", "0");
    run_and_dump(&path_b, "0.0166666667", "10", "0");

    let a = std::fs::read(&path_a).expect("first dump should exist");
    let b = std::fs::read(&path_b).expect("second dump should exist");
    assert_eq!(a, b, "frame dumps must match for deterministic replay");
}

#[test]
fn deterministic_replay_holds_for_seeded_input_stream() {
    let path_a = std::env::temp_dir().join("vcon-determinism-seeded-a.ppm");
    let path_b = std::env::temp_dir().join("vcon-determinism-seeded-b.ppm");
    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);

    run_and_dump(&path_a, "0.0166666667", "30", "1337");
    run_and_dump(&path_b, "0.0166666667", "30", "1337");

    let a = std::fs::read(&path_a).expect("first seeded dump should exist");
    let b = std::fs::read(&path_b).expect("second seeded dump should exist");
    assert_eq!(a, b, "seeded replay must remain deterministic");
}

#[test]
fn deterministic_replay_holds_at_higher_tick_rate() {
    let path_a = std::env::temp_dir().join("vcon-determinism-hi-tick-a.ppm");
    let path_b = std::env::temp_dir().join("vcon-determinism-hi-tick-b.ppm");
    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);

    // Stress determinism with a different fixed step cadence.
    run_and_dump(&path_a, "0.0083333333", "60", "2026");
    run_and_dump(&path_b, "0.0083333333", "60", "2026");

    let a = std::fs::read(&path_a).expect("first hi-tick dump should exist");
    let b = std::fs::read(&path_b).expect("second hi-tick dump should exist");
    assert_eq!(a, b, "high tick replay must remain deterministic");
}
