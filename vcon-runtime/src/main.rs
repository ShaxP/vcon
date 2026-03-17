use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use vcon_engine::boot_cartridge;

mod audio_backend;
mod gamepad;
mod python_host;
mod render_backend;
mod wgpu_presenter;
mod window_runtime;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InputSourceArg {
    None,
    Scripted,
    Gamepad,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum RenderBackendArg {
    Auto,
    Software,
    Moderngl,
    Wgpu,
}

#[derive(Debug, Parser)]
#[command(name = "vcon-runtime", about = "Virtual Console runtime host")]
struct Args {
    #[arg(long, default_value = "cartridges/sample-game")]
    cartridge: PathBuf,
    #[arg(long, default_value = "/tmp/vcon/saves")]
    saves_root: PathBuf,
    #[arg(long, default_value = "vcon-sdk")]
    sdk_root: PathBuf,
    #[arg(long, default_value_t = 3)]
    frames: u32,
    #[arg(long, default_value_t = 1.0 / 60.0)]
    dt_fixed: f64,
    #[arg(long, default_value_t = 1280)]
    width: u32,
    #[arg(long, default_value_t = 800)]
    height: u32,
    #[arg(long, value_enum, default_value_t = InputSourceArg::Scripted)]
    input_source: InputSourceArg,
    #[arg(long, default_value_t = 0)]
    input_seed: u64,
    #[arg(long, value_enum, default_value_t = RenderBackendArg::Auto)]
    render_backend: RenderBackendArg,
    #[arg(long)]
    dump_frame: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    windowed: bool,
    #[arg(long, default_value_t = 60)]
    windowed_target_fps: u32,
    #[arg(long)]
    windowed_max_frames: Option<u32>,
    #[arg(long, default_value = "vcon-runtime")]
    window_title: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let report = boot_cartridge(&args.cartridge, &args.saves_root)?;

    println!(
        "Loaded cartridge: {} ({})",
        report.manifest.name, report.manifest.id
    );
    println!("Entrypoint: {}", report.entrypoint_path.display());
    println!("Entrypoint API: module-level `cartridge` object");
    println!("Save namespace: {}", report.save_namespace.root.display());
    println!("Save quota: {} MB", report.save_namespace.quota_mb);

    let mut none_provider = python_host::NoneInputProvider;
    let mut scripted_provider = python_host::ScriptedInputProvider::with_seed(args.input_seed);
    let mut gamepad_provider = gamepad::GamepadInputProvider::new();

    let input_provider: &mut dyn python_host::InputProvider = match args.input_source {
        InputSourceArg::None => &mut none_provider,
        InputSourceArg::Scripted => &mut scripted_provider,
        InputSourceArg::Gamepad => &mut gamepad_provider,
    };

    let backend_request = match args.render_backend {
        RenderBackendArg::Auto => render_backend::RenderBackendRequest::Auto,
        RenderBackendArg::Software => render_backend::RenderBackendRequest::Software,
        RenderBackendArg::Moderngl => render_backend::RenderBackendRequest::Moderngl,
        RenderBackendArg::Wgpu => render_backend::RenderBackendRequest::Wgpu,
    };
    let backend_selection = render_backend::select_render_backend(backend_request);

    let runtime_report = if args.windowed {
        let (mut window_input, mut window_observer) = window_runtime::create_window_runtime(
            &args.window_title,
            args.width,
            args.height,
            args.windowed_target_fps,
        )?;
        python_host::run_cartridge_with_loop(
            &report.entrypoint_path,
            &args.cartridge,
            &args.sdk_root,
            python_host::FrameLoopMode::UntilStopped {
                max_frames: args.windowed_max_frames,
            },
            args.dt_fixed,
            args.width,
            args.height,
            &mut window_input,
            &report.save_namespace.root,
            report.save_namespace.quota_mb,
            Some(&args.cartridge.join(&report.manifest.assets_path)),
            args.dump_frame.as_deref(),
            backend_selection.active,
            Some(&mut window_observer),
        )?
    } else {
        python_host::run_cartridge(
            &report.entrypoint_path,
            &args.cartridge,
            &args.sdk_root,
            args.frames,
            args.dt_fixed,
            args.width,
            args.height,
            input_provider,
            &report.save_namespace.root,
            report.save_namespace.quota_mb,
            Some(&args.cartridge.join(&report.manifest.assets_path)),
            args.dump_frame.as_deref(),
            backend_selection.active,
        )?
    };

    if runtime_report.on_boot_called {
        println!("Invoked lifecycle callback: on_boot() [python]");
    }
    println!(
        "Loop callbacks invoked: on_update={} on_render={}",
        runtime_report.on_update_calls, runtime_report.on_render_calls
    );
    println!(
        "Event callbacks invoked: on_event={} (physics events: {})",
        runtime_report.on_event_calls, runtime_report.physics_events_dispatched
    );
    println!(
        "Draw commands submitted: {}",
        runtime_report.draw_commands_submitted
    );
    println!(
        "Draw commands rendered: {} (unsupported: {})",
        runtime_report.draw_commands_rendered, runtime_report.draw_commands_unsupported
    );
    println!(
        "Render backend: requested={:?} active={}",
        backend_selection.requested,
        runtime_report.render_backend.as_str()
    );
    println!(
        "Physics backend: {}",
        runtime_report.physics_backend.as_str()
    );
    println!(
        "Audio backend: {} (underruns={} overruns={} dropped_buffers={})",
        runtime_report.audio_backend,
        runtime_report.audio_underruns,
        runtime_report.audio_overruns,
        runtime_report.audio_dropped_buffers
    );
    println!(
        "Render timing (us): cpu_render_total={} present_total={} pacing_anomalies={}",
        runtime_report.render_cpu_micros_total,
        runtime_report.present_micros_total,
        runtime_report.frame_pacing_anomalies
    );
    if let Some(reason) = &backend_selection.fallback_reason {
        println!("Render backend fallback: {reason}");
    }
    if runtime_report.on_shutdown_called {
        println!("Invoked lifecycle callback: on_shutdown() [python]");
    }
    if let Some(path) = args.dump_frame {
        println!("Dumped final frame to {}", path.display());
    }

    Ok(())
}
