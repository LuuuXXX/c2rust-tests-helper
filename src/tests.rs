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
fn test_cli_accepts_commands() {
    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "interface"]).unwrap();
    match cli.command {
        crate::Command::Interface { report } => {
            assert_eq!(report, None)
        }
        _ => panic!("expected interface command"),
    }

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "discover"]).unwrap();
    match cli.command {
        crate::Command::Discover {
            c_test_root,
            output,
        } => {
            assert_eq!(c_test_root, PathBuf::from("."));
            assert_eq!(output, None);
        }
        _ => panic!("expected discover command"),
    }

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "map"]).unwrap();
    match cli.command {
        crate::Command::Map {
            report,
            c_test_root,
            output,
        } => {
            assert_eq!(report, None);
            assert_eq!(c_test_root, PathBuf::from("."));
            assert_eq!(output, None);
        }
        _ => panic!("expected map command"),
    }
}

#[test]
fn test_cli_rejects_legacy_commands() {
    assert!(crate::Cli::try_parse_from(["c2rust-tests-helper", "collect"]).is_err());
    assert!(crate::Cli::try_parse_from(["c2rust-tests-helper", "lint"]).is_err());
    assert!(crate::Cli::try_parse_from(["c2rust-tests-helper", "report"]).is_err());
    assert!(crate::Cli::try_parse_from(["c2rust-tests-helper", "scan"]).is_err());
    assert!(crate::Cli::try_parse_from(["c2rust-tests-helper", "coverage"]).is_err());
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
fn test_discover_c_tests_finds_test_functions() {
    let dir = create_temp_dir("discover");
    let tests_dir = dir.path().join("tests");
    fs::create_dir_all(&tests_dir).unwrap();

    fs::write(
        tests_dir.join("test_math.c"),
        r#"
#include <assert.h>

void test_add(void) {
    assert(add(1, 2) == 3);
}

int test_add_negative(void) {
    return add(-1, -2) == -3 ? 0 : 1;
}

static void helper_not_a_test(void) {}
"#,
    )
    .unwrap();

    fs::write(
        tests_dir.join("test_counter.c"),
        r#"
void test_counter_inc(void) {
    counter = 0;
    counter_inc();
    assert(counter == 1);
}
"#,
    )
    .unwrap();

    let found = crate::discover_c_tests(dir.path()).unwrap();
    assert_eq!(found.len(), 3);

    let names: Vec<&str> = found.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"test_add"));
    assert!(names.contains(&"test_add_negative"));
    assert!(names.contains(&"test_counter_inc"));
    assert!(!names.contains(&"helper_not_a_test"));
}

#[test]
fn test_discover_c_tests_ignores_non_c_files() {
    let dir = create_temp_dir("discover-ext");
    fs::write(
        dir.path().join("test_foo.rs"),
        "fn test_foo() {}",
    )
    .unwrap();
    fs::write(
        dir.path().join("test_bar.c"),
        "void test_bar(void) {}\n",
    )
    .unwrap();

    let found = crate::discover_c_tests(dir.path()).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "test_bar");
}

#[test]
fn test_discover_and_map_write_reports() {
    let dir = create_temp_dir("write-report");
    let meta = dir.path().join("meta");
    let c_dir = dir.path().join("tests").join("c");
    fs::create_dir_all(&meta).unwrap();
    fs::create_dir_all(&c_dir).unwrap();

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
        c_dir.join("test_math.c"),
        r#"
void test_add_smoke(void) {
    assert(add(1, 2) == 3);
}
"#,
    )
    .unwrap();

    let report_path = meta.join("init-interface-report.md");
    let discover_output = meta.join("c-test-discovery-report.md");
    let map_output = meta.join("c-test-map-report.md");

    crate::cmd_discover(&c_dir, &discover_output).unwrap();
    crate::cmd_map(&report_path, &c_dir, &map_output).unwrap();

    let discovery = fs::read_to_string(&discover_output).unwrap();
    assert!(discovery.contains("# C Test Discovery Report"));
    assert!(discovery.contains("| test function | file |"));
    assert!(discovery.contains("test_add_smoke"));
    assert!(discovery.contains("| total C tests | 1 |"));

    let mapping = fs::read_to_string(&map_output).unwrap();
    assert!(mapping.contains("# C Test to Interface Candidate Mapping Report"));
    assert!(mapping.contains("| test function | file | candidate symbols |"));
    assert!(mapping.contains("test_add_smoke"));
    assert!(mapping.contains("add"));
    assert!(mapping.contains("| total C tests | 1 |"));
    assert!(mapping.contains("| total interface symbols | 1 |"));
}

#[test]
fn test_infer_candidate_symbols_word_boundary() {
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

    // "add" appears as a whole word
    let candidates = crate::infer_candidate_symbols("test_add\nassert(add(1,2) == 3);", &symbols);
    assert!(candidates.contains(&"add".to_string()));
    assert!(!candidates.contains(&"counter".to_string()));

    // "add" as a substring should NOT match "add_extra"
    let candidates = crate::infer_candidate_symbols("add_extra(1);", &symbols);
    assert!(!candidates.contains(&"add".to_string()));

    // both symbols present
    let candidates = crate::infer_candidate_symbols("add(1); counter = 0;", &symbols);
    assert!(candidates.contains(&"add".to_string()));
    assert!(candidates.contains(&"counter".to_string()));
}

