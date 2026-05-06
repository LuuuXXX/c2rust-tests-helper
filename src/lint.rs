use anyhow::{bail, Result};

use crate::config::{Config, TestStatus};
use crate::feature::index::FeatureIndex;

/// Validate all `TestEntry` items in `cfg` against the loaded `index`.
///
/// Checks performed for every entry:
/// - `selected_file` (if set) must appear in `index.selected_files`
/// - `module` (if set) must appear in `index.modules`
/// - each symbol in `symbols` must exist in the named module's surface
/// - an entry with `symbols` must also specify a `module`
/// - `ported` entries must have at least one `rust_tests` item
/// - `skipped` entries must include `notes`
///
/// All errors are collected before returning so the caller sees every problem
/// at once rather than stopping at the first one.
pub fn lint(cfg: &Config, index: &FeatureIndex) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    for entry in &cfg.tests {
        let id = &entry.c_test;

        // ── selected_file ──────────────────────────────────────────────────────
        if let Some(sf) = &entry.selected_file {
            if !index.selected_files.contains(sf) {
                errors.push(format!(
                    "[{id}] selected_file {sf:?} not found in feature surface \
                     (meta/selected_files.json)"
                ));
            }
        }

        // ── module + symbols ───────────────────────────────────────────────────
        let resolved_module = entry.module.as_deref().and_then(|mod_name| {
            let found = index.modules.iter().find(|m| m.name == mod_name);
            if found.is_none() {
                errors.push(format!(
                    "[{id}] module {mod_name:?} not found in feature surface"
                ));
            }
            found
        });

        if !entry.symbols.is_empty() {
            if entry.module.is_none() {
                errors.push(format!(
                    "[{id}] has symbols {:?} but no module specified; \
                     add a `module` field",
                    entry.symbols
                ));
            } else if let Some(m) = resolved_module {
                // All symbols in the entry must exist in that module.
                let all: Vec<&str> = m
                    .functions
                    .iter()
                    .chain(m.decls.iter())
                    .chain(m.vars.iter())
                    .map(String::as_str)
                    .collect();
                for sym in &entry.symbols {
                    if !all.contains(&sym.as_str()) {
                        errors.push(format!(
                            "[{id}] symbol {sym:?} not found in module {:?} \
                             (known: {:?})",
                            m.name, all
                        ));
                    }
                }
            }
        }

        // ── status constraints ────────────────────────────────────────────────
        if entry.status == TestStatus::Ported && entry.rust_tests.is_empty() {
            errors.push(format!(
                "[{id}] status is 'ported' but rust_tests is empty; \
                 add at least one Rust test name"
            ));
        }

        if entry.status == TestStatus::Skipped && entry.notes.is_none() {
            errors.push(format!(
                "[{id}] status is 'skipped' but notes is missing; \
                 explain why this test is skipped"
            ));
        }
    }

    if errors.is_empty() {
        println!("lint: OK ({} entries checked)", cfg.tests.len());
        Ok(())
    } else {
        for e in &errors {
            eprintln!("lint error: {e}");
        }
        bail!("{} lint error(s) found", errors.len());
    }
}
