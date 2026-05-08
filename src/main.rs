#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

const INIT_REPORT_PATH: &str = "meta/init-interface-report.md";
const MERGE_REPORT_PATH: &str = "meta/merge-interface-report.md";
const SCAN_OUTPUT_FILE: &str = "test-scan-report.md";
const COVERAGE_OUTPUT_FILE: &str = "coverage-report.md";

#[derive(Parser)]
#[command(
    name = "c2rust-tests-helper",
    about = "Interface and test coverage helper for c2rust-demo translation outputs",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all external interface symbols parsed from interface report.
    Interface {
        /// Path to interface report file. Defaults to auto fallback between init and merge reports.
        #[arg(long, short = 'r')]
        report: Option<PathBuf>,
    },
    /// Scan Rust tests and classify as ST / DT, then match interface symbols.
    Scan {
        /// Path to interface report file. Defaults to auto fallback between init and merge reports.
        #[arg(long, short = 'r')]
        report: Option<PathBuf>,
        /// Root directory to recursively scan for Rust tests.
        #[arg(long = "dir", short = 'd', default_value = ".")]
        rust_root: PathBuf,
        /// Output file path for scan report.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Print ST/DT coverage matrix for each interface symbol in Markdown.
    Coverage {
        /// Path to interface report file. Defaults to auto fallback between init and merge reports.
        #[arg(long, short = 'r')]
        report: Option<PathBuf>,
        /// Root directory to recursively scan for Rust tests.
        #[arg(long = "dir", short = 'd', default_value = ".")]
        rust_root: PathBuf,
        /// Output file path for coverage report.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e:?}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Interface { report } => {
            let report_path = resolve_report_path(report)?;
            cmd_interface(&report_path)
        }
        Command::Scan {
            report,
            rust_root,
            output,
        } => {
            let report_path = resolve_report_path(report)?;
            let output_path = resolve_output_path(&report_path, output, SCAN_OUTPUT_FILE);
            cmd_scan(&report_path, &rust_root, &output_path)
        }
        Command::Coverage {
            report,
            rust_root,
            output,
        } => {
            let report_path = resolve_report_path(report)?;
            let output_path = resolve_output_path(&report_path, output, COVERAGE_OUTPUT_FILE);
            cmd_coverage(&report_path, &rust_root, &output_path)
        }
    }
}

fn resolve_report_path(report: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = report {
        return Ok(path);
    }
    let current_dir = std::env::current_dir().context("reading current working directory")?;
    resolve_default_report_path(&current_dir)
}

fn resolve_default_report_path(current_dir: &Path) -> Result<PathBuf> {
    let init = current_dir.join(INIT_REPORT_PATH);
    if init.exists() {
        return Ok(PathBuf::from(INIT_REPORT_PATH));
    }
    let merge = current_dir.join(MERGE_REPORT_PATH);
    if merge.exists() {
        return Ok(PathBuf::from(MERGE_REPORT_PATH));
    }
    let c2rust_dir = current_dir.join(".c2rust");
    if let Ok(entries) = fs::read_dir(&c2rust_dir) {
        let mut feature_dirs = entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| match entry.file_type() {
                Ok(file_type) if file_type.is_dir() => Some((entry.file_name(), entry.path())),
                _ => None,
            })
            .collect::<Vec<_>>();
        feature_dirs.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));

        for (_, feature_dir) in &feature_dirs {
            let feature_init = feature_dir.join(INIT_REPORT_PATH);
            if feature_init.exists() {
                return Ok(feature_init);
            }
        }
        for (_, feature_dir) in &feature_dirs {
            let feature_merge = feature_dir.join(MERGE_REPORT_PATH);
            if feature_merge.exists() {
                return Ok(feature_merge);
            }
        }
    }
    anyhow::bail!(
        "interface report not found: tried {}, {}, and .c2rust/<feature>/meta/{{init,merge}}-interface-report.md",
        INIT_REPORT_PATH,
        MERGE_REPORT_PATH
    );
}

