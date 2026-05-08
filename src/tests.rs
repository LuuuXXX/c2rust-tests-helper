use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn create_temp_dir(name: &str) -> TempDirGuard {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("c2rust-tests-helper_{name}_{timestamp}"));
    fs::create_dir_all(&path).expect("create temp dir");
    TempDirGuard { path }
}

#[test]
fn test_cli_accepts_new_commands() {
    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "interface"]).unwrap();
    match cli.command {
        crate::Command::Interface { report } => {
            assert_eq!(report, None)
        }
        _ => panic!("expected interface command"),
    }

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "scan"]).unwrap();
    match cli.command {
        crate::Command::Scan {
            report,
            rust_root,
            output,
        } => {
            assert_eq!(report, None);
            assert_eq!(rust_root, PathBuf::from("."));
            assert_eq!(output, None);
        }
        _ => panic!("expected scan command"),
    }

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "coverage"]).unwrap();
    match cli.command {
        crate::Command::Coverage {
            report,
            rust_root,
            output,
        } => {
            assert_eq!(report, None);
            assert_eq!(rust_root, PathBuf::from("."));
            assert_eq!(output, None);
        }
        _ => panic!("expected coverage command"),
    }
}

#[test]
fn test_cli_rejects_legacy_commands() {
    assert!(crate::Cli::try_parse_from(["c2rust-tests-helper", "collect"]).is_err());
    assert!(crate::Cli::try_parse_from(["c2rust-tests-helper", "lint"]).is_err());
    assert!(crate::Cli::try_parse_from(["c2rust-tests-helper", "report"]).is_err());
}

#[test]
fn test_parse_interface_report_function_and_variable() {
    let report = r#"
# Init Interface Report — feature `demo`

---

## mod_src_foo

### `add` (function)

- **Rust symbol:** `add`

### `counter` (variable)
- **FFI:** `static mut counter: c_int`
"#;
    let symbols = crate::parse_interface_report(report).unwrap();
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].module, "mod_src_foo");
    assert_eq!(symbols[0].name, "add");
    assert_eq!(symbols[0].kind, crate::SymbolKind::Function);
    assert_eq!(symbols[1].name, "counter");
    assert_eq!(symbols[1].kind, crate::SymbolKind::Variable);
}

#[test]
fn test_parse_interface_report_skips_non_module_sections() {
    let report = r#"
## Summary

### `ignored` (function)

## `lib.rs` — Shared FFI

### `also_ignored` (variable)

## mod_src_foo

### `kept_symbol` (function)

## my_lib.rs_parser

### `also_kept` (function)
"#;
    let symbols = crate::parse_interface_report(report).unwrap();
    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].module, "mod_src_foo");
    assert_eq!(symbols[0].name, "kept_symbol");
    assert_eq!(symbols[1].module, "my_lib.rs_parser");
    assert_eq!(symbols[1].name, "also_kept");
}

#[test]
fn test_parse_merge_report_symbols() {
    let report = r#"
# Merge Interface Report — feature `default`

## Summary

| Item | Count |
|---|---|
| modules | 1 |

## `lib.rs` — Shared FFI

- `shared_fn`

## mod_src_foo

### Final Rust functions

- `add`
- `compute`

### Final Rust variables

- `counter`

### Module-local FFI

- `local_fn`

### Source files merged

- `mod_src_foo/mod.rs`
"#;
    let symbols = crate::parse_interface_report(report).unwrap();
    assert_eq!(symbols.len(), 4);
    assert_eq!(symbols[0].module, "lib.rs");
    assert_eq!(symbols[0].name, "shared_fn");
    assert_eq!(symbols[0].kind, crate::SymbolKind::Function);
    assert_eq!(symbols[1].module, "mod_src_foo");
    assert_eq!(symbols[1].name, "add");
    assert_eq!(symbols[1].kind, crate::SymbolKind::Function);
    assert_eq!(symbols[2].name, "compute");
    assert_eq!(symbols[2].kind, crate::SymbolKind::Function);
    assert_eq!(symbols[3].name, "counter");
    assert_eq!(symbols[3].kind, crate::SymbolKind::Variable);
}

