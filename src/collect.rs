/// `collect` subcommand – walk C source files, extract test names via regex,
/// and merge new entries into the manifest.
use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use regex::Regex;
use walkdir::WalkDir;

use crate::manifest::{Manifest, TestEntry};

pub fn run(config_path: &Path) -> Result<()> {
    let mut manifest = Manifest::load(config_path)?;

    // Resolve the base directory (directory containing the config file).
    let base_dir = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."));

    // Build compiled regexes once.
    let compiled: Vec<Regex> = manifest
        .config
        .discovery
        .patterns
        .iter()
        .map(|p| Regex::new(&p.regex).expect("regex already validated"))
        .collect();

    let extensions: HashSet<String> = manifest
        .config
        .discovery
        .extensions
        .iter()
        .map(|e| e.trim_start_matches('.').to_lowercase())
        .collect();

    // Build set of already-known (name, source_file) pairs so we don't add
    // duplicates.
    let known: HashSet<(String, String)> = manifest
        .tests
        .iter()
        .map(|e| (e.name.clone(), e.source_file.clone()))
        .collect();

    let mut discovered: Vec<TestEntry> = Vec::new();

    for search_path in &manifest.config.discovery.paths.clone() {
        let full_path = base_dir.join(search_path);
        for entry in WalkDir::new(&full_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();

            // Check extension.
            if let Some(ext) = file_path.extension() {
                if !extensions.contains(&ext.to_string_lossy().to_lowercase()) {
                    continue;
                }
            } else {
                continue;
            }

            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "{} skipping '{}': {}",
                        "warning:".yellow(),
                        file_path.display(),
                        e
                    );
                    continue;
                }
            };

            // Make the stored source_file relative to base_dir when possible.
            let relative_path = file_path
                .strip_prefix(base_dir)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| file_path.to_string_lossy().to_string());

            for re in &compiled {
                for caps in re.captures_iter(&content) {
                    if let Some(m) = caps.get(1) {
                        let name = m.as_str().to_string();
                        let key = (name.clone(), relative_path.clone());
                        if !known.contains(&key) {
                            discovered.push(TestEntry::new(name, relative_path.clone()));
                        }
                    }
                }
            }
        }
    }

    let count = discovered.len();
    manifest.tests.extend(discovered);

    // Sort for stable diffs: by source_file then name.
    manifest
        .tests
        .sort_by(|a, b| a.source_file.cmp(&b.source_file).then(a.name.cmp(&b.name)));

    manifest.save(config_path)?;

    if count == 0 {
        println!("{} No new C test entries found.", "collect:".cyan().bold());
    } else {
        println!(
            "{} Added {} new C test {} to the manifest.",
            "collect:".cyan().bold(),
            count.to_string().green().bold(),
            if count == 1 { "entry" } else { "entries" }
        );
    }
    println!(
        "         Total tracked entries: {}",
        manifest.tests.len().to_string().bold()
    );

    Ok(())
}
