#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

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
    /// List all external interface symbols parsed from init-interface-report.md.
    Interface {
        /// Path to interface report file.
        #[arg(long, short = 'r', default_value = "meta/init-interface-report.md")]
        report: PathBuf,
    },
    /// Scan Rust tests and classify as ST / DT, then match interface symbols.
    Scan {
        /// Path to interface report file.
        #[arg(long, short = 'r', default_value = "meta/init-interface-report.md")]
        report: PathBuf,
        /// Root directory to recursively scan for Rust tests.
        #[arg(long = "dir", short = 'd', default_value = ".")]
        rust_root: PathBuf,
    },
    /// Print ST/DT coverage matrix for each interface symbol in Markdown.
    Coverage {
        /// Path to interface report file.
        #[arg(long, short = 'r', default_value = "meta/init-interface-report.md")]
        report: PathBuf,
        /// Root directory to recursively scan for Rust tests.
        #[arg(long = "dir", short = 'd', default_value = ".")]
        rust_root: PathBuf,
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
        Command::Interface { report } => cmd_interface(&report),
        Command::Scan { report, rust_root } => cmd_scan(&report, &rust_root),
        Command::Coverage { report, rust_root } => cmd_coverage(&report, &rust_root),
    }
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

fn cmd_scan(report_path: &Path, rust_root: &Path) -> Result<()> {
    let symbols = load_interface_symbols(report_path)?;
    let tests = scan_rust_tests(rust_root, &symbols)?;
    print_scan_results(&tests);
    Ok(())
}

fn cmd_coverage(report_path: &Path, rust_root: &Path) -> Result<()> {
    let symbols = load_interface_symbols(report_path)?;
    let tests = scan_rust_tests(rust_root, &symbols)?;
    print_coverage_matrix(&symbols, &tests);
    Ok(())
}

fn load_interface_symbols(report_path: &Path) -> Result<Vec<InterfaceSymbol>> {
    let content = fs::read_to_string(report_path)
        .with_context(|| format!("reading interface report {}", report_path.display()))?;
    parse_interface_report(&content)
        .with_context(|| format!("parsing interface report {}", report_path.display()))
}

fn parse_interface_report(content: &str) -> Result<Vec<InterfaceSymbol>> {
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

fn print_scan_results(tests: &[TestMatch]) {
    println!("| test | type | file | interfaces |");
    println!("|---|---|---|---|");
    for test in tests {
        println!(
            "| {} | {} | {} | {} |",
            test.name,
            test.kind.as_str(),
            test.file.display(),
            if test.interfaces.is_empty() {
                "-".to_string()
            } else {
                test.interfaces.join(", ")
            }
        );
    }
}

fn print_coverage_matrix(symbols: &[InterfaceSymbol], tests: &[TestMatch]) {
    println!("| interface | kind | ST | DT |");
    println!("|---|---|---|---|");
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
        println!(
            "| {}::{} | {} | {} | {} |",
            symbol.module,
            symbol.name,
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
        );
    }
}
