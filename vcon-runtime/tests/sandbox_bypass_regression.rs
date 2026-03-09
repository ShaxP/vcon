use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_temp_cartridge(entrypoint_source: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();

    let root = std::env::temp_dir().join(format!("vcon-sandbox-regression-{stamp}"));
    let src = root.join("src");
    let assets = root.join("assets");
    fs::create_dir_all(&src).expect("temp src dir should be created");
    fs::create_dir_all(&assets).expect("temp assets dir should be created");

    fs::write(
        root.join("vcon.toml"),
        r#"id = "com.vcon.sandbox_regression"
name = "Sandbox Regression"
version = "0.1.0"
entrypoint = "src/main.py"
sdk_version = "1"
assets_path = "assets"
save_quota_mb = 8
permissions = []
"#,
    )
    .expect("manifest should be written");

    fs::write(src.join("main.py"), entrypoint_source).expect("entrypoint should be written");
    root
}

fn run_runtime(cartridge: &Path) -> std::process::Output {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let sdk_root = workspace.join("vcon-sdk");
    let saves_root = std::env::temp_dir().join("vcon-runtime-sandbox-regression-saves");

    Command::new(env!("CARGO_BIN_EXE_vcon-runtime"))
        .arg("--cartridge")
        .arg(cartridge)
        .arg("--saves-root")
        .arg(&saves_root)
        .arg("--sdk-root")
        .arg(&sdk_root)
        .arg("--frames")
        .arg("1")
        .output()
        .expect("runtime should execute")
}

#[test]
fn rejects_dynamic_import_bypass_patterns_at_boot() {
    let root = write_temp_cartridge(
        r#"
import vcon
import importlib
module = importlib.import_module("socket")
"#,
    );

    let output = run_runtime(&root);
    assert!(!output.status.success(), "runtime must reject cartridge");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("dynamic import pattern"));
    assert!(stderr.contains("importlib.import_module"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_obfuscated_runtime_escape_attempts() {
    let root = write_temp_cartridge(
        r#"
import vcon
imp = __builtins__["__im" + "port__"] if isinstance(__builtins__, dict) else getattr(__builtins__, "__im" + "port__")
imp("socket")
"#,
    );

    let output = run_runtime(&root);
    assert!(
        !output.status.success(),
        "runtime must reject escape attempt"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("blocked network module") || stderr.contains("outside SDK-facing APIs"),
        "unexpected stderr: {stderr}"
    );
    assert!(stderr.contains("socket"), "unexpected stderr: {stderr}");

    let _ = fs::remove_dir_all(root);
}
