use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub version: u32,
    pub project: Project,
    pub feature_source: FeatureSource,
    #[serde(default)]
    pub test_commands: HashMap<String, String>,
    #[serde(default)]
    pub discovery: Discovery,
    #[serde(default)]
    pub tests: Vec<serde_yaml::Value>,
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
