use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{anyhow, Context, Result};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use vcon_engine::{
    scripted_input_frame_seeded, ActiveVoice, AssetStore, AudioMixer, DrawCommand,
    FrameCommandBuffer, InputFrame, NodeId, PhysicsBackend, PhysicsBody2D, PhysicsBodyKind,
    PhysicsVec2, PhysicsWorld, RenderStats, SceneGraph,
};

use crate::audio_backend::{AudioBackendHealth, SimulatedAudioDevice};
use crate::render_backend::{ActiveRenderBackend, RenderExecutor};

static NEXT_MODULE_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeInvocationReport {
    pub on_boot_called: bool,
    pub on_update_calls: u32,
    pub on_render_calls: u32,
    pub on_event_calls: u32,
    pub physics_events_dispatched: u32,
    pub draw_commands_submitted: u32,
    pub draw_commands_rendered: u32,
    pub draw_commands_unsupported: u32,
    pub render_backend: ActiveRenderBackend,
    pub physics_backend: PhysicsBackend,
    pub audio_backend: String,
    pub audio_underruns: u64,
    pub audio_overruns: u64,
    pub audio_dropped_buffers: u64,
    pub on_shutdown_called: bool,
}

pub trait InputProvider {
    fn next_frame(&mut self, frame_idx: u32) -> InputFrame;
}

#[derive(Debug, Clone, Copy)]
pub enum FrameLoopMode {
    Fixed(u32),
    UntilStopped { max_frames: Option<u32> },
}

