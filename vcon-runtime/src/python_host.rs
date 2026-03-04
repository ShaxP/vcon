use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{anyhow, Context, Result};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use vcon_engine::{
    scripted_input_frame, AssetStore, DrawCommand, FrameCommandBuffer, InputFrame, RenderStats,
    SoftwareFrame,
};

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
    pub draw_commands_submitted: u32,
    pub draw_commands_rendered: u32,
    pub draw_commands_unsupported: u32,
    pub on_shutdown_called: bool,
}

pub trait InputProvider {
    fn next_frame(&mut self, frame_idx: u32) -> InputFrame;
}

pub struct NoneInputProvider;

impl InputProvider for NoneInputProvider {
    fn next_frame(&mut self, _frame_idx: u32) -> InputFrame {
        InputFrame::default()
    }
}

pub struct ScriptedInputProvider;

impl InputProvider for ScriptedInputProvider {
    fn next_frame(&mut self, frame_idx: u32) -> InputFrame {
        scripted_input_frame(frame_idx)
    }
}

pub fn run_cartridge(
    entrypoint_path: &Path,
    cartridge_root: &Path,
    sdk_root: &Path,
    frames: u32,
    dt_fixed: f64,
    width: u32,
    height: u32,
    input_provider: &mut dyn InputProvider,
    save_root: &Path,
    save_quota_mb: u32,
    asset_dir: Option<&Path>,
    dump_frame_path: Option<&Path>,
) -> Result<RuntimeInvocationReport> {
    let source = fs::read_to_string(entrypoint_path).with_context(|| {
        format!(
            "failed to read python entrypoint at {}",
            entrypoint_path.display()
        )
    })?;

    let assets = if let Some(dir) = asset_dir {
        Some(AssetStore::load_from_dir(dir)?)
    } else {
        None
    };

    Python::with_gil(|py| {
        extend_sys_path(py, cartridge_root, sdk_root)?;
        install_runtime_guards(py)?;
        configure_save_api(py, save_root, save_quota_mb)?;

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
        let mut draw_commands_submitted = 0;
        let mut draw_commands_rendered = 0;
        let mut draw_commands_unsupported = 0;

        let mut surface = SoftwareFrame::new(width, height);

        for frame_idx in 0..frames {
            let input_frame = input_provider.next_frame(frame_idx);
            inject_input_state(py, &input_frame)?;

            if call_if_present1_f64(&module, "on_update", dt_fixed)? {
                on_update_calls += 1;
            }

            begin_render_frame(py)?;
            if call_if_present1_f64(&module, "on_render", 1.0)? {
                on_render_calls += 1;
            }
            let frame_commands = drain_and_validate_render_commands(py)?;
            draw_commands_submitted += frame_commands.commands.len() as u32;

            let frame_stats: RenderStats = surface.apply_with_assets(&frame_commands, assets.as_ref());
            draw_commands_rendered += frame_stats.commands_executed as u32;
            draw_commands_unsupported += frame_stats.commands_unsupported as u32;
        }

        if let Some(path) = dump_frame_path {
            surface.write_ppm(path)?;
        }

        let on_shutdown_called = call_if_present0(&module, "on_shutdown")?;

        Ok(RuntimeInvocationReport {
            on_boot_called,
            on_update_calls,
            on_render_calls,
            draw_commands_submitted,
            draw_commands_rendered,
            draw_commands_unsupported,
            on_shutdown_called,
        })
    })
}

fn configure_save_api(py: Python<'_>, save_root: &Path, quota_mb: u32) -> Result<()> {
    let save_mod = py
        .import_bound("vcon.save")
        .context("failed to import vcon.save")?;
    save_mod
        .getattr("_set_runtime_state")
        .context("vcon.save._set_runtime_state not found")?
        .call1((save_root.to_string_lossy().to_string(), quota_mb))
        .context("vcon.save._set_runtime_state() failed")?;
    Ok(())
}

