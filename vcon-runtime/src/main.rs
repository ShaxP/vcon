use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use vcon_engine::boot_cartridge;

mod gamepad;
mod python_host;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InputSourceArg {
    None,
    Scripted,
    Gamepad,
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
    #[arg(long)]
    dump_frame: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let report = boot_cartridge(&args.cartridge, &args.saves_root)?;

    println!("Loaded cartridge: {} ({})", report.manifest.name, report.manifest.id);
    println!("Entrypoint: {}", report.entrypoint_path.display());
    println!("Lifecycle availability:");
    println!("  on_boot: {}", report.lifecycle.on_boot);
    println!("  on_shutdown: {}", report.lifecycle.on_shutdown);
    println!("Save namespace: {}", report.save_namespace.root.display());
    println!(
        "Save quota: {} MB",
        report.save_namespace.quota_mb
    );

    let mut none_provider = python_host::NoneInputProvider;
    let mut scripted_provider = python_host::ScriptedInputProvider;
    let mut gamepad_provider = gamepad::GamepadInputProvider::new();

    let input_provider: &mut dyn python_host::InputProvider = match args.input_source {
        InputSourceArg::None => &mut none_provider,
        InputSourceArg::Scripted => &mut scripted_provider,
        InputSourceArg::Gamepad => &mut gamepad_provider,
    };

    let runtime_report = python_host::run_cartridge(
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
    )?;

    if runtime_report.on_boot_called {
        println!("Invoked lifecycle callback: on_boot() [python]");
    }
    println!(
        "Loop callbacks invoked: on_update={} on_render={}",
        runtime_report.on_update_calls, runtime_report.on_render_calls
    );
    println!(
        "Draw commands submitted: {}",
        runtime_report.draw_commands_submitted
    );
    println!(
        "Draw commands rendered: {} (unsupported: {})",
        runtime_report.draw_commands_rendered, runtime_report.draw_commands_unsupported
    );
    if runtime_report.on_shutdown_called {
        println!("Invoked lifecycle callback: on_shutdown() [python]");
    }
    if let Some(path) = args.dump_frame {
        println!("Dumped final frame to {}", path.display());
    }

    Ok(())
}
