use std::path::Path;
use std::process::Command;

#[test]
fn windowed_wgpu_smoke_capability_gated() {
    if cfg!(target_os = "linux")
        && std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
    {
        eprintln!("Skipping windowed wgpu smoke: no DISPLAY/WAYLAND_DISPLAY in environment");
        return;
    }

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/sample-game");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-windowed-wgpu-saves");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--saves-root")
        .arg(&saves_root)
        .arg("--sdk-root")
        .arg(&sdk_root)
        .arg("--windowed")
        .arg("--windowed-max-frames")
        .arg("2")
        .arg("--render-backend")
        .arg("wgpu")
        .arg("--width")
        .arg("640")
        .arg("--height")
        .arg("400")
        .output()
        .expect("windowed wgpu smoke should execute");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Render backend: requested=Wgpu"),
            "missing requested backend marker: {stdout}"
        );
        return;
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if is_capability_missing(&stderr) {
        eprintln!("Skipping windowed wgpu smoke due to missing capability: {stderr}");
        return;
    }

    panic!("windowed wgpu smoke run failed: {stderr}");
}

fn is_capability_missing(stderr: &str) -> bool {
    let needles = [
        "failed to create window",
        "wgpu window init failed",
        "failed to create wgpu surface",
        "failed to acquire wgpu adapter",
        "failed to acquire wgpu adapter for window surface",
        "failed to create wgpu device",
        "No such file or directory (os error 2)",
        "XOpenDisplayFailed",
    ];

    needles.iter().any(|needle| stderr.contains(needle))
}
