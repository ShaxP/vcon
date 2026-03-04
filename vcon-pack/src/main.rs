use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use vcon_engine::sandbox::{scan_entrypoint_source, validate_manifest_permissions};
use vcon_engine::Manifest;

const BUNDLE_MAGIC: &[u8] = b"VCONPKG";
const BUNDLE_VERSION: u8 = 1;
const MANIFEST_PATH: &str = "vcon.toml";
const DISALLOWED_DEPENDENCY_FILES: &[&str] = &["requirements.txt", "pyproject.toml", "pipfile", "poetry.lock"];

#[derive(Debug, Parser)]
#[command(name = "vcon-pack", about = "Cartridge packaging and validation tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Build {
        #[arg(long, default_value = "cartridges/sample-game")]
        cartridge: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Validate {
        /// Cartridge directory or .vcon bundle path.
        #[arg(long, default_value = "cartridges/sample-game")]
        cartridge: PathBuf,
    },
}

#[derive(Debug, Clone)]
struct BundleFile {
    path: String,
    bytes: Vec<u8>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { cartridge, output } => build_cartridge(&cartridge, output.as_deref()),
        Commands::Validate { cartridge } => validate_input(&cartridge),
    }
}

fn build_cartridge(cartridge_dir: &Path, output: Option<&Path>) -> Result<()> {
    let canonical_cartridge = fs::canonicalize(cartridge_dir)
        .with_context(|| format!("failed to resolve cartridge path {}", cartridge_dir.display()))?;

    let manifest = validate_cartridge_dir(&canonical_cartridge)?;

    let output_path = resolve_output_path(output, &manifest)?;
    let output_abs = absolute_path(&output_path)?;

    let files = collect_cartridge_files(&canonical_cartridge, Some(&output_abs))?;
    let bundle_bytes = encode_bundle(&files)?;

    // Explicit reproducibility check: same input set must encode identically.
    let reproduced = encode_bundle(&files)?;
    if reproduced != bundle_bytes {
        bail!("non-deterministic bundle encoding detected; refusing to emit package");
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating output directory {}", parent.display()))?;
    }

    let mut file = fs::File::create(&output_path)
        .with_context(|| format!("failed creating output file {}", output_path.display()))?;
    file.write_all(&bundle_bytes)
        .with_context(|| format!("failed writing output file {}", output_path.display()))?;

    println!(
        "Built bundle for cartridge {} -> {} ({} files, {} bytes)",
        manifest.id,
        output_path.display(),
        files.len(),
        bundle_bytes.len()
    );

    Ok(())
}

fn validate_input(path: &Path) -> Result<()> {
    if path.is_file() {
        validate_bundle(path)
    } else {
        let manifest = validate_cartridge_dir(path)?;
        println!("Manifest valid for cartridge {}", manifest.id);
        Ok(())
    }
}

fn validate_cartridge_dir(cartridge_dir: &Path) -> Result<Manifest> {
    let manifest_path = cartridge_dir.join(MANIFEST_PATH);
    let source = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed reading {}", manifest_path.display()))?;

    let manifest = parse_manifest_with_context(&source, &manifest_path)?;
    validate_manifest_and_policy(&manifest, cartridge_dir)?;

    Ok(manifest)
}

fn validate_bundle(bundle_path: &Path) -> Result<()> {
    let bytes = fs::read(bundle_path)
        .with_context(|| format!("failed reading bundle {}", bundle_path.display()))?;
    let files = decode_bundle(&bytes)
        .with_context(|| format!("invalid bundle format in {}", bundle_path.display()))?;

    let manifest_file = files
        .iter()
        .find(|file| file.path == MANIFEST_PATH)
        .ok_or_else(|| anyhow!("bundle missing required file `{MANIFEST_PATH}`"))?;
    let manifest_source = std::str::from_utf8(&manifest_file.bytes)
        .context("bundle manifest must be valid UTF-8")?;
    let manifest = parse_manifest_with_context(manifest_source, Path::new(MANIFEST_PATH))?;

    validate_manifest_and_policy_from_files(&manifest, &files)?;

    println!(
        "Bundle valid for cartridge {} ({}, {} files)",
        manifest.id,
        bundle_path.display(),
        files.len()
    );

    Ok(())
}