pub trait FrameObserver {
    fn on_frame(
        &mut self,
        frame_idx: u32,
        frame_rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<bool>;
}

pub struct NoneInputProvider;

impl InputProvider for NoneInputProvider {
    fn next_frame(&mut self, _frame_idx: u32) -> InputFrame {
        InputFrame::default()
    }
}

#[derive(Default)]
pub struct ScriptedInputProvider {
    seed: u64,
}

impl ScriptedInputProvider {
    pub fn with_seed(seed: u64) -> Self {
        Self { seed }
    }
}

impl InputProvider for ScriptedInputProvider {
    fn next_frame(&mut self, frame_idx: u32) -> InputFrame {
        scripted_input_frame_seeded(self.seed, frame_idx)
    }
}

#[derive(Debug, Clone)]
struct PhysicsBodySpec {
    name: String,
    x: f64,
    y: f64,
    velocity_x: f64,
    velocity_y: f64,
    radius: f64,
    dynamic: bool,
    restitution: f64,
}

#[derive(Debug, Clone)]
struct PhysicsSyncInput {
    gravity: PhysicsVec2,
    bodies: Vec<PhysicsBodySpec>,
}

#[derive(Debug)]
struct RuntimePhysics {
    scene: SceneGraph,
    world: PhysicsWorld,
    names_to_nodes: HashMap<String, NodeId>,
    nodes_to_names: HashMap<NodeId, String>,
}

impl Default for RuntimePhysics {
    fn default() -> Self {
        let backend = match std::env::var("VCON_PHYSICS_BACKEND") {
            Ok(value) if value.eq_ignore_ascii_case("legacy") => PhysicsBackend::Legacy,
            _ => PhysicsBackend::Box2d,
        };

        Self {
            scene: SceneGraph::default(),
            world: PhysicsWorld::with_backend(backend),
            names_to_nodes: HashMap::new(),
            nodes_to_names: HashMap::new(),
        }
    }
}

#[derive(Debug, Default)]
struct RuntimeAudio {
    mixer: AudioMixer,
    device: SimulatedAudioDevice,
}

#[derive(Debug, Clone)]
struct AudioPlayRequestSpec {
    clip_id: String,
    volume: f32,
    looped: bool,
}

#[derive(Debug, Default, Clone)]
struct AudioRuntimeCommands {
    play_requests: Vec<AudioPlayRequestSpec>,
    stop_voice_ids: Vec<u64>,
    stop_all: bool,
}

#[allow(clippy::too_many_arguments)]
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
    render_backend: ActiveRenderBackend,
) -> Result<RuntimeInvocationReport> {
    run_cartridge_with_loop(
        entrypoint_path,
        cartridge_root,
        sdk_root,
        FrameLoopMode::Fixed(frames),
        dt_fixed,
        width,
        height,
        input_provider,
        save_root,
        save_quota_mb,
        asset_dir,
        dump_frame_path,
        render_backend,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn run_cartridge_with_loop(
    entrypoint_path: &Path,
    cartridge_root: &Path,
    sdk_root: &Path,
    frame_loop_mode: FrameLoopMode,
    dt_fixed: f64,
    width: u32,
    height: u32,
    input_provider: &mut dyn InputProvider,
    save_root: &Path,
    save_quota_mb: u32,
    asset_dir: Option<&Path>,
    dump_frame_path: Option<&Path>,
    render_backend: ActiveRenderBackend,
    mut frame_observer: Option<&mut dyn FrameObserver>,
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
        install_runtime_guards(py, cartridge_root)?;
        configure_save_api(py, save_root, save_quota_mb)?;
        configure_physics_api(py)?;
        configure_audio_api(py)?;
        let mut executor = RenderExecutor::new(render_backend, width, height);
        configure_graphics_api(py, width, height, executor.backend())?;

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
        let mut on_event_calls = 0;
        let mut physics_events_dispatched = 0;
        let mut draw_commands_submitted = 0;
        let mut draw_commands_rendered = 0;
        let mut draw_commands_unsupported = 0;
        let mut physics = RuntimePhysics::default();
        let mut audio = RuntimeAudio::default();

        let mut frame_idx = 0_u32;
        loop {
            match frame_loop_mode {
                FrameLoopMode::Fixed(total_frames) if frame_idx >= total_frames => break,
                FrameLoopMode::UntilStopped {
                    max_frames: Some(max),
                } if frame_idx >= max => break,
                _ => {}
            }

            let input_frame = input_provider.next_frame(frame_idx);
            inject_input_state(py, &input_frame)?;

            if call_if_present1_f64(&module, "on_update", dt_fixed)? {
                on_update_calls += 1;
            }

            let physics_input = read_physics_sync_state(py)?;
            synchronize_physics(&mut physics, &physics_input)?;
            let collisions = step_physics(&mut physics, dt_fixed);
            publish_physics_runtime_state(py, &physics)?;
            physics_events_dispatched += collisions.len() as u32;
            for event in collisions {
                if call_if_present1_event(&module, "on_event", &event)? {
                    on_event_calls += 1;
                }
            }

            let audio_commands = read_audio_runtime_commands(py)?;
            apply_audio_runtime_commands(&mut audio, &audio_commands);
            let active_voices = audio.mixer.flush_queue().to_vec();
            audio.device.process_frame(dt_fixed, active_voices.len());
            publish_audio_runtime_state(py, &active_voices, &audio.device.health())?;

            begin_render_frame(py)?;
            if call_if_present1_f64(&module, "on_render", 1.0)? {
                on_render_calls += 1;
            }
            let frame_commands = drain_and_validate_render_commands(py)?;
            draw_commands_submitted += frame_commands.commands.len() as u32;

            let frame_stats: RenderStats = executor.render_frame(&frame_commands, assets.as_ref());
            draw_commands_rendered += frame_stats.commands_executed as u32;
            draw_commands_unsupported += frame_stats.commands_unsupported as u32;

            if let Some(observer) = frame_observer.as_deref_mut() {
                if !observer.on_frame(
                    frame_idx,
                    executor.pixels_rgba(),
                    executor.width(),
                    executor.height(),
                )? {
                    break;
                }
            }

            frame_idx = frame_idx.saturating_add(1);
        }

        if let Some(path) = dump_frame_path {
            executor.dump_ppm(path)?;
        }

        let on_shutdown_called = call_if_present0(&module, "on_shutdown")?;
        let audio_health = audio.device.health();

        Ok(RuntimeInvocationReport {
            on_boot_called,
            on_update_calls,
            on_render_calls,
            on_event_calls,
            physics_events_dispatched,
            draw_commands_submitted,
            draw_commands_rendered,
            draw_commands_unsupported,
            render_backend: executor.backend(),
            physics_backend: physics.world.backend(),
            audio_backend: SimulatedAudioDevice::BACKEND_NAME.to_owned(),
            audio_underruns: audio_health.underruns,
            audio_overruns: audio_health.overruns,
            audio_dropped_buffers: audio_health.dropped_buffers,
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

fn configure_physics_api(py: Python<'_>) -> Result<()> {
    let physics_mod = py
        .import_bound("vcon.physics")
        .context("failed to import vcon.physics")?;
    physics_mod
        .getattr("_set_runtime_state")
        .context("vcon.physics._set_runtime_state not found")?
        .call1(((0.0_f64, 0.0_f64), Vec::<&str>::new()))
        .context("vcon.physics._set_runtime_state() failed")?;
    Ok(())
}

fn configure_audio_api(py: Python<'_>) -> Result<()> {
    let audio_mod = py
        .import_bound("vcon.audio")
        .context("failed to import vcon.audio")?;
    let active = PyList::empty_bound(py);
    let health = PyDict::new_bound(py);
    health
        .set_item("initialized", true)
        .context("audio initialized health set failed")?;
    health
        .set_item("queued_buffers", 0_u32)
        .context("audio queued_buffers health set failed")?;
    health
        .set_item("underruns", 0_u64)
        .context("audio underruns health set failed")?;
    health
        .set_item("overruns", 0_u64)
        .context("audio overruns health set failed")?;
    health
        .set_item("dropped_buffers", 0_u64)
        .context("audio dropped_buffers health set failed")?;

    audio_mod
        .getattr("_set_runtime_state")
        .context("vcon.audio._set_runtime_state not found")?
        .call1((active, health))
        .context("vcon.audio._set_runtime_state() failed")?;
    Ok(())
}

fn configure_graphics_api(
    py: Python<'_>,
    width: u32,
    height: u32,
    render_backend: ActiveRenderBackend,
) -> Result<()> {
    let graphics_mod = py
        .import_bound("vcon.graphics")
        .context("failed to import vcon.graphics")?;
    graphics_mod
        .getattr("_set_runtime_state")
        .context("vcon.graphics._set_runtime_state not found")?
        .call1((width, height, render_backend.as_str()))
        .context("vcon.graphics._set_runtime_state() failed")?;
    Ok(())
}

fn read_physics_sync_state(py: Python<'_>) -> Result<PhysicsSyncInput> {
    let physics_mod = py
        .import_bound("vcon.physics")
        .context("failed to import vcon.physics")?;
    let exported = physics_mod
        .getattr("_export_runtime_state")
        .context("vcon.physics._export_runtime_state not found")?
        .call0()
        .context("vcon.physics._export_runtime_state() failed")?;
    let dict = exported
        .downcast_into::<PyDict>()
        .map_err(|_| anyhow!("vcon.physics._export_runtime_state() must return dict"))?;

    let gravity = dict
        .get_item("gravity")
        .context("gravity lookup failed")?
        .ok_or_else(|| anyhow!("physics state missing `gravity`"))?
        .extract::<(f64, f64)>()
        .map_err(|_| anyhow!("physics `gravity` must be (x, y) numbers"))?;

    let bodies = dict
        .get_item("bodies")
        .context("bodies lookup failed")?
        .ok_or_else(|| anyhow!("physics state missing `bodies`"))?
        .downcast_into::<PyList>()
        .map_err(|_| anyhow!("physics `bodies` must be list"))?;

    let mut out = Vec::new();
    for item in bodies.iter() {
        let body = item
            .downcast_into::<PyDict>()
            .map_err(|_| anyhow!("physics body entry must be dict"))?;
        out.push(PhysicsBodySpec {
            name: body
                .get_item("name")
                .context("physics body name lookup failed")?
                .ok_or_else(|| anyhow!("physics body missing `name`"))?
                .extract::<String>()
                .map_err(|_| anyhow!("physics body `name` must be string"))?,
            x: body
                .get_item("x")
                .context("physics body x lookup failed")?
                .ok_or_else(|| anyhow!("physics body missing `x`"))?
                .extract::<f64>()
                .map_err(|_| anyhow!("physics body `x` must be number"))?,
            y: body
                .get_item("y")
                .context("physics body y lookup failed")?
                .ok_or_else(|| anyhow!("physics body missing `y`"))?
                .extract::<f64>()
                .map_err(|_| anyhow!("physics body `y` must be number"))?,
            velocity_x: body
                .get_item("vx")
                .context("physics body vx lookup failed")?
                .ok_or_else(|| anyhow!("physics body missing `vx`"))?
                .extract::<f64>()
                .map_err(|_| anyhow!("physics body `vx` must be number"))?,
            velocity_y: body
                .get_item("vy")
                .context("physics body vy lookup failed")?
                .ok_or_else(|| anyhow!("physics body missing `vy`"))?
                .extract::<f64>()
                .map_err(|_| anyhow!("physics body `vy` must be number"))?,
            radius: body
                .get_item("radius")
                .context("physics body radius lookup failed")?
                .ok_or_else(|| anyhow!("physics body missing `radius`"))?
                .extract::<f64>()
                .map_err(|_| anyhow!("physics body `radius` must be number"))?,
            dynamic: body
                .get_item("dynamic")
                .context("physics body dynamic lookup failed")?
                .ok_or_else(|| anyhow!("physics body missing `dynamic`"))?
                .extract::<bool>()
                .map_err(|_| anyhow!("physics body `dynamic` must be bool"))?,
            restitution: body
                .get_item("restitution")
                .context("physics body restitution lookup failed")?
                .ok_or_else(|| anyhow!("physics body missing `restitution`"))?
                .extract::<f64>()
                .map_err(|_| anyhow!("physics body `restitution` must be number"))?,
        });
    }

    Ok(PhysicsSyncInput {
        gravity: PhysicsVec2::new(gravity.0, gravity.1),
        bodies: out,
    })
}

fn synchronize_physics(state: &mut RuntimePhysics, input: &PhysicsSyncInput) -> Result<()> {
    state.world.set_gravity(input.gravity);

    let mut seen_names = HashMap::new();
    for body in &input.bodies {
        let node = if let Some(node_id) = state.names_to_nodes.get(&body.name).copied() {
            node_id
        } else {
            let node_id = state
                .scene
                .add_node(state.scene.root(), format!("physics:{}", body.name))
                .with_context(|| {
                    format!("failed to add scene node for physics body `{}`", body.name)
                })?;
            state.names_to_nodes.insert(body.name.clone(), node_id);
            state.nodes_to_names.insert(node_id, body.name.clone());
            node_id
        };

        state
            .scene
            .set_node_transform(node, body.x, body.y, 0.0, 1.0, 1.0)
            .with_context(|| format!("failed to set transform for physics body `{}`", body.name))?;
        state
            .scene
            .set_physics_body(
                node,
                PhysicsBody2D {
                    kind: if body.dynamic {
                        PhysicsBodyKind::Dynamic
                    } else {
                        PhysicsBodyKind::Static
                    },
                    radius: body.radius,
                    velocity_x: body.velocity_x,
                    velocity_y: body.velocity_y,
                    restitution: body.restitution,
                },
            )
            .with_context(|| format!("failed to set body for physics body `{}`", body.name))?;

        seen_names.insert(body.name.clone(), node);
    }

    let stale = state
        .names_to_nodes
        .iter()
        .filter_map(|(name, node)| {
            if seen_names.contains_key(name) {
                None
            } else {
                Some((name.clone(), *node))
            }
        })
        .collect::<Vec<_>>();

    for (name, node) in stale {
        state.names_to_nodes.remove(&name);
        state.nodes_to_names.remove(&node);
        let _ = state.scene.clear_physics_body(node);
    }

    state.world.sync_from_scene(&state.scene);
    Ok(())
}

fn step_physics(state: &mut RuntimePhysics, dt_fixed: f64) -> Vec<PyPhysicsEvent> {
    let events = state.world.step(dt_fixed);
    state.world.apply_to_scene(&mut state.scene);

    events
        .into_iter()
        .filter_map(|event| {
            let a = state.nodes_to_names.get(&event.a)?.clone();
            let b = state.nodes_to_names.get(&event.b)?.clone();
            Some(PyPhysicsEvent { a, b })
        })
        .collect()
}

fn publish_physics_runtime_state(py: Python<'_>, state: &RuntimePhysics) -> Result<()> {
    let physics_mod = py
        .import_bound("vcon.physics")
        .context("failed to import vcon.physics")?;

    let bodies = PyList::empty_bound(py);
    for node in state.scene.nodes() {
        let Some(body) = node.physics_body.as_ref() else {
            continue;
        };
        let Some(name) = state.nodes_to_names.get(&node.id) else {
            continue;
        };
        let item = PyDict::new_bound(py);
        item.set_item("name", name)
            .context("set physics body name failed")?;
        item.set_item("x", node.transform.x)
            .context("set physics body x failed")?;
        item.set_item("y", node.transform.y)
            .context("set physics body y failed")?;
        item.set_item("vx", body.velocity_x)
            .context("set physics body vx failed")?;
        item.set_item("vy", body.velocity_y)
            .context("set physics body vy failed")?;
        item.set_item("radius", body.radius)
            .context("set physics body radius failed")?;
        bodies.append(item).context("append physics body failed")?;
    }

    let gravity = state.world.gravity();
    physics_mod
        .getattr("_set_runtime_state")
        .context("vcon.physics._set_runtime_state not found")?
        .call1(((gravity.x, gravity.y), bodies))
        .context("vcon.physics._set_runtime_state() failed")?;
    Ok(())
}

fn read_audio_runtime_commands(py: Python<'_>) -> Result<AudioRuntimeCommands> {
    let audio_mod = py
        .import_bound("vcon.audio")
        .context("failed to import vcon.audio")?;
    let exported = audio_mod
        .getattr("_export_runtime_state")
        .context("vcon.audio._export_runtime_state not found")?
        .call0()
        .context("vcon.audio._export_runtime_state() failed")?;
    let dict = exported
        .downcast_into::<PyDict>()
        .map_err(|_| anyhow!("vcon.audio._export_runtime_state() must return dict"))?;

    let mut out = AudioRuntimeCommands::default();

    if let Some(play_requests) = dict
        .get_item("play_requests")
        .context("audio play_requests lookup failed")?
    {
        let play_requests = play_requests
            .downcast_into::<PyList>()
            .map_err(|_| anyhow!("audio play_requests must be list"))?;

        for item in play_requests.iter() {
            let req = item
                .downcast_into::<PyDict>()
                .map_err(|_| anyhow!("audio play request entry must be dict"))?;
            let clip_id = req
                .get_item("clip_id")
                .context("audio clip_id lookup failed")?
                .ok_or_else(|| anyhow!("audio play request missing `clip_id`"))?
                .extract::<String>()
                .map_err(|_| anyhow!("audio clip_id must be string"))?;
            let volume = req
                .get_item("volume")
                .context("audio volume lookup failed")?
                .and_then(|value| value.extract::<f32>().ok())
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            let looped = req
                .get_item("looped")
                .context("audio looped lookup failed")?
                .and_then(|value| value.extract::<bool>().ok())
                .unwrap_or(false);
            out.play_requests.push(AudioPlayRequestSpec {
                clip_id,
                volume,
                looped,
            });
        }
    }

    if let Some(stop_ids) = dict
        .get_item("stop_voice_ids")
        .context("audio stop_voice_ids lookup failed")?
    {
        let stop_ids = stop_ids
            .downcast_into::<PyList>()
            .map_err(|_| anyhow!("audio stop_voice_ids must be list"))?;
        for item in stop_ids.iter() {
            out.stop_voice_ids.push(
                item.extract::<u64>()
                    .map_err(|_| anyhow!("audio stop voice id must be integer"))?,
            );
        }
    }

    out.stop_all = dict
        .get_item("stop_all")
        .context("audio stop_all lookup failed")?
        .and_then(|value| value.extract::<bool>().ok())
        .unwrap_or(false);

    Ok(out)
}

fn apply_audio_runtime_commands(audio: &mut RuntimeAudio, commands: &AudioRuntimeCommands) {
    if commands.stop_all {
        audio.mixer.stop_all();
    }
    for voice_id in &commands.stop_voice_ids {
        audio.mixer.stop_voice(*voice_id);
    }
    for request in &commands.play_requests {
        if request.looped {
            audio
                .mixer
                .queue_music(request.clip_id.clone(), request.volume, true);
        } else {
            audio
                .mixer
                .queue_sfx(request.clip_id.clone(), request.volume);
        }
    }
}

fn publish_audio_runtime_state(
    py: Python<'_>,
    active_voices: &[ActiveVoice],
    health: &AudioBackendHealth,
) -> Result<()> {
    let audio_mod = py
        .import_bound("vcon.audio")
        .context("failed to import vcon.audio")?;

    let active = PyList::empty_bound(py);
    for voice in active_voices {
        let item = PyDict::new_bound(py);
        item.set_item("voice_id", voice.voice_id)
            .context("set active audio voice id failed")?;
        item.set_item("clip_id", &voice.clip_id)
            .context("set active audio clip id failed")?;
        item.set_item("volume", voice.volume)
            .context("set active audio volume failed")?;
        item.set_item("looped", voice.looped)
            .context("set active audio looped failed")?;
        active.append(item).context("append active voice failed")?;
    }

    let health_dict = PyDict::new_bound(py);
    health_dict
        .set_item("initialized", health.initialized)
        .context("set audio health initialized failed")?;
    health_dict
        .set_item("queued_buffers", health.queued_buffers)
        .context("set audio health queued buffers failed")?;
    health_dict
        .set_item("underruns", health.underruns)
        .context("set audio health underruns failed")?;
    health_dict
        .set_item("overruns", health.overruns)
        .context("set audio health overruns failed")?;
    health_dict
        .set_item("dropped_buffers", health.dropped_buffers)
        .context("set audio health dropped buffers failed")?;

    audio_mod
        .getattr("_set_runtime_state")
        .context("vcon.audio._set_runtime_state not found")?
        .call1((active, health_dict))
        .context("vcon.audio._set_runtime_state() failed")?;
    Ok(())
}

#[derive(Debug, Clone)]
struct PyPhysicsEvent {
    a: String,
    b: String,
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

fn call_if_present1_event(
    module: &Bound<'_, PyModule>,
    callback: &str,
    event: &PyPhysicsEvent,
) -> Result<bool> {
    if let Ok(function) = module.getattr(callback) {
        if function.is_callable() {
            let py = module.py();
            let payload = PyDict::new_bound(py);
            payload
                .set_item("type", "physics.collision")
                .context("failed to set event type")?;
            payload
                .set_item("a", event.a.as_str())
                .context("failed to set event a")?;
            payload
                .set_item("b", event.b.as_str())
                .context("failed to set event b")?;

            function
                .call1((payload,))
                .with_context(|| format!("lifecycle callback `{callback}(event)` failed"))?;
            return Ok(true);
        }
    }

    Ok(false)
}

fn install_runtime_guards(py: Python<'_>, cartridge_root: &Path) -> Result<()> {
    let guard_source = build_sandbox_guard_source(cartridge_root)?;
    PyModule::from_code_bound(
        py,
        &guard_source,
        "_vcon_runtime_guard.py",
        "_vcon_runtime_guard",
    )
    .context("failed to install runtime sandbox guard")?;
    Ok(())
}

fn build_sandbox_guard_source(cartridge_root: &Path) -> Result<String> {
    let root = cartridge_root
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(cartridge_root))
        .to_string_lossy()
        .replace('\\', "/");
    let escaped = python_single_quoted_literal(&root);

    Ok(format!(
        r#"
import builtins
import os

_ALLOWED_ROOTS = {{"vcon"}}
_BLOCKED_NETWORK_ROOTS = {{"socket", "urllib", "http", "requests", "asyncio"}}
_CARTRIDGE_ROOT = os.path.normpath('{escaped}')


def _is_cartridge_context(globals_dict):
    # Direct __import__ calls can pass globals=None; treat those as sandboxed.
    if globals_dict is None:
        return True
    if not globals_dict:
        return False
    importer = globals_dict.get("__name__", "") or ""
    if importer.startswith("cartridge_entry"):
        return True

    module_file = globals_dict.get("__file__", "")
    if not module_file:
        return False
    normalized = os.path.normpath(str(module_file))
    return normalized == _CARTRIDGE_ROOT or normalized.startswith(_CARTRIDGE_ROOT + os.sep)


if not getattr(builtins, "__vcon_guard_installed__", False):
    _real_import = builtins.__import__

    def _vcon_import(name, globals=None, locals=None, fromlist=(), level=0):
        root = name.split(".", 1)[0] if name else ""

        if not _is_cartridge_context(globals):
            return _real_import(name, globals, locals, fromlist, level)

        if root in _BLOCKED_NETWORK_ROOTS:
            raise ImportError(f"vcon sandbox: blocked network module '{{root}}'")

        if level == 0 and root not in _ALLOWED_ROOTS:
            raise ImportError(
                f"vcon sandbox: import '{{root}}' is outside SDK-facing APIs"
            )

        return _real_import(name, globals, locals, fromlist, level)

    builtins.__import__ = _vcon_import
    builtins.__vcon_guard_installed__ = True
"#
    ))
}

fn python_single_quoted_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
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
    use crate::render_backend::ActiveRenderBackend;
    use vcon_engine::PhysicsBackend;

    #[test]
    fn invokes_sample_lifecycle_callbacks_loop_and_draw_commands() {
        let entrypoint = Path::new("../cartridges/sample-game/src/main.py");
        let cartridge_root = Path::new("../cartridges/sample-game");
        let sdk_root = Path::new("../vcon-sdk");
        let asset_dir = cartridge_root.join("assets");
        let save_root = std::env::temp_dir().join("vcon-runtime-save-test-sample");
        let _ = fs::remove_dir_all(&save_root);
        let mut provider = ScriptedInputProvider::default();

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
            ActiveRenderBackend::Software,
        )
        .expect("callbacks should execute");

        assert!(report.on_boot_called);
        assert!(report.on_shutdown_called);
        assert_eq!(report.on_update_calls, 4);
        assert_eq!(report.on_render_calls, 4);
        assert_eq!(report.on_event_calls, 0);
        assert_eq!(report.physics_events_dispatched, 0);
        assert_eq!(report.draw_commands_submitted, 24);
        assert_eq!(report.draw_commands_rendered, 24);
        assert_eq!(report.draw_commands_unsupported, 0);
        assert_eq!(report.render_backend, ActiveRenderBackend::Software);
        assert_eq!(report.physics_backend, PhysicsBackend::Box2d);
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
        let mut provider = ScriptedInputProvider::default();

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
            ActiveRenderBackend::Software,
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
        let mut provider = ScriptedInputProvider::default();

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
            ActiveRenderBackend::Software,
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

    #[test]
    fn blocks_original_import_bypass_attempt() {
        let (root, entrypoint) = write_temp_entrypoint(
            r#"
import vcon
import builtins
builtins.__vcon_original_import__("socket")
"#,
        );
        let save_root = std::env::temp_dir().join("vcon-runtime-save-test-bypass");
        let _ = fs::remove_dir_all(&save_root);
        let mut provider = ScriptedInputProvider::default();

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
            ActiveRenderBackend::Software,
        );
        let err = result.expect_err("bypass attempt should fail");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("__vcon_original_import__") || msg.contains("outside SDK-facing APIs"),
            "unexpected error: {msg}"
        );

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&save_root);
    }

    #[test]
    fn blocks_obfuscated_import_escape_at_runtime() {
        let (root, entrypoint) = write_temp_entrypoint(
            r#"
import vcon
imp = __builtins__["__im" + "port__"] if isinstance(__builtins__, dict) else getattr(__builtins__, "__im" + "port__")
imp("socket")
"#,
        );
        let save_root = std::env::temp_dir().join("vcon-runtime-save-test-obfuscated-bypass");
        let _ = fs::remove_dir_all(&save_root);
        let mut provider = ScriptedInputProvider::default();

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
            ActiveRenderBackend::Software,
        );
        let err = result.expect_err("obfuscated bypass should fail");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("blocked network module") && msg.contains("socket"),
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
