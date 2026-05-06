use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::config::{Config, TestEntry};

/// Scan the configured discovery paths under `project.root`, extract test names
/// using the configured regex patterns, and merge any new entries into
/// `cfg.tests`.  Existing entries are left untouched (idempotent).
///
/// Returns the number of newly-added test entries.
pub fn collect_tests(cfg: &mut Config, config_path: &Path) -> Result<usize> {
    let project_root = resolve_path(config_path, &cfg.project.root);

    // Names that are already tracked – used to avoid duplicates.
    let known: HashSet<String> = cfg.tests.iter().map(|t| t.c_test.clone()).collect();

    // Compile all regexes up-front so we bail early on syntax errors.
    let compiled: Vec<Regex> = cfg
        .discovery
        .patterns
        .iter()
        .map(|p| {
            Regex::new(&p.regex)
                .with_context(|| format!("invalid discovery regex: {}", p.regex))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut new_entries: Vec<TestEntry> = Vec::new();

    for path_str in &cfg.discovery.paths {
        let scan_root = project_root.join(path_str);
        if !scan_root.exists() {
            eprintln!(
                "collect: discovery path does not exist, skipping: {}",
                scan_root.display()
            );
            continue;
        }

        for file in walk_files(&scan_root, &cfg.discovery.extensions)? {
            // Relative path from project root (used as source_file).
            let rel_path = file
                .strip_prefix(&project_root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/"); // normalise on Windows

            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;

            for re in &compiled {
                for cap in re.captures_iter(&content) {
                    if let Some(m) = cap.get(1) {
                        let test_name = m.as_str().to_owned();
                        let already_known = known.contains(&test_name);
                        let already_new = new_entries.iter().any(|e| e.c_test == test_name);
                        if !already_known && !already_new {
                            new_entries.push(TestEntry {
                                c_test: test_name,
                                source_file: Some(rel_path.clone()),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
    }

    let added = new_entries.len();
    cfg.tests.extend(new_entries);
    Ok(added)
}

// ── file-system helpers ────────────────────────────────────────────────────────

/// Recursively collect files under `root` whose extension matches any entry in
/// `extensions` (case-insensitive, without the leading dot).
fn walk_files(root: &Path, extensions: &[String]) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    walk_dir(root, extensions, &mut result)?;
    result.sort(); // deterministic ordering
    Ok(result)
}

fn walk_dir(dir: &Path, extensions: &[String], out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory: {}", dir.display()))?;

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, extensions, out)?;
        } else if extensions.is_empty() || has_extension(&path, extensions) {
            out.push(path);
        }
    }
    Ok(())
}

fn has_extension(path: &Path, extensions: &[String]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_lowercase();
            extensions.iter().any(|x| x.to_lowercase() == lower)
        })
        .unwrap_or(false)
}

/// Resolve `p` relative to the directory that contains `config_path`.
/// Absolute paths are returned unchanged.
fn resolve_path(config_path: &Path, p: &str) -> PathBuf {
    let relative_to = config_path.parent().unwrap_or_else(|| Path::new("."));
    let joined = relative_to.join(p);
    joined.canonicalize().unwrap_or(joined)
}
