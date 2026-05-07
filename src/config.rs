use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub version: u32,
    pub project: Project,
    pub feature_source: FeatureSource,
    #[serde(default)]
    pub test_commands: TestCommands,
    #[serde(default)]
    pub discovery: Discovery,
    #[serde(default)]
    pub tests: Vec<TestEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    pub root: String,
    pub feature: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FeatureSource {
    pub kind: String,
    pub root: String,
}

/// Test commands used by `verify` / legacy `check`.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct TestCommands {
    /// Command to run the C test suite (e.g. `"make test"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub c: Option<String>,
    /// Command to run the translated Rust test suite (e.g. `"cargo test"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust: Option<String>,
    /// Optional legacy-compatible extra Rust command; prefer putting the exact
    /// Rust test command in `rust`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_rust: Option<String>,
}

/// Migration status of a single C test.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    /// Not yet evaluated (default for newly discovered tests).
    #[default]
    Pending,
    /// The test has been ported to Rust; `rust_tests` must be non-empty.
    Ported,
    /// The test is intentionally skipped; `notes` must explain why.
    Skipped,
    /// The test is not applicable to the Rust port.
    NotApplicable,
}

/// A single test-migration entry.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct TestEntry {
    /// Original C test function name (unique identifier within the manifest).
    pub c_test: String,
    /// Source file path relative to `project.root` where the test was found.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    /// Migration status.
    #[serde(default)]
    pub status: TestStatus,
    /// Selected file (from `meta/selected_files.json`) this test exercises.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_file: Option<String>,
    /// Generated module (e.g. `mod_math`) this test exercises.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    /// Symbols within the module exercised by this test.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<String>,
    /// Rust test names that verify the same behaviour.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rust_tests: Vec<String>,
    /// Brief description of the migration contract / invariant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<String>,
    /// Free-form notes (required for `skipped` entries).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Discovery {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<DiscoveryPattern>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DiscoveryPattern {
    pub regex: String,
    #[serde(default)]
    pub framework: String,
}

pub fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let config: Config = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse config file: {}", path.display()))?;
    Ok(config)
}

/// Serialize `config` and write it back to `path`.
pub fn save_config(path: &Path, config: &Config) -> Result<()> {
    let content = serde_yaml::to_string(config)
        .with_context(|| "failed to serialize config")?;
    std::fs::write(path, content)
        .with_context(|| format!("failed to write config file: {}", path.display()))?;
    Ok(())
}
