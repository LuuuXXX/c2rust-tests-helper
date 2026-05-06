/// `check` subcommand – validate the manifest, run C and Rust test commands,
/// write a results file, and print a human-readable report.
use std::path::{Path, PathBuf};

use anyhow::Result;
use colored::Colorize;

use crate::manifest::{Manifest, RunResults};
use crate::runner::run_command;

pub fn run(config_path: &Path, results_path: Option<PathBuf>) -> Result<()> {
    // Resolve results file path.
    let results_path = results_path.unwrap_or_else(|| default_results_path(config_path));

    println!("{}", "=== c2rust-tests-helper check ===".bold());
    println!("  Manifest : {}", config_path.display());
    println!("  Results  : {}", results_path.display());
    println!();

    // 1. Load & validate manifest (fails loudly on bad data).
    let manifest = Manifest::load(config_path)?;
    println!("{} manifest OK ({} entries)", "✓".green(), manifest.tests.len());

    // 2. Determine working directory.
    let working_dir = resolve_working_dir(&manifest, config_path);

    // 3. Run C tests.
    println!(
        "\n{} Running C tests: {}",
        "→".cyan(),
        manifest.config.c_test_command.bold()
    );
    let c_result = run_command(
        &manifest.config.c_test_command,
        &working_dir,
        &manifest.config.env,
    )?;
    print_command_result(&c_result);

    // 4. Run Rust tests.
    println!(
        "\n{} Running Rust tests: {}",
        "→".cyan(),
        manifest.config.rust_test_command.bold()
    );
    let rust_result = run_command(
        &manifest.config.rust_test_command,
        &working_dir,
        &manifest.config.env,
    )?;
    print_command_result(&rust_result);

    // 5. Write results file.
    let timestamp = current_timestamp();
    let run_results = RunResults {
        timestamp,
        c_result,
        rust_result,
    };
    run_results.save(&results_path)?;
    println!(
        "\n{} Results written to '{}'",
        "✓".green(),
        results_path.display()
    );

    // 6. Summary.
    println!("\n{}", "=== Summary ===".bold());
    let c_status = if run_results.c_result.success {
        "PASS".green().bold()
    } else {
        "FAIL".red().bold()
    };
    let rust_status = if run_results.rust_result.success {
        "PASS".green().bold()
    } else {
        "FAIL".red().bold()
    };
    println!("  C tests   : {}", c_status);
    println!("  Rust tests: {}", rust_status);

    let counts = manifest.status_counts();
    println!("\n  Migration manifest ({} total):", manifest.tests.len());
    for status in &["ported", "pending", "skipped", "n/a"] {
        let n = counts.get(*status).copied().unwrap_or(0);
        println!("    {:<14}: {}", status, n);
    }

    if !run_results.c_result.success || !run_results.rust_result.success {
        anyhow::bail!("one or more test commands failed – see output above");
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

fn resolve_working_dir(manifest: &Manifest, config_path: &Path) -> PathBuf {
    if let Some(wd) = &manifest.config.working_dir {
        let p = PathBuf::from(wd);
        if p.is_absolute() {
            return p;
        }
        // Relative to the config file's directory.
        let base = config_path.parent().unwrap_or_else(|| Path::new("."));
        return base.join(p);
    }
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn print_command_result(r: &crate::manifest::CommandResult) {
    let status = if r.success {
        "PASS".green().bold()
    } else {
        "FAIL".red().bold()
    };
    println!(
        "  Status: {} (exit code: {})",
        status,
        r.exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string())
    );
    if !r.output_tail.is_empty() {
        println!("  Output (tail):");
        for line in r.output_tail.lines() {
            println!("    {}", line);
        }
    }
}

fn current_timestamp() -> String {
    // Use a simple approach without extra dependencies: read /proc/... or
    // fall back to the Unix epoch as a string via std.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as ISO 8601-ish without chrono.
    let (y, mo, d, h, mi, s) = epoch_to_ymd(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, mi, s)
}

/// Minimal epoch → (year, month, day, hour, min, sec) without chrono.
fn epoch_to_ymd(epoch: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = epoch % 60;
    let mi = (epoch / 60) % 60;
    let h = (epoch / 3600) % 24;
    let days = epoch / 86400;

    // Compute year/month/day using the proleptic Gregorian calendar algorithm.
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };

    (y, mo, d, h, mi, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch_to_ymd_unix_epoch() {
        let (y, mo, d, h, mi, s) = epoch_to_ymd(0);
        assert_eq!((y, mo, d, h, mi, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn test_epoch_to_ymd_known_date() {
        // 2024-03-15 12:30:00 UTC = 1_710_505_800
        let (y, mo, d, h, mi, s) = epoch_to_ymd(1_710_505_800);
        assert_eq!((y, mo, d), (2024, 3, 15));
        assert_eq!((h, mi, s), (12, 30, 0));
    }

    #[test]
    fn test_default_results_path() {
        let p = Path::new("/some/dir/helper.yml");
        let r = default_results_path(p);
        assert_eq!(r, PathBuf::from("/some/dir/helper-results.yml"));
    }
}