fn resolve_output_path(report_path: &Path, output: Option<PathBuf>, default_file_name: &str) -> PathBuf {
    if let Some(path) = output {
        return path;
    }
    report_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(default_file_name)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymbolKind {
    Function,
    Variable,
}

impl SymbolKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Variable => "variable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InterfaceSymbol {
    module: String,
    name: String,
    kind: SymbolKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestKind {
    St,
    Dt,
}

impl TestKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::St => "ST",
            Self::Dt => "DT",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestMatch {
    name: String,
    kind: TestKind,
    file: PathBuf,
    interfaces: Vec<String>,
}

fn cmd_interface(report_path: &Path) -> Result<()> {
    let symbols = load_interface_symbols(report_path)?;
    print_interface_symbols(&symbols);
    Ok(())
}

fn cmd_scan(report_path: &Path, rust_root: &Path, output_path: &Path) -> Result<()> {
    let symbols = load_interface_symbols(report_path)?;
    let tests = scan_rust_tests(rust_root, &symbols)?;
    let report = render_scan_results(&tests);
    print!("{report}");
    write_output(output_path, &report)?;
    Ok(())
}

fn cmd_coverage(report_path: &Path, rust_root: &Path, output_path: &Path) -> Result<()> {
    let symbols = load_interface_symbols(report_path)?;
    let tests = scan_rust_tests(rust_root, &symbols)?;
    let report = render_coverage_matrix(&symbols, &tests);
    print!("{report}");
    write_output(output_path, &report)?;
    Ok(())
}

fn load_interface_symbols(report_path: &Path) -> Result<Vec<InterfaceSymbol>> {
    let content = fs::read_to_string(report_path)
        .with_context(|| format!("reading interface report {}", report_path.display()))?;
    parse_interface_report(&content)
        .with_context(|| format!("parsing interface report {}", report_path.display()))
}

fn parse_interface_report(content: &str) -> Result<Vec<InterfaceSymbol>> {
    let first_non_empty_line = content.lines().find(|line| !line.trim().is_empty());
    if let Some(line) = first_non_empty_line {
        if line.trim().starts_with("# Merge Interface Report") {
            return parse_merge_report(content);
        }
    }
    parse_init_report(content)
}

fn parse_init_report(content: &str) -> Result<Vec<InterfaceSymbol>> {
    let section_re = Regex::new(r"^##\s+(.+)$")?;
    let symbol_re = Regex::new(r"^###\s+`([^`]+)`\s+\((function|variable)\)\s*$")?;
    let mut symbols = Vec::new();
    let mut current_module: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(caps) = section_re.captures(trimmed) {
            let module_name = caps[1].trim();
            let lowered = module_name.to_ascii_lowercase();
            let normalized = lowered.replace('`', "");
            if normalized == "summary"
                || normalized == "lib.rs"
                || normalized.starts_with("lib.rs ")
            {
                current_module = None;
                continue;
            }
            current_module = Some(module_name.to_string());
            continue;
        }
        if let Some(caps) = symbol_re.captures(trimmed) {
            if let Some(module) = &current_module {
                let kind = match &caps[2] {
                    "function" => SymbolKind::Function,
                    "variable" => SymbolKind::Variable,
                    _ => continue,
                };
                symbols.push(InterfaceSymbol {
                    module: module.clone(),
                    name: caps[1].trim().to_string(),
                    kind,
                });
            }
        }
    }

    symbols.sort_by(|a, b| {
        a.module
            .cmp(&b.module)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.kind.as_str().cmp(b.kind.as_str()))
    });
    Ok(symbols)
}