fn inject_input_state(py: Python<'_>, frame: &InputFrame) -> Result<()> {
    let input_mod = py
        .import_bound("vcon.input")
        .context("failed to import vcon.input")?;

    let axes = PyDict::new_bound(py);
    for (name, value) in frame.axes() {
        axes.set_item(name, value)
            .with_context(|| format!("failed setting input axis `{name}`"))?;
    }

    let actions = PyDict::new_bound(py);
    for name in frame.actions() {
        actions
            .set_item(name, true)
            .with_context(|| format!("failed setting input action `{name}`"))?;
    }

    input_mod
        .getattr("_set_runtime_state")
        .context("vcon.input._set_runtime_state not found")?
        .call1((axes, actions))
        .context("vcon.input._set_runtime_state() failed")?;

    Ok(())
}

fn begin_render_frame(py: Python<'_>) -> Result<()> {
    let graphics = py
        .import_bound("vcon.graphics")
        .context("failed to import vcon.graphics")?;
    graphics
        .getattr("begin_frame")
        .context("vcon.graphics.begin_frame not found")?
        .call0()
        .context("vcon.graphics.begin_frame() failed")?;
    Ok(())
}

fn drain_and_validate_render_commands(py: Python<'_>) -> Result<FrameCommandBuffer> {
    let graphics = py
        .import_bound("vcon.graphics")
        .context("failed to import vcon.graphics")?;
    let drained = graphics
        .getattr("drain_commands")
        .context("vcon.graphics.drain_commands not found")?
        .call0()
        .context("vcon.graphics.drain_commands() failed")?;

    let list = drained
        .downcast_into::<PyList>()
        .map_err(|_| anyhow!("vcon.graphics.drain_commands() must return list"))?;

    let mut frame = FrameCommandBuffer::default();
    for item in list.iter() {
        let command = parse_draw_command(&item)?;
        frame.push(command)?;
    }

    Ok(frame)
}

fn parse_draw_command(item: &Bound<'_, PyAny>) -> Result<DrawCommand> {
    let dict = item
        .downcast::<PyDict>()
        .map_err(|_| anyhow!("draw command item must be dict"))?;
    let kind = extract_str(dict, "kind")?;

    match kind.as_str() {
        "clear" => Ok(DrawCommand::Clear {
            color: extract_color(dict, "color")?,
        }),
        "line" => Ok(DrawCommand::Line {
            x1: extract_f64(dict, "x1")?,
            y1: extract_f64(dict, "y1")?,
            x2: extract_f64(dict, "x2")?,
            y2: extract_f64(dict, "y2")?,
            color: extract_color(dict, "color")?,
            thickness: extract_f64(dict, "thickness")?,
        }),
        "rect" => Ok(DrawCommand::Rect {
            x: extract_f64(dict, "x")?,
            y: extract_f64(dict, "y")?,
            w: extract_f64(dict, "w")?,
            h: extract_f64(dict, "h")?,
            color: extract_color(dict, "color")?,
            filled: extract_bool(dict, "filled")?,
            thickness: extract_f64(dict, "thickness")?,
        }),
        "circle" => Ok(DrawCommand::Circle {
            x: extract_f64(dict, "x")?,
            y: extract_f64(dict, "y")?,
            r: extract_f64(dict, "r")?,
            color: extract_color(dict, "color")?,
            filled: extract_bool(dict, "filled")?,
            thickness: extract_f64(dict, "thickness")?,
        }),
        "sprite" => Ok(DrawCommand::Sprite {
            asset_id: extract_str(dict, "asset_id")?,
            x: extract_f64(dict, "x")?,
            y: extract_f64(dict, "y")?,
            rotation: extract_f64(dict, "rotation")?,
            scale: extract_f64(dict, "scale")?,
            color: extract_color(dict, "color")?,
        }),
        "text" => Ok(DrawCommand::Text {
            value: extract_str(dict, "value")?,
            x: extract_f64(dict, "x")?,
            y: extract_f64(dict, "y")?,
            size: extract_f64(dict, "size")?,
            color: extract_color(dict, "color")?,
        }),
        _ => Err(anyhow!("unknown draw command kind `{kind}`")),
    }
}