fn validate_manifest_and_policy(manifest: &Manifest, cartridge_dir: &Path) -> Result<()> {
    manifest
        .validate_sdk_version_compatibility()
        .map_err(|err| anyhow!("{}", err))?;

    let permission_violations = validate_manifest_permissions(manifest);
    if !permission_violations.is_empty() {
        let msg = permission_violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        bail!("policy validation failed: {msg}");
    }

    let entrypoint_path = cartridge_dir.join(&manifest.entrypoint);
    if !entrypoint_path.is_file() {
        bail!("entrypoint file not found: {}", entrypoint_path.display());
    }

    let assets_path = cartridge_dir.join(&manifest.assets_path);
    if !assets_path.is_dir() {
        bail!(
            "assets path not found or not a directory: {}",
            assets_path.display()
        );
    }

    let all_files = collect_cartridge_files(cartridge_dir, None)?;
    validate_disallowed_dependency_files(all_files.iter().map(|f| f.path.as_str()))?;

    let mut python_files = all_files
        .iter()
        .filter(|file| file.path.ends_with(".py"))
        .collect::<Vec<_>>();
    python_files.sort_by(|a, b| a.path.cmp(&b.path));

    if python_files.is_empty() {
        bail!("cartridge must include at least one Python source file (*.py)");
    }

    for file in python_files {
        let abs = cartridge_dir.join(&file.path);
        let source = fs::read_to_string(&abs)
            .with_context(|| format!("failed reading python source {}", abs.display()))?;
        let violations = scan_entrypoint_source(&source);
        if !violations.is_empty() {
            let msg = violations
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            bail!("policy validation failed in {}: {msg}", file.path);
        }
    }

    Ok(())
}

fn validate_manifest_and_policy_from_files(manifest: &Manifest, files: &[BundleFile]) -> Result<()> {
    manifest
        .validate_sdk_version_compatibility()
        .map_err(|err| anyhow!("{}", err))?;

    let permission_violations = validate_manifest_permissions(manifest);
    if !permission_violations.is_empty() {
        let msg = permission_violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        bail!("policy validation failed: {msg}");
    }

    if !files.iter().any(|file| file.path == manifest.entrypoint) {
        bail!(
            "entrypoint file not found in bundle: {}",
            manifest.entrypoint
        );
    }

    let assets_prefix = normalized_bundle_dir_prefix(&manifest.assets_path)?;
    if !files
        .iter()
        .any(|file| file.path.starts_with(&assets_prefix))
    {
        bail!(
            "assets path not found in bundle: {}",
            manifest.assets_path
        );
    }

    validate_disallowed_dependency_files(files.iter().map(|f| f.path.as_str()))?;

    let mut python_files = files
        .iter()
        .filter(|file| file.path.ends_with(".py"))
        .collect::<Vec<_>>();
    python_files.sort_by(|a, b| a.path.cmp(&b.path));

    if python_files.is_empty() {
        bail!("bundle must include at least one Python source file (*.py)");
    }

    for file in python_files {
        let source = std::str::from_utf8(&file.bytes)
            .with_context(|| format!("python file {} must be UTF-8", file.path))?;
        let violations = scan_entrypoint_source(source);
        if !violations.is_empty() {
            let msg = violations
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            bail!("policy validation failed in {}: {msg}", file.path);
        }
    }

    Ok(())
}

fn parse_manifest_with_context(source: &str, manifest_path: &Path) -> Result<Manifest> {
    Manifest::parse(source).map_err(|err| {
        anyhow!(
            "manifest error in {}: {err}",
            manifest_path.display()
        )
    })
}

fn resolve_output_path(output: Option<&Path>, manifest: &Manifest) -> Result<PathBuf> {
    if let Some(path) = output {
        return Ok(path.to_path_buf());
    }

    let file_name = format!(
        "{}-{}.vcon",
        sanitize_for_filename(&manifest.id),
        sanitize_for_filename(&manifest.version)
    );

    Ok(PathBuf::from("dist").join(file_name))
}

fn sanitize_for_filename(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn collect_cartridge_files(cartridge_root: &Path, output_abs: Option<&Path>) -> Result<Vec<BundleFile>> {
    let mut out = Vec::new();
    let mut pending = vec![cartridge_root.to_path_buf()];

    while let Some(dir) = pending.pop() {
        let mut entries = fs::read_dir(&dir)
            .with_context(|| format!("failed listing directory {}", dir.display()))?
            .collect::<std::io::Result<Vec<_>>>()
            .with_context(|| format!("failed reading directory entries for {}", dir.display()))?;

        entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .with_context(|| format!("failed reading metadata for {}", path.display()))?;

            if metadata.file_type().is_symlink() {
                bail!("symlinks are not allowed in cartridge package: {}", path.display());
            }

            if metadata.is_dir() {
                pending.push(path);
                continue;
            }

            if !metadata.is_file() {
                continue;
            }

            if let Some(output_abs) = output_abs {
                let file_abs = absolute_path(&path)?;
                if file_abs == *output_abs {
                    continue;
                }
            }

            let rel = path
                .strip_prefix(cartridge_root)
                .with_context(|| {
                    format!(
                        "failed computing relative path for {} against {}",
                        path.display(),
                        cartridge_root.display()
                    )
                })?;

            let normalized = normalize_relative_path(rel)?;
            let bytes = fs::read(&path)
                .with_context(|| format!("failed reading file {}", path.display()))?;

            out.push(BundleFile {
                path: normalized,
                bytes,
            });
        }
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn normalize_relative_path(path: &Path) -> Result<String> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let text = part
                    .to_str()
                    .ok_or_else(|| anyhow!("path contains non-UTF-8 segment: {}", path.display()))?;
                if text.is_empty() {
                    bail!("path contains empty segment: {}", path.display());
                }
                components.push(text.to_owned());
            }
            Component::CurDir => {}
            _ => bail!("path is not a clean relative path: {}", path.display()),
        }
    }

    if components.is_empty() {
        bail!("path resolved to empty relative path: {}", path.display());
    }

    Ok(components.join("/"))
}