#[test]
fn test_resolve_default_report_path_prefers_init_then_merge() {
    let dir = create_temp_dir("fallback");
    let meta = dir.path().join("meta");
    fs::create_dir_all(&meta).unwrap();
    fs::write(meta.join("init-interface-report.md"), "# Init Interface Report").unwrap();
    fs::write(meta.join("merge-interface-report.md"), "# Merge Interface Report").unwrap();

    let resolved = crate::resolve_default_report_path(dir.path()).unwrap();
    assert_eq!(resolved, PathBuf::from("meta/init-interface-report.md"));

    fs::remove_file(meta.join("init-interface-report.md")).unwrap();
    let resolved = crate::resolve_default_report_path(dir.path()).unwrap();
    assert_eq!(resolved, PathBuf::from("meta/merge-interface-report.md"));
}

#[test]
fn test_resolve_default_report_path_errors_when_missing() {
    let dir = create_temp_dir("fallback-missing");
    let err = crate::resolve_default_report_path(dir.path()).unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("meta/init-interface-report.md"));
    assert!(msg.contains("meta/merge-interface-report.md"));
    assert!(msg.contains(".c2rust/<feature>/meta/{init,merge}-interface-report.md"));
}

#[test]
fn test_resolve_default_report_path_finds_c2rust_feature_init() {
    let dir = create_temp_dir("fallback-c2rust-init");
    let meta = dir.path().join(".c2rust/default/meta");
    fs::create_dir_all(&meta).unwrap();
    let report_path = meta.join("init-interface-report.md");
    fs::write(&report_path, "# Init Interface Report").unwrap();

    let resolved = crate::resolve_default_report_path(dir.path()).unwrap();
    assert_eq!(resolved, report_path);
}

#[test]
fn test_resolve_default_report_path_finds_c2rust_feature_merge() {
    let dir = create_temp_dir("fallback-c2rust-merge");
    let meta = dir.path().join(".c2rust/default/meta");
    fs::create_dir_all(&meta).unwrap();
    let report_path = meta.join("merge-interface-report.md");
    fs::write(&report_path, "# Merge Interface Report").unwrap();

    let resolved = crate::resolve_default_report_path(dir.path()).unwrap();
    assert_eq!(resolved, report_path);
}

#[test]
fn test_resolve_default_report_path_prefers_meta_over_c2rust() {
    let dir = create_temp_dir("fallback-meta-priority");
    let meta = dir.path().join("meta");
    fs::create_dir_all(&meta).unwrap();
    fs::write(meta.join("init-interface-report.md"), "# Init Interface Report").unwrap();

    let c2rust_meta = dir.path().join(".c2rust/default/meta");
    fs::create_dir_all(&c2rust_meta).unwrap();
    fs::write(
        c2rust_meta.join("init-interface-report.md"),
        "# C2rust Init Interface Report",
    )
    .unwrap();

    let resolved = crate::resolve_default_report_path(dir.path()).unwrap();
    assert_eq!(resolved, PathBuf::from("meta/init-interface-report.md"));
}

#[test]
fn test_resolve_default_report_path_c2rust_prefers_init_over_merge() {
    let dir = create_temp_dir("fallback-c2rust-priority");
    let meta = dir.path().join(".c2rust/default/meta");
    fs::create_dir_all(&meta).unwrap();
    let init_path = meta.join("init-interface-report.md");
    fs::write(&init_path, "# Init Interface Report").unwrap();
    fs::write(meta.join("merge-interface-report.md"), "# Merge Interface Report").unwrap();

    let resolved = crate::resolve_default_report_path(dir.path()).unwrap();
    assert_eq!(resolved, init_path);
}

