#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

const INIT_REPORT_PATH: &str = "meta/init-interface-report.md";
const MERGE_REPORT_PATH: &str = "meta/merge-interface-report.md";
const DISCOVER_OUTPUT_FILE: &str = "c-test-discovery-report.md";
const MAP_OUTPUT_FILE: &str = "c-test-map-report.md";

#[derive(Parser)]
#[command(
    name = "c2rust-tests-helper",
    about = "C test discovery and interface candidate mapping helper for c2rust-demo translation outputs",
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
    /// Scan C source files for test functions and output a discovery report.
    Discover {
        /// Root directory to recursively scan for C test files.
        #[arg(long = "dir", short = 'd', default_value = ".")]
        c_test_root: PathBuf,
        /// Output file path for the discovery report.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Map discovered C tests to candidate interface symbols.
    Map {
        /// Path to interface report file. Defaults to auto fallback between init and merge reports.
        #[arg(long, short = 'r')]
        report: Option<PathBuf>,
        /// Root directory to recursively scan for C test files.
        #[arg(long = "dir", short = 'd', default_value = ".")]
        c_test_root: PathBuf,
        /// Output file path for the mapping report.
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
        Command::Discover { c_test_root, output } => {
            let output_path = resolve_output_path_direct(output, DISCOVER_OUTPUT_FILE);
            cmd_discover(&c_test_root, &output_path)
        }
        Command::Map {
            report,
            c_test_root,
            output,
        } => {
            let report_path = resolve_report_path(report)?;
            let output_path = resolve_output_path(&report_path, output, MAP_OUTPUT_FILE);
            cmd_map(&report_path, &c_test_root, &output_path)
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

fn resolve_output_path_direct(output: Option<PathBuf>, default_file_name: &str) -> PathBuf {
    output.unwrap_or_else(|| PathBuf::from(default_file_name))
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CTestCase {
    name: String,
    file: PathBuf,
    body: String,
}

fn cmd_interface(report_path: &Path) -> Result<()> {
    let symbols = load_interface_symbols(report_path)?;
    print_interface_symbols(&symbols);
    Ok(())
}

fn cmd_discover(c_test_root: &Path, output_path: &Path) -> Result<()> {
    let tests = discover_c_tests(c_test_root)?;
    let report = render_discovery_report(c_test_root, &tests);
    print!("{report}");
    write_output(output_path, &report)?;
    Ok(())
}

fn cmd_map(report_path: &Path, c_test_root: &Path, output_path: &Path) -> Result<()> {
    let symbols = load_interface_symbols(report_path)?;
    let tests = discover_c_tests(c_test_root)?;
    let report = render_map_report(c_test_root, &tests, &symbols);
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

fn discover_c_tests(c_test_root: &Path) -> Result<Vec<CTestCase>> {
    let mut c_files = Vec::new();
    collect_c_files(c_test_root, &mut c_files)?;
    c_files.sort();

    let test_fn_re = Regex::new(r"(?m)^(?:void|int)\s+(test_[A-Za-z0-9_]+)\s*\(")?;
    let mut tests = Vec::new();

    for file in c_files {
        let content = fs::read_to_string(&file)
            .with_context(|| format!("reading C source {}", file.display()))?;
        for caps in test_fn_re.captures_iter(&content) {
            let name = caps[1].to_string();
            let after_match = caps.get(0).unwrap().end();
            let body = extract_function_body(&content[after_match..]);
            tests.push(CTestCase {
                name,
                file: file.clone(),
                body,
            });
        }
    }

    tests.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.file.cmp(&b.file)));
    Ok(tests)
}

fn extract_function_body(src: &str) -> String {
    let mut depth = 0_usize;
    let mut in_body = false;
    let mut body = String::new();
    for ch in src.chars() {
        match ch {
            '{' => {
                depth += 1;
                in_body = true;
                body.push(ch);
            }
            '}' => {
                if depth > 0 {
                    depth -= 1;
                    body.push(ch);
                    if depth == 0 {
                        break;
                    }
                }
            }
            _ => {
                if in_body {
                    body.push(ch);
                }
            }
        }
    }
    body
}

fn collect_c_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if root.is_file() {
        if root.extension().and_then(|e| e.to_str()) == Some("c") {
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
            collect_c_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("c") {
            out.push(path);
        }
    }
    Ok(())
}

fn infer_candidate_symbols(test_source: &str, symbols: &[InterfaceSymbol]) -> Vec<String> {
    let mut candidates = Vec::new();
    for symbol in symbols {
        let pattern = format!(r"\b{}\b", regex::escape(&symbol.name));
        if let Ok(re) = Regex::new(&pattern) {
            if re.is_match(test_source) {
                candidates.push(symbol.name.clone());
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

fn render_discovery_report(c_test_root: &Path, tests: &[CTestCase]) -> String {
    let mut output = String::from("# C Test Discovery Report\n\n");
    output.push_str(&format!(
        "Scanned: `{}`\n\n",
        c_test_root.display()
    ));
    output.push_str("## Discovered Test Functions\n\n");
    output.push_str("| test function | file |\n|---|---|\n");
    for test in tests {
        output.push_str(&format!(
            "| {} | {} |\n",
            test.name,
            test.file.display()
        ));
    }
    output.push_str(&format!("\n## Summary\n\n| metric | value |\n|---|---|\n| total C tests | {} |\n", tests.len()));
    output
}

fn render_map_report(c_test_root: &Path, tests: &[CTestCase], symbols: &[InterfaceSymbol]) -> String {
    let mut output = String::from("# C Test to Interface Candidate Mapping Report\n\n");
    output.push_str(&format!(
        "Scanned: `{}`\n\n",
        c_test_root.display()
    ));
    output.push_str("## Candidate Mappings\n\n");
    output.push_str("| test function | file | candidate symbols |\n|---|---|---|\n");

    for test in tests {
        let candidates = infer_candidate_symbols(&format!("{}\n{}", test.name, test.body), symbols);
        output.push_str(&format!(
            "| {} | {} | {} |\n",
            test.name,
            test.file.display(),
            if candidates.is_empty() {
                "-".to_string()
            } else {
                candidates.join(", ")
            }
        ));
    }

    output.push_str(&format!(
        "\n## Summary\n\n| metric | value |\n|---|---|\n| total C tests | {} |\n| total interface symbols | {} |\n",
        tests.len(),
        symbols.len()
    ));
    output
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