fn extract_str(dict: &Bound<'_, PyDict>, key: &str) -> Result<String> {
    dict.get_item(key)
        .context("dict lookup failed")?
        .ok_or_else(|| anyhow!("missing draw command key `{key}`"))?
        .extract::<String>()
        .map_err(|_| anyhow!("draw command key `{key}` must be string"))
}

fn extract_f64(dict: &Bound<'_, PyDict>, key: &str) -> Result<f64> {
    dict.get_item(key)
        .context("dict lookup failed")?
        .ok_or_else(|| anyhow!("missing draw command key `{key}`"))?
        .extract::<f64>()
        .map_err(|_| anyhow!("draw command key `{key}` must be number"))
}

fn extract_bool(dict: &Bound<'_, PyDict>, key: &str) -> Result<bool> {
    dict.get_item(key)
        .context("dict lookup failed")?
        .ok_or_else(|| anyhow!("missing draw command key `{key}`"))?
        .extract::<bool>()
        .map_err(|_| anyhow!("draw command key `{key}` must be bool"))
}

fn extract_color(dict: &Bound<'_, PyDict>, key: &str) -> Result<[u8; 4]> {
    let value = dict
        .get_item(key)
        .context("dict lookup failed")?
        .ok_or_else(|| anyhow!("missing draw command key `{key}`"))?;
    let tuple = value
        .extract::<(u8, u8, u8, u8)>()
        .map_err(|_| anyhow!("draw command key `{key}` must be RGBA tuple"))?;
    Ok([tuple.0, tuple.1, tuple.2, tuple.3])
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
        .map_err(|_| anyhow!("sys.path is not a list"))?;

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

    use super::{run_cartridge, ScriptedInputProvider};

    #[test]
    fn invokes_sample_lifecycle_callbacks_loop_and_draw_commands() {
        let entrypoint = Path::new("../cartridges/sample-game/src/main.py");
        let cartridge_root = Path::new("../cartridges/sample-game");
        let sdk_root = Path::new("../vcon-sdk");
        let asset_dir = cartridge_root.join("assets");
        let save_root = std::env::temp_dir().join("vcon-runtime-save-test-sample");
        let _ = fs::remove_dir_all(&save_root);
        let mut provider = ScriptedInputProvider;

        let report = run_cartridge(
            entrypoint,
            cartridge_root,
            sdk_root,
            4,
            1.0 / 60.0,
            1280,
            800,
            &mut provider,
            &save_root,
            8,
            Some(&asset_dir),
            None,
        )
        .expect("callbacks should execute");

        assert!(report.on_boot_called);
        assert!(report.on_shutdown_called);
        assert_eq!(report.on_update_calls, 4);
        assert_eq!(report.on_render_calls, 4);
        assert_eq!(report.draw_commands_submitted, 24);
        assert_eq!(report.draw_commands_rendered, 24);
        assert_eq!(report.draw_commands_unsupported, 0);
        let _ = fs::remove_dir_all(&save_root);
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
        let save_root = std::env::temp_dir().join("vcon-runtime-save-test-net");
        let _ = fs::remove_dir_all(&save_root);
        let mut provider = ScriptedInputProvider;

        let result = run_cartridge(
            &entrypoint,
            &root,
            Path::new("../vcon-sdk"),
            1,
            1.0 / 60.0,
            1280,
            800,
            &mut provider,
            &save_root,
            8,
            None,
            None,
        );
        let err = result.expect_err("network import should be blocked");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("blocked network module") && msg.contains("socket"),
            "unexpected error: {msg}"
        );

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&save_root);
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
        let save_root = std::env::temp_dir().join("vcon-runtime-save-test-nonsdk");
        let _ = fs::remove_dir_all(&save_root);
        let mut provider = ScriptedInputProvider;

        let result = run_cartridge(
            &entrypoint,
            &root,
            Path::new("../vcon-sdk"),
            1,
            1.0 / 60.0,
            1280,
            800,
            &mut provider,
            &save_root,
            8,
            None,
            None,
        );
        let err = result.expect_err("non-sdk import should be blocked");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("outside SDK-facing APIs") && msg.contains("random"),
            "unexpected error: {msg}"
        );

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&save_root);
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
