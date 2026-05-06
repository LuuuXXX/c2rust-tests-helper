mod collect;
mod config;
mod feature;
mod lint;
mod report;
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
    /// Discover C tests and merge them into the migration manifest.
    Collect {
        /// Path to the migration config file (default: migration.yml).
        #[arg(long, short, default_value = "migration.yml")]
        config: PathBuf,
    },
    /// Validate the migration manifest against the loaded feature surface.
    Lint {
        /// Path to the migration config file (default: migration.yml).
        #[arg(long, short, default_value = "migration.yml")]
        config: PathBuf,
    },
    /// Print a migration-status and feature-coverage report.
    Report {
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
        Command::Collect { config } => cmd_collect(&config),
        Command::Lint { config } => cmd_lint(&config),
        Command::Report { config } => cmd_report(&config),
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

// ── collect ───────────────────────────────────────────────────────────────────

fn cmd_collect(config_path: &Path) -> Result<()> {
    let mut cfg = config::load_config(config_path)
        .with_context(|| format!("loading config from {}", config_path.display()))?;

    validate_config(&cfg, config_path)?;

    let added = collect::collect_tests(&mut cfg, config_path)
        .with_context(|| "collecting tests")?;

    config::save_config(config_path, &cfg)
        .with_context(|| format!("writing config back to {}", config_path.display()))?;

    println!("collect: added {added} new test(s); manifest updated.");
    Ok(())
}

// ── lint ──────────────────────────────────────────────────────────────────────

fn cmd_lint(config_path: &Path) -> Result<()> {
    let cfg = config::load_config(config_path)
        .with_context(|| format!("loading config from {}", config_path.display()))?;

    validate_config(&cfg, config_path)?;

    let feature_root = resolve_path(config_path, &cfg.feature_source.root);
    let index = feature::load(&feature_root)
        .with_context(|| format!("loading feature surface from {}", feature_root.display()))?;

    lint::lint(&cfg, &index)
}

// ── report ────────────────────────────────────────────────────────────────────

fn cmd_report(config_path: &Path) -> Result<()> {
    let cfg = config::load_config(config_path)
        .with_context(|| format!("loading config from {}", config_path.display()))?;

    validate_config(&cfg, config_path)?;

    let feature_root = resolve_path(config_path, &cfg.feature_source.root);
    let index = feature::load(&feature_root)
        .with_context(|| format!("loading feature surface from {}", feature_root.display()))?;

    report::report(&cfg, &index);
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

    // Lightweight consistency check: the last path component of feature_source.root
    // should match project.feature so the two fields don't silently diverge.
    if let Some(feature_dir) = Path::new(&cfg.feature_source.root).file_name() {
        let feature_dir = feature_dir.to_str().with_context(|| {
            format!(
                "feature_source.root last component is not valid UTF-8: {}",
                cfg.feature_source.root
            )
        })?;
        if feature_dir != cfg.project.feature {
            bail!(
                "project.feature ({:?}) does not match the last component of \
                 feature_source.root ({:?}); they should refer to the same feature",
                cfg.project.feature,
                feature_dir
            );
        }
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
