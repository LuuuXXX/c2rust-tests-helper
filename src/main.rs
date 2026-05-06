mod config;
mod feature;
#[cfg(test)]
mod tests;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "c2rust-tests-helper",
    about = "Test migration helper for c2rust-demo feature workspaces",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Load and print the feature surface from a migration config.
    Surface {
        /// Path to the migration config file (default: migration.yml).
        #[arg(long, short, default_value = "migration.yml")]
        config: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e:?}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Surface { config } => cmd_surface(&config),
    }
}

// ── surface ───────────────────────────────────────────────────────────────────

fn cmd_surface(config_path: &Path) -> Result<()> {
    let cfg = config::load_config(config_path)
        .with_context(|| format!("loading config from {}", config_path.display()))?;

    validate_config(&cfg, config_path)?;

    let feature_root = resolve_path(config_path, &cfg.feature_source.root);
    let index = feature::load(&feature_root)
        .with_context(|| format!("loading feature surface from {}", feature_root.display()))?;

    index.print_summary();
    Ok(())
}

// ── validation ────────────────────────────────────────────────────────────────

fn validate_config(cfg: &config::Config, config_path: &Path) -> Result<()> {
    let project_root = resolve_path(config_path, &cfg.project.root);
    if !project_root.exists() {
        bail!(
            "project.root does not exist: {} (resolved from config at {})",
            project_root.display(),
            config_path.display()
        );
    }

    let feature_root = resolve_path(config_path, &cfg.feature_source.root);
    if !feature_root.exists() {
        bail!(
            "feature_source.root does not exist: {} (resolved from config at {})",
            feature_root.display(),
            config_path.display()
        );
    }

    Ok(())
}

/// Resolve `p` relative to the directory that contains `config_path`.
/// Absolute paths are returned unchanged.
fn resolve_path(config_path: &Path, p: &str) -> PathBuf {
    let relative_to = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let joined = relative_to.join(p);
    // Canonicalise if possible; fall back to the joined path.
    joined.canonicalize().unwrap_or(joined)
}
