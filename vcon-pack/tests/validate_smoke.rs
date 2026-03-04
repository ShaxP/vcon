use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("vcon-pack-{label}-{stamp}"))
}

fn write_valid_manifest(root: &Path) {
    std::fs::write(
        root.join("vcon.toml"),
        r#"id = "com.vcon.test"
name = "Test"
version = "0.1.0"
entrypoint = "src/main.py"
sdk_version = "1"
assets_path = "assets"
save_quota_mb = 8
permissions = ["storage"]
"#,
    )
    .expect("manifest write should succeed");
}

#[test]
fn validate_accepts_sample_cartridge() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/sample-game");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-pack"))
        .arg("validate")
        .arg("--cartridge")
        .arg(&cartridge)
        .output()
        .expect("vcon-pack should execute");

    assert!(output.status.success(), "validate must succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Manifest valid for cartridge com.vcon.sample_game"));
}

#[test]
fn validate_rejects_unsupported_sdk_version() {
    let root = unique_temp_dir("bad-sdk");
    let src = root.join("src");
    let assets = root.join("assets");
    std::fs::create_dir_all(&src).expect("src should be created");
    std::fs::create_dir_all(&assets).expect("assets should be created");

    std::fs::write(
        root.join("vcon.toml"),
        r#"id = "com.vcon.bad_sdk"
name = "Bad SDK"
version = "0.1.0"
entrypoint = "src/main.py"
sdk_version = "2"
assets_path = "assets"
save_quota_mb = 8
permissions = ["storage"]
"#,
    )
    .expect("manifest write should succeed");
    std::fs::write(src.join("main.py"), "import vcon\n").expect("entrypoint write should succeed");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-pack"))
        .arg("validate")
        .arg("--cartridge")
        .arg(&root)
        .output()
        .expect("vcon-pack should execute");

    assert!(!output.status.success(), "validate must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("sdk_version"), "unexpected stderr: {stderr}");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn validate_rejects_disallowed_dependency_file() {
    let root = unique_temp_dir("dependency");
    let src = root.join("src");
    let assets = root.join("assets");
    std::fs::create_dir_all(&src).expect("src should be created");
    std::fs::create_dir_all(&assets).expect("assets should be created");

    write_valid_manifest(&root);
    std::fs::write(src.join("main.py"), "import vcon\n").expect("entrypoint write should succeed");
    std::fs::write(root.join("pyproject.toml"), "[project]\nname='bad'\n")
        .expect("dependency file should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-pack"))
        .arg("validate")
        .arg("--cartridge")
        .arg(&root)
        .output()
        .expect("vcon-pack should execute");

    assert!(!output.status.success(), "validate must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("disallowed dependency manifest"),
        "unexpected stderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn validate_reports_manifest_line_context() {
    let root = unique_temp_dir("manifest-context");
    let src = root.join("src");
    let assets = root.join("assets");
    std::fs::create_dir_all(&src).expect("src should be created");
    std::fs::create_dir_all(&assets).expect("assets should be created");

    std::fs::write(
        root.join("vcon.toml"),
        r#"id = "com.vcon.bad"
name = "Broken
version = "0.1.0"
entrypoint = "src/main.py"
sdk_version = "1"
assets_path = "assets"
save_quota_mb = 8
permissions = ["storage"]
"#,
    )
    .expect("manifest write should succeed");
    std::fs::write(src.join("main.py"), "import vcon\n").expect("entrypoint write should succeed");

    let output = Command::new(env!("CARGO_BIN_EXE_vcon-pack"))
        .arg("validate")
        .arg("--cartridge")
        .arg(&root)
        .output()
        .expect("vcon-pack should execute");

    assert!(!output.status.success(), "validate must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("vcon.toml"), "unexpected stderr: {stderr}");
    assert!(stderr.contains("line"), "unexpected stderr: {stderr}");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn build_creates_bundle_and_validate_accepts_it() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/sample-game");
    let out_dir = unique_temp_dir("build-validate");
    let bundle_path = out_dir.join("sample.vcon");

    std::fs::create_dir_all(&out_dir).expect("out dir should exist");

    let build = Command::new(env!("CARGO_BIN_EXE_vcon-pack"))
        .arg("build")
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--output")
        .arg(&bundle_path)
        .output()
        .expect("build should execute");

    assert!(build.status.success(), "build must succeed");
    assert!(bundle_path.is_file(), "bundle should be created");

    let validate = Command::new(env!("CARGO_BIN_EXE_vcon-pack"))
        .arg("validate")
        .arg("--cartridge")
        .arg(&bundle_path)
        .output()
        .expect("validate should execute");

    assert!(validate.status.success(), "bundle validate must succeed");
    let stdout = String::from_utf8_lossy(&validate.stdout);
    assert!(stdout.contains("Bundle valid for cartridge com.vcon.sample_game"));

    let _ = std::fs::remove_dir_all(out_dir);
}

#[test]
fn build_output_is_deterministic() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cartridge = workspace.join("cartridges/sample-game");
    let out_dir = unique_temp_dir("deterministic");
    let one = out_dir.join("one.vcon");
    let two = out_dir.join("two.vcon");

    std::fs::create_dir_all(&out_dir).expect("out dir should exist");

    let out1 = Command::new(env!("CARGO_BIN_EXE_vcon-pack"))
        .arg("build")
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--output")
        .arg(&one)
        .output()
        .expect("first build should execute");
    assert!(out1.status.success(), "first build must succeed");

    let out2 = Command::new(env!("CARGO_BIN_EXE_vcon-pack"))
        .arg("build")
        .arg("--cartridge")
        .arg(&cartridge)
        .arg("--output")
        .arg(&two)
        .output()
        .expect("second build should execute");
    assert!(out2.status.success(), "second build must succeed");

    let first = std::fs::read(&one).expect("first bundle should be readable");
    let second = std::fs::read(&two).expect("second bundle should be readable");
    assert_eq!(first, second, "bundle bytes should be deterministic");

    let _ = std::fs::remove_dir_all(out_dir);
}
