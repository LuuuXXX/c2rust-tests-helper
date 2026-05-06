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
    let total_symbols: usize = index
        .modules
        .iter()
        .map(|m| m.functions.len() + m.decls.len() + m.vars.len())
        .sum();

    println!("=== Feature Surface ===");
    println!("  Selected files : {total_files}");
    println!("  Modules        : {total_modules}");
    println!("  Symbols        : {total_symbols}");
    println!();
}

// ── mapping coverage ──────────────────────────────────────────────────────────

fn print_coverage(cfg: &Config, index: &FeatureIndex) {
    let referenced_files: HashSet<&str> = cfg
        .tests
        .iter()
        .filter_map(|t| t.selected_file.as_deref())
        .collect();

    let referenced_modules: HashSet<&str> = cfg
        .tests
        .iter()
        .filter_map(|t| t.module.as_deref())
        .collect();

    let referenced_symbols: HashSet<&str> = cfg
        .tests
        .iter()
        .flat_map(|t| t.symbols.iter().map(String::as_str))
        .collect();

    let total_files = index.selected_files.len();
    let total_modules = index.modules.len();
    let total_symbols: usize = index
        .modules
        .iter()
        .map(|m| m.functions.len() + m.decls.len() + m.vars.len())
        .sum();

    println!("=== Mapping Coverage ===");
    println!(
        "  Selected files : {}/{} ({})",
        referenced_files.len(),
        total_files,
        pct(referenced_files.len(), total_files)
    );
    println!(
        "  Modules        : {}/{} ({})",
        referenced_modules.len(),
        total_modules,
        pct(referenced_modules.len(), total_modules)
    );
    println!(
        "  Symbols        : {}/{} ({})",
        referenced_symbols.len(),
        total_symbols,
        pct(referenced_symbols.len(), total_symbols)
    );
    println!();
}

// ── unmapped gaps ─────────────────────────────────────────────────────────────

fn print_gaps(cfg: &Config, index: &FeatureIndex) {
    let referenced_files: HashSet<&str> = cfg
        .tests
        .iter()
        .filter_map(|t| t.selected_file.as_deref())
        .collect();

    let referenced_modules: HashSet<&str> = cfg
        .tests
        .iter()
        .filter_map(|t| t.module.as_deref())
        .collect();

    let referenced_symbols: HashSet<String> = cfg
        .tests
        .iter()
        .flat_map(|t| {
            let mod_name = t.module.as_deref().unwrap_or("");
            t.symbols
                .iter()
                .map(move |s| format!("{mod_name}/{s}"))
        })
        .collect();

    let unmapped_files: Vec<&str> = index
        .selected_files
        .iter()
        .map(String::as_str)
        .filter(|f| !referenced_files.contains(f))
        .collect();

    let unmapped_modules: Vec<&str> = index
        .modules
        .iter()
        .map(|m| m.name.as_str())
        .filter(|m| !referenced_modules.contains(m))
        .collect();

    let unmapped_symbols: Vec<String> = index
        .modules
        .iter()
        .flat_map(|m| {
            m.functions
                .iter()
                .chain(m.decls.iter())
                .chain(m.vars.iter())
                .map(move |sym| format!("{}/{sym}", m.name))
        })
        .filter(|key| !referenced_symbols.contains(key))
        .collect();

    println!("=== Unmapped Gaps ===");

    if unmapped_files.is_empty() {
        println!("  Selected files : (none – all mapped)");
    } else {
        println!("  Selected files with no mapped tests ({}):", unmapped_files.len());
        for f in &unmapped_files {
            println!("    {f}");
        }
    }

    if unmapped_modules.is_empty() {
        println!("  Modules        : (none – all mapped)");
    } else {
        println!("  Modules with no mapped tests ({}):", unmapped_modules.len());
        for m in &unmapped_modules {
            println!("    {m}");
        }
    }

    if unmapped_symbols.is_empty() {
        println!("  Symbols        : (none – all mapped)");
    } else {
        println!("  Symbols with no mapped tests ({}):", unmapped_symbols.len());
        for s in &unmapped_symbols {
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
