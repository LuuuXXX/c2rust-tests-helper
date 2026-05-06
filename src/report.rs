/// `report` subcommand – print a migration summary from the manifest and,
/// optionally, the last run results written by `check`.
use std::path::{Path, PathBuf};

use anyhow::Result;
use colored::Colorize;

use crate::manifest::{Manifest, MigrationStatus, RunResults};

pub fn run(config_path: &Path, results_path: Option<PathBuf>) -> Result<()> {
    let manifest = Manifest::load(config_path)?;

    println!("{}", "=== c2rust-tests-helper report ===".bold());
    println!("  Manifest: {}", config_path.display());
    println!();

    // --- Migration summary ------------------------------------------------
    print_migration_summary(&manifest);

    // --- Last run results (optional) --------------------------------------
    let resolved_results = results_path.or_else(|| {
        // Auto-detect the companion results file if it exists.
        let default = default_results_path(config_path);
        if default.exists() {
            Some(default)
        } else {
            None
        }
    });

    if let Some(rp) = resolved_results {
        println!();
        match RunResults::load(&rp) {
            Ok(results) => print_run_results(&results, &rp),
            Err(e) => eprintln!("{} could not load results file: {}", "warning:".yellow(), e),
        }
    } else {
        println!(
            "{}  No run results available. Run `check` first to generate them.",
            "ℹ".blue()
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_results_path(config_path: &Path) -> PathBuf {
    let stem = config_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "helper".to_string());
    let parent = config_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{}-results.yml", stem))
}

fn print_migration_summary(manifest: &Manifest) {
    let total = manifest.tests.len();
    let counts = manifest.status_counts();

    let ported = counts.get("ported").copied().unwrap_or(0);
    let pending = counts.get("pending").copied().unwrap_or(0);
    let skipped = counts.get("skipped").copied().unwrap_or(0);
    let na = counts.get("n/a").copied().unwrap_or(0);

    let pct = if total > 0 {
        ported as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    println!("{}", "Migration Status".bold().underline());
    println!("  Total entries : {}", total.to_string().bold());
    println!(
        "  Ported        : {} ({:.1}%)",
        ported.to_string().green().bold(),
        pct
    );
    println!("  Pending       : {}", pending.to_string().yellow().bold());
    println!("  Skipped       : {}", skipped.to_string().cyan());
    println!("  N/A           : {}", na.to_string().dimmed());

    // List pending entries as a quick to-do list.
    if pending > 0 {
        println!();
        println!("{}", "Pending entries (not yet ported):".bold());
        for entry in manifest
            .tests
            .iter()
            .filter(|e| e.status == MigrationStatus::Pending)
        {
            print!(
                "  • {} ({})",
                entry.name.yellow(),
                entry.source_file.dimmed()
            );
            if let Some(note) = &entry.notes {
                print!("  — {}", note.italic());
            }
            println!();
        }
    }
}

fn print_run_results(results: &RunResults, path: &Path) {
    println!(
        "{} (from '{}', run at {})",
        "Last Run Results".bold().underline(),
        path.display(),
        results.timestamp.dimmed()
    );

    let c_icon = if results.c_result.success {
        "✓".green().bold()
    } else {
        "✗".red().bold()
    };
    let rust_icon = if results.rust_result.success {
        "✓".green().bold()
    } else {
        "✗".red().bold()
    };

    println!(
        "  C tests   : {} (exit {})",
        c_icon,
        results
            .c_result
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string())
    );
    println!(
        "  Rust tests: {} (exit {})",
        rust_icon,
        results
            .rust_result
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string())
    );
}
