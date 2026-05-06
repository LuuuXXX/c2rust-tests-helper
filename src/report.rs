use std::collections::HashSet;

use crate::config::{Config, TestStatus};
use crate::feature::index::FeatureIndex;

/// Print a migration-status and feature-coverage report to stdout.
pub fn report(cfg: &Config, index: &FeatureIndex) {
    print_status_summary(cfg);
    print_surface_totals(index);
    print_coverage(cfg, index);
    print_gaps(cfg, index);
}

// ── migration status ──────────────────────────────────────────────────────────

fn print_status_summary(cfg: &Config) {
    let total = cfg.tests.len();
    let pending = count_status(cfg, TestStatus::Pending);
    let ported = count_status(cfg, TestStatus::Ported);
    let skipped = count_status(cfg, TestStatus::Skipped);
    let na = count_status(cfg, TestStatus::NotApplicable);

    println!("=== Migration Status ===");
    println!("  Total tests      : {total}");
    println!("  pending          : {pending}");
    println!("  ported           : {ported}");
    println!("  skipped          : {skipped}");
    println!("  not_applicable   : {na}");
    if total > 0 {
        let pct = ported as f64 / total as f64 * 100.0;
        println!("  migration done   : {ported}/{total} ({pct:.0}%)");
    }
    println!();
}

fn count_status(cfg: &Config, status: TestStatus) -> usize {
    cfg.tests.iter().filter(|t| t.status == status).count()
}

// ── feature surface totals ────────────────────────────────────────────────────

fn print_surface_totals(index: &FeatureIndex) {
    let total_files = index.selected_files.len();
    let total_modules = index.modules.len();
    let total_symbols = count_surface_symbols(index);

    println!("=== Feature Surface ===");
    println!("  Selected files : {total_files}");
    println!("  Modules        : {total_modules}");
    println!("  Symbols        : {total_symbols}");
    println!();
}

/// Count the total number of symbols (functions + decls + vars) across all modules.
fn count_surface_symbols(index: &FeatureIndex) -> usize {
    index
        .modules
        .iter()
        .map(|m| m.functions.len() + m.decls.len() + m.vars.len())
        .sum()
}

/// Build the complete set of `"module/symbol"` keys that exist in the surface.
fn all_surface_symbol_keys(index: &FeatureIndex) -> HashSet<String> {
    index
        .modules
        .iter()
        .flat_map(|m| {
            m.functions
                .iter()
                .chain(m.decls.iter())
                .chain(m.vars.iter())
                .map(move |sym| format!("{}/{sym}", m.name))
        })
        .collect()
}

// ── mapping coverage ──────────────────────────────────────────────────────────

fn print_coverage(cfg: &Config, index: &FeatureIndex) {
    // Build surface membership sets for fast lookup.
    let surface_files: HashSet<&str> =
        index.selected_files.iter().map(String::as_str).collect();
    let surface_modules: HashSet<&str> =
        index.modules.iter().map(|m| m.name.as_str()).collect();
    let surface_symbol_keys = all_surface_symbol_keys(index);

    // Coverage numerators: only manifest references that actually exist in the surface.
    let covered_files: HashSet<&str> = cfg
        .tests
        .iter()
        .filter_map(|t| t.selected_file.as_deref())
        .filter(|f| surface_files.contains(f))
        .collect();

    let covered_modules: HashSet<&str> = cfg
        .tests
        .iter()
        .filter_map(|t| t.module.as_deref())
        .filter(|m| surface_modules.contains(m))
        .collect();

    let covered_symbols: HashSet<String> = cfg
        .tests
        .iter()
        .filter_map(|t| t.module.as_deref().map(|m| (m, &t.symbols)))
        .flat_map(|(mod_name, syms)| syms.iter().map(move |s| format!("{mod_name}/{s}")))
        .filter(|key| surface_symbol_keys.contains(key))
        .collect();

    let total_files = index.selected_files.len();
    let total_modules = index.modules.len();
    let total_symbols = count_surface_symbols(index);

    println!("=== Mapping Coverage ===");
    println!(
        "  Selected files : {}/{} ({})",
        covered_files.len(),
        total_files,
        pct(covered_files.len(), total_files)
    );
    println!(
        "  Modules        : {}/{} ({})",
        covered_modules.len(),
        total_modules,
        pct(covered_modules.len(), total_modules)
    );
    println!(
        "  Symbols        : {}/{} ({})",
        covered_symbols.len(),
        total_symbols,
        pct(covered_symbols.len(), total_symbols)
    );
    println!();
}

// ── unmapped gaps ─────────────────────────────────────────────────────────────

fn print_gaps(cfg: &Config, index: &FeatureIndex) {
    // Same surface-validated sets used for gap computation.
    let surface_files: HashSet<&str> =
        index.selected_files.iter().map(String::as_str).collect();
    let surface_modules: HashSet<&str> =
        index.modules.iter().map(|m| m.name.as_str()).collect();
    let surface_symbol_keys = all_surface_symbol_keys(index);

    let covered_files: HashSet<&str> = cfg
        .tests
        .iter()
        .filter_map(|t| t.selected_file.as_deref())
        .filter(|f| surface_files.contains(f))
        .collect();

    let covered_modules: HashSet<&str> = cfg
        .tests
        .iter()
        .filter_map(|t| t.module.as_deref())
        .filter(|m| surface_modules.contains(m))
        .collect();

    let covered_symbols: HashSet<String> = cfg
        .tests
        .iter()
        .filter_map(|t| t.module.as_deref().map(|m| (m, &t.symbols)))
        .flat_map(|(mod_name, syms)| syms.iter().map(move |s| format!("{mod_name}/{s}")))
        .filter(|key| surface_symbol_keys.contains(key))
        .collect();

    let unmapped_files: Vec<&str> = index
        .selected_files
        .iter()
        .map(String::as_str)
        .filter(|f| !covered_files.contains(f))
        .collect();

    let unmapped_modules: Vec<&str> = index
        .modules
        .iter()
        .map(|m| m.name.as_str())
        .filter(|m| !covered_modules.contains(m))
        .collect();

    let unmapped_symbol_keys: Vec<&str> = surface_symbol_keys
        .iter()
        .map(String::as_str)
        .filter(|k| !covered_symbols.contains(*k))
        .collect();

    println!("=== Unmapped Gaps ===");

    if unmapped_files.is_empty() {
        println!("  Selected files : (none – all mapped)");
    } else {
        println!(
            "  Selected files with no mapped tests ({}):",
            unmapped_files.len()
        );
        let mut sorted = unmapped_files.clone();
        sorted.sort_unstable();
        for f in &sorted {
            println!("    {f}");
        }
    }

    if unmapped_modules.is_empty() {
        println!("  Modules        : (none – all mapped)");
    } else {
        println!(
            "  Modules with no mapped tests ({}):",
            unmapped_modules.len()
        );
        let mut sorted = unmapped_modules.clone();
        sorted.sort_unstable();
        for m in &sorted {
            println!("    {m}");
        }
    }

    if unmapped_symbol_keys.is_empty() {
        println!("  Symbols        : (none – all mapped)");
    } else {
        println!(
            "  Symbols with no mapped tests ({}):",
            unmapped_symbol_keys.len()
        );
        let mut sorted = unmapped_symbol_keys.clone();
        sorted.sort_unstable();
        for s in &sorted {
            println!("    {s}");
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn pct(num: usize, den: usize) -> String {
    if den == 0 {
        "N/A".to_owned()
    } else {
        format!("{:.0}%", num as f64 / den as f64 * 100.0)
    }
}
