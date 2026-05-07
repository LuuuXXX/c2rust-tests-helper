use std::path::Path;
use std::process::Command;

use crate::config::Config;
use crate::feature::index::FeatureIndex;

/// Result of a single check step.
#[derive(Debug, Clone, PartialEq)]
pub enum StepResult {
    Passed,
    Failed,
    Skipped,
}

impl std::fmt::Display for StepResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepResult::Passed => write!(f, "passed"),
            StepResult::Failed => write!(f, "failed"),
            StepResult::Skipped => write!(f, "skipped"),
        }
    }
}

/// Aggregated results from a `check` run.
pub struct CheckSummary {
    pub lint: StepResult,
    pub c_tests: StepResult,
    pub rust_tests: StepResult,
    pub feature_rust: Option<StepResult>,
}

impl CheckSummary {
    /// Returns `true` if no step failed (skipped steps are not failures).
    pub fn overall_passed(&self) -> bool {
        self.lint != StepResult::Failed
            && self.c_tests != StepResult::Failed
            && self.rust_tests != StepResult::Failed
            && self.feature_rust != Some(StepResult::Failed)
    }
}

/// Run the full check workflow and return a summary.
///
/// Execution order:
/// 1. `lint` – validate the manifest against the feature surface.
/// 2. If lint fails, skip all test commands and return immediately.
/// 3. Run each configured test command from `project_root`.
/// 4. Unconfigured `c` commands are reported as `skipped`.
/// 5. Unconfigured `rust` commands are reported as `skipped`.
/// 6. Optional legacy `feature_rust` is only included when configured.
pub fn run(cfg: &Config, index: &FeatureIndex, project_root: &Path) -> CheckSummary {
    // ── step 1: lint ──────────────────────────────────────────────────────────
    let lint_result = match crate::lint::lint(cfg, index) {
        Ok(()) => StepResult::Passed,
        Err(_) => StepResult::Failed,
    };

    // ── step 2: bail early if lint failed ────────────────────────────────────
    if lint_result == StepResult::Failed {
        eprintln!("check: lint failed, skipping all configured test commands");
        return CheckSummary {
            lint: lint_result,
            c_tests: StepResult::Skipped,
            rust_tests: StepResult::Skipped,
            feature_rust: cfg
                .test_commands
                .feature_rust
                .as_deref()
                .map(|_| StepResult::Skipped),
        };
    }

    // ── step 3: run configured test commands ─────────────────────────────────
    let c_tests = run_optional_command(cfg.test_commands.c.as_deref(), project_root);
    let rust_tests = run_optional_command(cfg.test_commands.rust.as_deref(), project_root);
    let feature_rust = cfg
        .test_commands
        .feature_rust
        .as_deref()
        .map(|cmd| execute_shell_command(cmd, project_root));

    CheckSummary {
        lint: lint_result,
        c_tests,
        rust_tests,
        feature_rust,
    }
}

/// Print the final summary to stdout.
pub fn print_summary(summary: &CheckSummary) {
    let overall = if summary.overall_passed() {
        StepResult::Passed
    } else {
        StepResult::Failed
    };

    println!();
    println!("=== Check ===");
    println!("lint          : {}", summary.lint);
    println!("c tests       : {}", summary.c_tests);
    println!("rust tests    : {}", summary.rust_tests);
    if let Some(feature_rust) = &summary.feature_rust {
        println!("feature_rust  : {feature_rust}");
    }
    println!();
    println!("overall       : {overall}");
}

// ── internal helpers ──────────────────────────────────────────────────────────

fn run_optional_command(cmd: Option<&str>, cwd: &Path) -> StepResult {
    match cmd {
        None => StepResult::Skipped,
        Some(cmd_str) => execute_shell_command(cmd_str, cwd),
    }
}

/// Execute `cmd_str` via the system shell (`sh -c`) from `cwd`, streaming
/// output to the caller's terminal.  Returns `Passed` on exit code 0,
/// `Failed` otherwise.
///
/// **Requirement**: a POSIX-compatible shell (`sh`) must be present on `PATH`.
fn execute_shell_command(cmd_str: &str, cwd: &Path) -> StepResult {
    println!("$ {cmd_str}");

    let result = Command::new("sh")
        .arg("-c")
        .arg(cmd_str)
        .current_dir(cwd)
        .status();

    match result {
        Ok(status) if status.success() => StepResult::Passed,
        Ok(status) => {
            eprintln!(
                "command exited with code {}",
                status.code().unwrap_or(-1)
            );
            StepResult::Failed
        }
        Err(e) => {
            eprintln!(
                "failed to start `sh -c {cmd_str:?}`: {e}\n\
                 hint: ensure a POSIX-compatible shell (sh) is available on PATH"
            );
            StepResult::Failed
        }
    }
}
