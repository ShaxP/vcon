use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use vcon_engine::boot_cartridge;

#[derive(Debug, Parser)]
#[command(name = "vcon-runtime", about = "Virtual Console runtime host")]
struct Args {
    #[arg(long, default_value = "cartridges/sample-game")]
    cartridge: PathBuf,
    #[arg(long, default_value = "/tmp/vcon/saves")]
    saves_root: PathBuf,
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

    if report.lifecycle.on_boot {
        println!("Invoking lifecycle callback: on_boot() [stub]");
    }

    if report.lifecycle.on_shutdown {
        println!("Invoking lifecycle callback: on_shutdown() [stub]");
    }

    Ok(())
}
