use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[test]
fn steam_deck_profile_frame_pacing_smoke_budget() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/sample-game");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-perf-saves");

    let start = Instant::now();
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
        .arg("120")
        .arg("--width")
        .arg("1280")
        .arg("--height")
        .arg("800")
        .arg("--dt-fixed")
        .arg("0.0166666667")
        .output()
        .expect("runtime should execute performance smoke");
    let elapsed = start.elapsed();

    assert!(
        output.status.success(),
        "performance smoke run should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Soft budget to catch severe regressions without making CI timing-sensitive.
    assert!(
        elapsed.as_secs_f64() < 8.0,
        "runtime exceeded performance smoke budget: {:.3}s",
        elapsed.as_secs_f64()
    );
}