fn normalized_bundle_dir_prefix(path: &str) -> Result<String> {
    let normalized = normalize_relative_path(Path::new(path))?;
    Ok(format!("{normalized}/"))
}

fn validate_disallowed_dependency_files<'a>(paths: impl Iterator<Item = &'a str>) -> Result<()> {
    for path in paths {
        let file_name = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .to_ascii_lowercase();

        if DISALLOWED_DEPENDENCY_FILES.contains(&file_name.as_str()) {
            bail!(
                "disallowed dependency manifest detected at {} (third-party dependencies are not supported in V1)",
                path
            );
        }
    }
    Ok(())
}

fn encode_bundle(files: &[BundleFile]) -> Result<Vec<u8>> {
    let mut seen = HashSet::new();

    for file in files {
        if !seen.insert(file.path.as_str()) {
            bail!("duplicate bundle path detected: {}", file.path);
        }
    }

    let mut out = Vec::new();
    out.extend_from_slice(BUNDLE_MAGIC);
    out.push(BUNDLE_VERSION);
    out.extend_from_slice(&(files.len() as u32).to_le_bytes());

    for file in files {
        let path_bytes = file.path.as_bytes();
        out.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(path_bytes);
        out.extend_from_slice(&(file.bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(&file.bytes);
    }

    Ok(out)
}

fn decode_bundle(bytes: &[u8]) -> Result<Vec<BundleFile>> {
    if bytes.len() < BUNDLE_MAGIC.len() + 1 + 4 {
        bail!("bundle too small to contain header");
    }

    if &bytes[..BUNDLE_MAGIC.len()] != BUNDLE_MAGIC {
        bail!("bundle magic mismatch");
    }

    let mut cursor = BUNDLE_MAGIC.len();
    let version = read_u8(bytes, &mut cursor)?;
    if version != BUNDLE_VERSION {
        bail!(
            "unsupported bundle version marker `{version}` (expected `{BUNDLE_VERSION}`)"
        );
    }

    let file_count = read_u32(bytes, &mut cursor)? as usize;
    let mut files = Vec::with_capacity(file_count);
    let mut seen = HashSet::new();

    for _ in 0..file_count {
        let path_len = read_u32(bytes, &mut cursor)? as usize;
        let path_bytes = read_bytes(bytes, &mut cursor, path_len)?;
        let path = std::str::from_utf8(path_bytes)
            .context("bundle path must be valid UTF-8")?
            .to_owned();

        let normalized = normalize_relative_path(Path::new(&path))?;
        if normalized != path {
            bail!("bundle path is not normalized: {path}");
        }
        if !seen.insert(normalized.clone()) {
            bail!("duplicate bundle path detected: {normalized}");
        }

        let file_len = read_u64(bytes, &mut cursor)? as usize;
        let file_bytes = read_bytes(bytes, &mut cursor, file_len)?.to_vec();
        files.push(BundleFile {
            path: normalized,
            bytes: file_bytes,
        });
    }

    if cursor != bytes.len() {
        bail!("bundle has trailing bytes after declared payload");
    }

    Ok(files)
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8> {
    if *cursor >= bytes.len() {
        bail!("unexpected end of bundle while reading u8");
    }
    let value = bytes[*cursor];
    *cursor += 1;
    Ok(value)
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32> {
    let raw = read_bytes(bytes, cursor, 4)?;
    Ok(u32::from_le_bytes(raw.try_into().expect("u32 size is fixed")))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64> {
    let raw = read_bytes(bytes, cursor, 8)?;
    Ok(u64::from_le_bytes(raw.try_into().expect("u64 size is fixed")))
}

fn read_bytes<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| anyhow!("bundle offset overflow while reading payload"))?;
    if end > bytes.len() {
        bail!("unexpected end of bundle while reading payload");
    }
    let out = &bytes[*cursor..end];
    *cursor = end;
    Ok(out)
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .context("failed to resolve current directory")?
            .join(path))
    }
}