fn parse_merge_report(content: &str) -> Result<Vec<InterfaceSymbol>> {
    let section_re = Regex::new(r"^##\s+(.+)$")?;
    let sub_section_re = Regex::new(r"^###\s+(.+)$")?;
    let list_symbol_re = Regex::new(r"^- `([^`]+)`\s*$")?;
    let mut symbols = Vec::new();
    let mut current_module: Option<String> = None;
    let mut current_sub_section: Option<SymbolKind> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "*(none)*" {
            continue;
        }
        if let Some(caps) = section_re.captures(trimmed) {
            let section_name = caps[1].trim();
            let lowered = section_name.to_ascii_lowercase().replace('`', "");
            if lowered == "summary" {
                current_module = None;
                current_sub_section = None;
                continue;
            }
            if lowered.starts_with("lib.rs") {
                current_module = Some("lib.rs".to_string());
                current_sub_section = Some(SymbolKind::Function);
                continue;
            }
            current_module = Some(section_name.to_string());
            current_sub_section = None;
            continue;
        }
        if let Some(caps) = sub_section_re.captures(trimmed) {
            let sub_section_name = caps[1].trim().to_ascii_lowercase();
            current_sub_section = match sub_section_name.as_str() {
                "final rust functions" => Some(SymbolKind::Function),
                "final rust variables" => Some(SymbolKind::Variable),
                // These merge report subsections are metadata and do not define exported symbols.
                "module-local ffi" | "source files merged" => None,
                _ => current_sub_section,
            };
            continue;
        }
        if let Some(caps) = list_symbol_re.captures(trimmed) {
            if let (Some(module), Some(kind)) = (&current_module, current_sub_section) {
                symbols.push(InterfaceSymbol {
                    module: module.clone(),
                    name: caps[1].trim().to_string(),
                    kind,
                });
            }
        }
    }

    symbols.sort_by(|a, b| {
        a.module
            .cmp(&b.module)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.kind.as_str().cmp(b.kind.as_str()))
    });
    Ok(symbols)
}

fn print_interface_symbols(symbols: &[InterfaceSymbol]) {
    println!("| module | symbol | kind |");
    println!("|---|---|---|");
    for symbol in symbols {
        println!(
            "| {} | {} | {} |",
            symbol.module,
            symbol.name,
            symbol.kind.as_str()
        );
    }
}

