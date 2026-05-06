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

/// Per-language test commands used by `check` / `run` subcommands (PR3+).
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct TestCommands {
    /// Command to run the C test suite (e.g. `"make test"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub c: Option<String>,
    /// Command to run the translated Rust test suite (e.g. `"cargo test"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust: Option<String>,
    /// Command to run the feature-specific Rust test suite, when it differs from `rust`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_rust: Option<String>,
}

/// Minimal description of a single test case.
///
/// Populated by later PRs; kept typed here so the schema is stable.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct TestEntry {
    /// Identifier for this test (e.g. the C function name).
    pub name: String,
    /// Path to the C test source relative to `project.root`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub c_test: Option<String>,
    /// Path to the ported Rust test relative to `project.root`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust_test: Option<String>,
    /// c2rust API symbols exercised by this test.
    #[serde(default)]
    pub symbols: Vec<String>,
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
