use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use vcon_engine::Manifest;
use vcon_engine::sandbox::validate_manifest_permissions;

#[derive(Debug, Parser)]
#[command(name = "vcon-pack", about = "Cartridge packaging and validation tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Validate {
        #[arg(long, default_value = "cartridges/sample-game")]
        cartridge: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { cartridge } => validate_cartridge(&cartridge),
    }
}

fn validate_cartridge(cartridge_dir: &PathBuf) -> Result<()> {
    let manifest_path = cartridge_dir.join("vcon.toml");
    let source = fs::read_to_string(&manifest_path)
        .map_err(|err| anyhow!("failed reading {}: {err}", manifest_path.display()))?;

    let manifest = Manifest::parse(&source)?;
    let permission_violations = validate_manifest_permissions(&manifest);
    if !permission_violations.is_empty() {
        let msg = permission_violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(anyhow!("policy validation failed: {msg}"));
    }

    let entrypoint_path = cartridge_dir.join(&manifest.entrypoint);
    if !entrypoint_path.exists() {
        return Err(anyhow!(
            "entrypoint file not found: {}",
            entrypoint_path.display()
        ));
    }

    println!("Manifest valid for cartridge {}", manifest.id);
    Ok(())
}