#[test]
fn test_scan_rust_tests_and_match_interfaces() {
    let dir = create_temp_dir("scan");
    let src = dir.path().join("src");
    let tests = dir.path().join("tests");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&tests).unwrap();

    fs::write(
        src.join("unit.rs"),
        r#"
#[test]
fn dt_add_works() {
    assert_eq!(unsafe { add(1, 2) }, 3);
}

#[test]
fn system_keyword_in_name_but_unit_test() {
    assert_eq!(unsafe { add(1, 2) }, 3);
}
"#,
    )
    .unwrap();

    fs::write(
        tests.join("system.rs"),
        r#"
#[test]
fn st_counter_smoke() {
    unsafe { counter = 1; }
}
"#,
    )
    .unwrap();

    let symbols = vec![
        crate::InterfaceSymbol {
            module: "mod_src_foo".to_string(),
            name: "add".to_string(),
            kind: crate::SymbolKind::Function,
        },
        crate::InterfaceSymbol {
            module: "mod_src_foo".to_string(),
            name: "counter".to_string(),
            kind: crate::SymbolKind::Variable,
        },
    ];

    let scanned = crate::scan_rust_tests(dir.path(), &symbols).unwrap();
    assert_eq!(scanned.len(), 3);

    let dt = scanned.iter().find(|t| t.name == "dt_add_works").unwrap();
    assert_eq!(dt.kind, crate::TestKind::Dt);
    assert_eq!(dt.interfaces, vec!["add".to_string()]);

    let dt_system = scanned
        .iter()
        .find(|t| t.name == "system_keyword_in_name_but_unit_test")
        .unwrap();
    assert_eq!(dt_system.kind, crate::TestKind::Dt);

    let st = scanned.iter().find(|t| t.name == "st_counter_smoke").unwrap();
    assert_eq!(st.kind, crate::TestKind::St);
    assert_eq!(st.interfaces, vec!["counter".to_string()]);
}

#[test]
fn test_scan_and_coverage_write_reports() {
    let dir = create_temp_dir("write-report");
    let meta = dir.path().join("meta");
    let rust_dir = dir.path().join("rust");
    fs::create_dir_all(&meta).unwrap();
    fs::create_dir_all(&rust_dir).unwrap();

    fs::write(
        meta.join("init-interface-report.md"),
        r#"
# Init Interface Report — feature `demo`

## mod_src_foo

### `add` (function)
"#,
    )
    .unwrap();

    fs::write(
        rust_dir.join("mod.rs"),
        r#"
#[test]
fn dt_add_smoke() {
    unsafe { add(1, 2); }
}
"#,
    )
    .unwrap();

    let report_path = meta.join("init-interface-report.md");
    let scan_output_path = crate::resolve_output_path(&report_path, None, "test-scan-report.md");
    let coverage_output_path = crate::resolve_output_path(&report_path, None, "coverage-report.md");

    crate::cmd_scan(&report_path, &rust_dir, &scan_output_path).unwrap();
    crate::cmd_coverage(&report_path, &rust_dir, &coverage_output_path).unwrap();

    let scan_output = fs::read_to_string(meta.join("test-scan-report.md")).unwrap();
    assert!(scan_output.contains("# Test Scan Report"));
    assert!(scan_output.contains("| test | type | file | interfaces |"));
    assert!(scan_output.contains("dt_add_smoke"));

    let coverage_output = fs::read_to_string(meta.join("coverage-report.md")).unwrap();
    assert!(coverage_output.contains("# Coverage Report"));
    assert!(coverage_output.contains("| interface | kind | ST | DT |"));
    assert!(coverage_output.contains("mod_src_foo::add"));
    assert!(coverage_output.contains("## Uncovered Interfaces"));
    assert!(coverage_output.contains("- *(none)*"));
    assert!(coverage_output.contains("## Summary"));
    assert!(coverage_output.contains("| total interfaces | 1 |"));
    assert!(coverage_output.contains("| covered by DT | 1 |"));
}

#[test]
fn test_coverage_report_includes_uncovered_list_and_summary() {
    let symbols = vec![
        crate::InterfaceSymbol {
            module: "m".to_string(),
            name: "covered".to_string(),
            kind: crate::SymbolKind::Function,
        },
        crate::InterfaceSymbol {
            module: "m".to_string(),
            name: "uncovered".to_string(),
            kind: crate::SymbolKind::Variable,
        },
    ];
    let tests = vec![crate::TestMatch {
        name: "dt_covered".to_string(),
        kind: crate::TestKind::Dt,
        file: PathBuf::from("src/mod.rs"),
        interfaces: vec!["covered".to_string()],
    }];

    let report = crate::render_coverage_matrix(&symbols, &tests);
    assert!(report.contains("## Uncovered Interfaces"));
    assert!(report.contains("- `m::uncovered (variable)`"));
    assert!(report.contains("## Summary"));
    assert!(report.contains("| total interfaces | 2 |"));
    assert!(report.contains("| covered by DT | 1 |"));
    assert!(report.contains("| uncovered | 1 |"));
}
