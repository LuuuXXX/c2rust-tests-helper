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
            assert_eq!(report, PathBuf::from("meta/init-interface-report.md"))
        }
        _ => panic!("expected interface command"),
    }

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "scan"]).unwrap();
    match cli.command {
        crate::Command::Scan { report, rust_root } => {
            assert_eq!(report, PathBuf::from("meta/init-interface-report.md"));
            assert_eq!(rust_root, PathBuf::from("."));
        }
        _ => panic!("expected scan command"),
    }

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "coverage"]).unwrap();
    match cli.command {
        crate::Command::Coverage { report, rust_root } => {
            assert_eq!(report, PathBuf::from("meta/init-interface-report.md"));
            assert_eq!(rust_root, PathBuf::from("."));
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
