use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use vcon_engine::boot_cartridge;

mod python_host;

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

    let runtime_report = python_host::run_cartridge(
        &report.entrypoint_path,
        &args.cartridge,
        &args.sdk_root,
        args.frames,
        args.dt_fixed,
    )?;

    if runtime_report.on_boot_called {
        println!("Invoked lifecycle callback: on_boot() [python]");
    }
    println!(
        "Loop callbacks invoked: on_update={} on_render={}",
        runtime_report.on_update_calls, runtime_report.on_render_calls
    );
    if runtime_report.on_shutdown_called {
        println!("Invoked lifecycle callback: on_shutdown() [python]");
    }

    Ok(())
}
