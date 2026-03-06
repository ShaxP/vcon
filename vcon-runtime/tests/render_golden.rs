use std::path::Path;
use std::process::Command;

fn run_and_dump(
    cartridge_name: &str,
    input_source: &str,
    frames: &str,
    dump_file: &str,
) -> Vec<u8> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges").join(cartridge_name);
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root =
        std::env::temp_dir().join(format!("vcon-runtime-golden-saves-{cartridge_name}"));
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
        .arg(input_source)
        .arg("--frames")
        .arg(frames)
        .arg("--width")
        .arg("320")
        .arg("--height")
        .arg("200")
        .arg("--dump-frame")
        .arg(&path)
        .output()
        .expect("runtime should execute for golden test");

    assert!(
        output.status.success(),
        "golden render run should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::read(&path).expect("golden dump should exist")
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[test]
fn sample_game_render_matches_golden_hash() {
    let bytes = run_and_dump("sample-game", "scripted", "6", "vcon-golden-sample.ppm");
    let digest = fnv1a64(&bytes);

    // Golden hash for 320x200 dump after 6 frames with scripted input seed=0.
    const EXPECTED: u64 = 2897972321711479642;
    assert_eq!(digest, EXPECTED, "sample-game golden frame changed");
}

#[test]
fn input_diagnostics_render_matches_golden_hash() {
    let bytes = run_and_dump("input-diagnostics", "none", "1", "vcon-golden-diag.ppm");
    let digest = fnv1a64(&bytes);

    // Golden hash for 320x200 dump after one neutral diagnostics frame.
    const EXPECTED: u64 = 8658551470775578051;
    assert_eq!(digest, EXPECTED, "input diagnostics golden frame changed");
}
