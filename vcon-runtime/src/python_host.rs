use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result};
use pyo3::prelude::*;
use pyo3::types::PyList;

static NEXT_MODULE_ID: AtomicUsize = AtomicUsize::new(0);

const SANDBOX_GUARD: &str = r#"
import builtins

_ALLOWED_ROOTS = {"vcon"}
_BLOCKED_NETWORK_ROOTS = {"socket", "urllib", "http", "requests", "asyncio"}
if hasattr(builtins, "__vcon_original_import__"):
    _real_import = builtins.__vcon_original_import__
else:
    builtins.__vcon_original_import__ = builtins.__import__
    _real_import = builtins.__vcon_original_import__


def _vcon_import(name, globals=None, locals=None, fromlist=(), level=0):
    root = name.split(".", 1)[0] if name else ""
    importer = ""
    if globals and "__name__" in globals:
        importer = globals["__name__"] or ""

    # Restrict only cartridge-authored imports; interpreter/SDK internals pass through.
    if not importer.startswith("cartridge_entry"):
        return _real_import(name, globals, locals, fromlist, level)

    if root in _BLOCKED_NETWORK_ROOTS:
        raise ImportError(f"vcon sandbox: blocked network module '{root}'")

    if level == 0 and root not in _ALLOWED_ROOTS:
        raise ImportError(
            f"vcon sandbox: import '{root}' is outside SDK-facing APIs"
        )

    return _real_import(name, globals, locals, fromlist, level)


builtins.__import__ = _vcon_import
"#;

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeInvocationReport {
    pub on_boot_called: bool,
    pub on_update_calls: u32,
    pub on_render_calls: u32,
    pub on_shutdown_called: bool,
}

pub fn run_cartridge(
    entrypoint_path: &Path,
    cartridge_root: &Path,
    sdk_root: &Path,
    frames: u32,
    dt_fixed: f64,
) -> Result<RuntimeInvocationReport> {
    let source = fs::read_to_string(entrypoint_path).with_context(|| {
        format!(
            "failed to read python entrypoint at {}",
            entrypoint_path.display()
        )
    })?;

    Python::with_gil(|py| {
        extend_sys_path(py, cartridge_root, sdk_root)?;
        install_runtime_guards(py)?;

        let module_name = format!(
            "cartridge_entry_{}",
            NEXT_MODULE_ID.fetch_add(1, Ordering::Relaxed)
        );
        let module = PyModule::from_code_bound(
            py,
            &source,
            &entrypoint_path.to_string_lossy(),
            &module_name,
        )
        .context("failed to compile cartridge entrypoint")?;

        let on_boot_called = call_if_present0(&module, "on_boot")?;

        let mut on_update_calls = 0;
        let mut on_render_calls = 0;

        for _ in 0..frames {
            if call_if_present1_f64(&module, "on_update", dt_fixed)? {
                on_update_calls += 1;
            }
            if call_if_present1_f64(&module, "on_render", 1.0)? {
                on_render_calls += 1;
            }
        }

        let on_shutdown_called = call_if_present0(&module, "on_shutdown")?;

        Ok(RuntimeInvocationReport {
            on_boot_called,
            on_update_calls,
            on_render_calls,
            on_shutdown_called,
        })
    })
}

fn call_if_present0(module: &Bound<'_, PyModule>, callback: &str) -> Result<bool> {
    if let Ok(function) = module.getattr(callback) {
        if function.is_callable() {
            function
                .call0()
                .with_context(|| format!("lifecycle callback `{callback}()` failed"))?;
            return Ok(true);
        }
    }

    Ok(false)
}

fn call_if_present1_f64(module: &Bound<'_, PyModule>, callback: &str, value: f64) -> Result<bool> {
    if let Ok(function) = module.getattr(callback) {
        if function.is_callable() {
            function
                .call1((value,))
                .with_context(|| format!("lifecycle callback `{callback}(...)` failed"))?;
            return Ok(true);
        }
    }

    Ok(false)
}

fn install_runtime_guards(py: Python<'_>) -> Result<()> {
    PyModule::from_code_bound(py, SANDBOX_GUARD, "_vcon_runtime_guard.py", "_vcon_runtime_guard")
        .context("failed to install runtime sandbox guard")?;
    Ok(())
}

fn extend_sys_path(py: Python<'_>, cartridge_root: &Path, sdk_root: &Path) -> Result<()> {
    let sys = py.import_bound("sys").context("failed to import sys")?;
    let sys_path = sys
        .getattr("path")
        .context("failed to access sys.path")?
        .downcast_into::<PyList>()
        .map_err(|_| anyhow::anyhow!("sys.path is not a list"))?;

    prepend_unique(&sys_path, &cartridge_root.to_string_lossy())?;
    prepend_unique(&sys_path, &sdk_root.to_string_lossy())?;

    Ok(())
}

fn prepend_unique(sys_path: &Bound<'_, PyList>, path: &str) -> Result<()> {
    let exists = sys_path
        .iter()
        .filter_map(|item| item.extract::<String>().ok())
        .any(|value| value == path);

    if !exists {
        sys_path
            .insert(0, path)
            .with_context(|| format!("failed to insert `{path}` into sys.path"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::run_cartridge;

    #[test]
    fn invokes_sample_lifecycle_callbacks_and_loop() {
        let entrypoint = Path::new("../cartridges/sample-game/src/main.py");
        let cartridge_root = Path::new("../cartridges/sample-game");
        let sdk_root = Path::new("../vcon-sdk");

        let report = run_cartridge(entrypoint, cartridge_root, sdk_root, 4, 1.0 / 60.0)
            .expect("callbacks should execute");

        assert!(report.on_boot_called);
        assert!(report.on_shutdown_called);
        assert_eq!(report.on_update_calls, 4);
        assert_eq!(report.on_render_calls, 4);
    }

    #[test]
    fn blocks_network_import_at_runtime() {
        let (root, entrypoint) = write_temp_entrypoint(
            r#"
import vcon
import socket


def on_boot():
    return None
"#,
        );

        let result = run_cartridge(&entrypoint, &root, Path::new("../vcon-sdk"), 1, 1.0 / 60.0);
        let err = result.expect_err("network import should be blocked");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("blocked network module") && msg.contains("socket"),
            "unexpected error: {msg}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn blocks_non_sdk_import_at_runtime() {
        let (root, entrypoint) = write_temp_entrypoint(
            r#"
import vcon
import random


def on_boot():
    return None
"#,
        );

        let result = run_cartridge(&entrypoint, &root, Path::new("../vcon-sdk"), 1, 1.0 / 60.0);
        let err = result.expect_err("non-sdk import should be blocked");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("outside SDK-facing APIs") && msg.contains("random"),
            "unexpected error: {msg}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    fn write_temp_entrypoint(source: &str) -> (PathBuf, PathBuf) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();

        let root = std::env::temp_dir().join(format!("vcon-runtime-test-{stamp}"));
        let src = root.join("src");
        fs::create_dir_all(&src).expect("temp src dir should be created");
        let entrypoint = src.join("main.py");
        fs::write(&entrypoint, source).expect("entrypoint should be written");

        (root, entrypoint)
    }
}
