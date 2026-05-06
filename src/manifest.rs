/// Data structures and I/O for the unified manifest / config YAML file.
///
/// A single file (e.g. `helper.yml`) contains:
///   - `config`  – discovery patterns, test commands
///   - `tests`   – list of discovered/tracked C-to-Rust migration entries
///
/// Why one file?  Keeping config and the migration table together means a
/// single `git diff helper.yml` shows every change to both policy and state,
/// which is ideal for review-oriented workflows.
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Status / category
// ---------------------------------------------------------------------------

/// Migration status of a single C test entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    /// No Rust counterpart has been written yet.
    #[default]
    Pending,
    /// A Rust test has been written and the entry is considered ported.
    Ported,
    /// Intentionally not ported (e.g. tests internal-only code not reachable
    /// through the public FFI surface).
    Skipped,
    /// Not applicable (e.g. helper / setup functions, not real tests).
    NotApplicable,
}

impl std::fmt::Display for MigrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Ported => write!(f, "ported"),
            Self::Skipped => write!(f, "skipped"),
            Self::NotApplicable => write!(f, "n/a"),
        }
    }
}

// ---------------------------------------------------------------------------
// Individual test entry
// ---------------------------------------------------------------------------

/// Represents one discovered C test and its current migration state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEntry {
    /// Name of the C test function / test case identifier.
    pub name: String,
    /// Path to the C source file where the test was discovered.
    pub source_file: String,
    /// Migration / category status.
    #[serde(default)]
    pub status: MigrationStatus,
    /// Names of the Rust tests that cover this C test (may be empty).
    #[serde(default)]
    pub rust_tests: Vec<String>,
    /// Free-form notes: public-contract rationale, non-1:1 mapping explanation, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl TestEntry {
    pub fn new(name: impl Into<String>, source_file: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source_file: source_file.into(),
            status: MigrationStatus::Pending,
            rust_tests: Vec::new(),
            notes: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Discovery configuration
// ---------------------------------------------------------------------------

/// A single regex pattern used to extract test names from C source files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryPattern {
    /// A regex with at least one capture group; the first capture group is
    /// used as the test name.
    pub regex: String,
    /// Human-readable label for the C framework / convention this matches
    /// (e.g. "unity", "cmocka", "custom").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
}

/// Configuration for C test source discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Directories (relative to the config file) to walk when searching for
    /// C test sources.
    pub paths: Vec<String>,
    /// File glob extensions to consider (defaults to `["c"]`).
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
    /// Ordered list of regex patterns to try on each file.
    pub patterns: Vec<DiscoveryPattern>,
}

fn default_extensions() -> Vec<String> {
    vec!["c".into()]
}

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

/// Tool-level configuration embedded in the manifest file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// C test discovery settings.
    pub discovery: DiscoveryConfig,
    /// Shell command used to run the C test suite.
    pub c_test_command: String,
    /// Shell command used to run the Rust test suite.
    pub rust_test_command: String,
    /// Optional working directory for running test commands (defaults to the
    /// directory containing the manifest file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    /// Additional environment variables forwarded to both test commands.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Full manifest
// ---------------------------------------------------------------------------

/// The complete on-disk manifest / config document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub config: Config,
    /// All tracked C test entries.
    #[serde(default)]
    pub tests: Vec<TestEntry>,
}

impl Manifest {
    /// Load a manifest from a YAML file, returning a rich error on failure.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("cannot read manifest file '{}'", path.display()))?;
        let manifest: Self = serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse manifest file '{}'", path.display()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Save the manifest back to a YAML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)
            .context("failed to serialize manifest to YAML")?;
        fs::write(path, content)
            .with_context(|| format!("cannot write manifest file '{}'", path.display()))?;
        Ok(())
    }

    /// Basic structural validation – fails loudly on bad data.
    pub fn validate(&self) -> Result<()> {
        if self.config.c_test_command.trim().is_empty() {
            anyhow::bail!("manifest validation error: `config.c_test_command` must not be empty");
        }
        if self.config.rust_test_command.trim().is_empty() {
            anyhow::bail!(
                "manifest validation error: `config.rust_test_command` must not be empty"
            );
        }
        if self.config.discovery.paths.is_empty() {
            anyhow::bail!(
                "manifest validation error: `config.discovery.paths` must not be empty"
            );
        }
        if self.config.discovery.patterns.is_empty() {
            anyhow::bail!(
                "manifest validation error: `config.discovery.patterns` must not be empty"
            );
        }
        for (i, pat) in self.config.discovery.patterns.iter().enumerate() {
            regex::Regex::new(&pat.regex).with_context(|| {
                format!(
                    "manifest validation error: discovery pattern [{}] has invalid regex: '{}'",
                    i, pat.regex
                )
            })?;
        }
        Ok(())
    }

    /// Return counts by status for summary reporting.
    pub fn status_counts(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for entry in &self.tests {
            *counts.entry(entry.status.to_string()).or_insert(0) += 1;
        }
        counts
    }
}

// ---------------------------------------------------------------------------
// Run results (written by `check`, read by `report`)
// ---------------------------------------------------------------------------

/// Outcome of a single shell command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    /// Captured stdout + stderr (truncated to keep the file small).
    pub output_tail: String,
}

/// Results written by the `check` subcommand to a companion file so that
/// `report` can display them without re-running the tests.
///
/// This second file is justified because running the test suite can be
/// expensive, and users may want to inspect results offline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResults {
    /// RFC-3339 timestamp of when `check` was run.
    pub timestamp: String,
    pub c_result: CommandResult,
    pub rust_result: CommandResult,
}

impl RunResults {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("cannot read results file '{}'", path.display()))?;
        serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse results file '{}'", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)
            .context("failed to serialize run results to YAML")?;
        fs::write(path, content)
            .with_context(|| format!("cannot write results file '{}'", path.display()))?;
        Ok(())
    }
}
