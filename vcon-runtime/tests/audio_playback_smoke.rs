use std::path::Path;
use std::process::Command;

#[test]
fn runtime_initializes_audio_backend_and_processes_playback_commands() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/audio-smoke");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-audio-saves");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--saves-root")
        .arg(&saves_root)
        .arg("--sdk-root")
        .arg(&sdk_root)
        .arg("--frames")
        .arg("36")
        .output()
        .expect("runtime should execute");

    assert!(output.status.success(), "runtime must succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Loaded cartridge: Audio Smoke"));
    assert!(stdout.contains("Loop callbacks invoked: on_update=36 on_render=36"));
    assert!(stdout.contains("Audio backend: simulated-device"));
    assert!(stdout.contains("underruns="));
    assert!(stdout.contains("overruns="));
    assert!(stdout.contains("dropped_buffers="));
}