fn scan_rust_tests(rust_root: &Path, symbols: &[InterfaceSymbol]) -> Result<Vec<TestMatch>> {
    let mut rs_files = Vec::new();
    collect_rs_files(rust_root, &mut rs_files)?;
    rs_files.sort();

    let fn_re = Regex::new(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")?;
    let mut tests = Vec::new();

    for file in rs_files {
        let content = fs::read_to_string(&file)
            .with_context(|| format!("reading Rust source {}", file.display()))?;
        let mut lines = content.lines().peekable();
        let mut has_test_attr = false;

        while let Some(line) = lines.next() {
            let trimmed = line.trim();
            if trimmed == "#[test]" {
                has_test_attr = true;
                continue;
            }
            if has_test_attr && trimmed.starts_with("#[") {
                continue;
            }
            if has_test_attr && trimmed.contains("fn ") {
                if let Some(caps) = fn_re.captures(trimmed) {
                    let name = caps[1].to_string();
                    let mut body = String::new();
                    let mut brace_depth = 0_usize;

                    if let Some(open_idx) = line.find('{') {
                        brace_depth = 1;
                        body.push_str(&line[open_idx + 1..]);
                        body.push('\n');
                    }

                    while brace_depth > 0 {
                        let Some(next_line) = lines.next() else {
                            break;
                        };
                        let opens = next_line.chars().filter(|c| *c == '{').count();
                        let closes = next_line.chars().filter(|c| *c == '}').count();
                        brace_depth = brace_depth.saturating_add(opens).saturating_sub(closes);
                        body.push_str(next_line);
                        body.push('\n');
                    }

                    let kind = classify_test(&file);
                    let interfaces = match_interfaces(&format!("{name}\n{body}"), symbols);
                    tests.push(TestMatch {
                        name,
                        kind,
                        file: file.clone(),
                        interfaces,
                    });
                }
                has_test_attr = false;
                continue;
            }
            if has_test_attr && !trimmed.is_empty() {
                has_test_attr = false;
            }
        }
    }

    tests.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.file.cmp(&b.file)));
    Ok(tests)
}

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if root.is_file() {
        if root.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(root.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(root).with_context(|| format!("reading dir {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some("target")
                || path.file_name().and_then(|n| n.to_str()) == Some(".git")
            {
                continue;
            }
            collect_rs_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn classify_test(file: &Path) -> TestKind {
    let path = file.to_string_lossy().to_ascii_lowercase();
    if path.contains("/tests/") || path.contains("\\tests\\") {
        TestKind::St
    } else {
        TestKind::Dt
    }
}

fn match_interfaces(test_source: &str, symbols: &[InterfaceSymbol]) -> Vec<String> {
    let mut matched = Vec::new();
    for symbol in symbols {
        let pattern = format!(r"\b{}\b", regex::escape(&symbol.name));
        if let Ok(re) = Regex::new(&pattern) {
            if re.is_match(test_source) {
                matched.push(symbol.name.clone());
            }
        }
    }
    matched.sort();
    matched.dedup();
    matched
}

fn render_scan_results(tests: &[TestMatch]) -> String {
    let mut output = String::from("# Test Scan Report\n\n## Test Interface Matches\n\n| test | type | file | interfaces |\n|---|---|---|---|\n");
    for test in tests {
        output.push_str(&format!(
            "| {} | {} | {} | {} |",
            test.name,
            test.kind.as_str(),
            test.file.display(),
            if test.interfaces.is_empty() {
                "-".to_string()
            } else {
                test.interfaces.join(", ")
            }
        ));
        output.push('\n');
    }
    output
}

fn render_coverage_matrix(symbols: &[InterfaceSymbol], tests: &[TestMatch]) -> String {
    let mut output = String::from(
        "# Coverage Report\n\n## Coverage Matrix\n\n| interface | kind | ST | DT |\n|---|---|---|---|\n",
    );
    let mut uncovered = Vec::new();
    let mut covered_by_st = 0_usize;
    let mut covered_by_dt = 0_usize;
    let mut covered_by_any = 0_usize;
    for symbol in symbols {
        let mut st = Vec::new();
        let mut dt = Vec::new();
        for test in tests {
            if test.interfaces.iter().any(|name| name == &symbol.name) {
                match test.kind {
                    TestKind::St => st.push(test.name.clone()),
                    TestKind::Dt => dt.push(test.name.clone()),
                }
            }
        }
        st.sort();
        dt.sort();
        let st_covered = !st.is_empty();
        let dt_covered = !dt.is_empty();
        if st_covered {
            covered_by_st += 1;
        }
        if dt_covered {
            covered_by_dt += 1;
        }
        if st_covered || dt_covered {
            covered_by_any += 1;
        } else {
            uncovered.push(format!(
                "{} ({})",
                format_interface_name(symbol),
                symbol.kind.as_str()
            ));
        }
        output.push_str(&format!(
            "| {} | {} | {} | {} |",
            format_interface_name(symbol),
            symbol.kind.as_str(),
            if st.is_empty() {
                "❌".to_string()
            } else {
                st.join(", ")
            },
            if dt.is_empty() {
                "❌".to_string()
            } else {
                dt.join(", ")
            }
        ));
        output.push('\n');
    }
    output.push_str("\n## Uncovered Interfaces\n\n");
    if uncovered.is_empty() {
        output.push_str("- *(none)*\n");
    } else {
        for item in uncovered {
            output.push_str(&format!("- `{item}`\n"));
        }
    }
    output.push_str("\n## Summary\n\n| metric | value |\n|---|---|\n");
    output.push_str(&format!("| total interfaces | {} |\n", symbols.len()));
    output.push_str(&format!("| covered by ST | {} |\n", covered_by_st));
    output.push_str(&format!("| covered by DT | {} |\n", covered_by_dt));
    output.push_str(&format!("| covered (ST or DT) | {} |\n", covered_by_any));
    output.push_str(&format!(
        "| uncovered | {} |\n",
        symbols.len().saturating_sub(covered_by_any)
    ));
    output
}

fn format_interface_name(symbol: &InterfaceSymbol) -> String {
    format!("{}::{}", symbol.module, symbol.name)
}

fn write_output(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating output directory {}", parent.display()))?;
        }
    }
    fs::write(path, content).with_context(|| format!("writing output {}", path.display()))?;
    eprintln!("written: {}", path.display());
    Ok(())
}
