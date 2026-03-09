use std::path::Path;
use std::process::Command;

fn run_and_dump(backend: &str, dump_file: &str) -> (String, Vec<u8>) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/sample-game");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join(format!("vcon-runtime-backend-parity-{backend}"));
    let path = std::env::temp_dir().join(dump_file);
    let _ = std::fs::remove_file(&path);

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
        .arg("6")
        .arg("--width")
        .arg("320")
        .arg("--height")
        .arg("200")
        .arg("--render-backend")
        .arg(backend)
        .arg("--dump-frame")
        .arg(&path)
        .output()
        .expect("runtime should execute for backend parity test");

    assert!(
        output.status.success(),
        "backend parity run should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let image = std::fs::read(&path).expect("parity dump should exist");
    (stdout, image)
}

#[test]
fn software_and_wgpu_dump_frames_match() {
    let (_software_stdout, software_dump) = run_and_dump("software", "vcon-parity-software.ppm");
    let (wgpu_stdout, wgpu_dump) = run_and_dump("wgpu", "vcon-parity-wgpu.ppm");

    if !wgpu_stdout.contains("Render backend: requested=Wgpu active=wgpu") {
        eprintln!("Skipping backend parity check: wgpu backend unavailable in environment");
        return;
    }

    assert_eq!(
        software_dump, wgpu_dump,
        "software and wgpu backend dumps diverged"
    );
}
