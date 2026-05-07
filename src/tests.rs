use std::fs;
use std::path::PathBuf;
use clap::{CommandFactory, Parser};

/// Create a minimal feature workspace under `base` and return its path.
fn make_feature_workspace(base: &PathBuf) -> PathBuf {
    let root = base.join("feature_root");
    let meta = root.join("meta");
    let rust_src = root.join("rust").join("src");
    let mod_a = rust_src.join("mod_src_alpha");

    fs::create_dir_all(&meta).unwrap();
    fs::create_dir_all(&mod_a).unwrap();

    fs::write(
        meta.join("selected_files.json"),
        r#"["src/alpha.c", "src/beta.c"]"#,
    )
    .unwrap();

    fs::write(mod_a.join("fun_add.rs"), "// add").unwrap();
    fs::write(mod_a.join("fun_sub.rs"), "// sub").unwrap();
    fs::write(mod_a.join("decl_foo.rs"), "// foo decl").unwrap();
    fs::write(mod_a.join("var_counter.rs"), "// counter").unwrap();

    root
}

#[test]
fn test_load_feature_index() {
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);

    let index = crate::feature::loader::load(&root).expect("load should succeed");

    assert_eq!(index.selected_files, vec!["src/alpha.c", "src/beta.c"]);
    assert_eq!(index.modules.len(), 1);

    let m = &index.modules[0];
    assert_eq!(m.name, "mod_src_alpha");
    assert_eq!(m.selected_file.as_deref(), Some("src/alpha.c"));
    assert_eq!(m.functions, vec!["add", "sub"]);
    assert_eq!(m.decls, vec!["foo"]);
    assert_eq!(m.vars, vec!["counter"]);
}

#[test]
fn test_load_missing_root() {
    let err = crate::feature::loader::load(std::path::Path::new("/nonexistent/path"))
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("does not exist"), "unexpected error: {msg}");
}

#[test]
fn test_load_missing_meta() {
    let tmp = tempdir();
    let root = tmp.join("feature");
    fs::create_dir_all(root.join("rust").join("src")).unwrap();
    // No meta/ directory.

    let err = crate::feature::loader::load(&root).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("meta/"), "unexpected error: {msg}");
}

#[test]
fn test_load_missing_selected_files() {
    let tmp = tempdir();
    let root = tmp.join("feature");
    fs::create_dir_all(root.join("meta")).unwrap();
    fs::create_dir_all(root.join("rust").join("src")).unwrap();
    // No selected_files.json.

    let err = crate::feature::loader::load(&root).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("selected_files.json"), "unexpected error: {msg}");
}

#[test]
fn test_load_missing_rust_src() {
    let tmp = tempdir();
    let root = tmp.join("feature");
    let meta = root.join("meta");
    fs::create_dir_all(&meta).unwrap();
    fs::write(meta.join("selected_files.json"), "[]").unwrap();
    // No rust/src/ directory.

    let err = crate::feature::loader::load(&root).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("rust/src/"), "unexpected error: {msg}");
}

#[test]
fn test_config_parse() {
    let yaml = r#"
version: 1
project:
  root: ../c2rust-demo
  feature: default
feature_source:
  kind: c2rust_feature
  root: ../c2rust-demo/.c2rust/default
test_commands:
  c: "make test"
  rust: "cargo test"
discovery:
  paths:
    - tests/c
  extensions:
    - c
  patterns:
    - regex: 'void\s+(test_\w+)\s*\('
      framework: custom
tests: []
"#;
    let tmp = tempdir();
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();

    let cfg = crate::config::load_config(&cfg_path).expect("parse should succeed");
    assert_eq!(cfg.version, 1);
    assert_eq!(cfg.project.feature, "default");
    assert_eq!(cfg.feature_source.kind, "c2rust_feature");
    assert_eq!(cfg.test_commands.rust.as_deref(), Some("cargo test"));
    assert_eq!(cfg.discovery.extensions, vec!["c"]);
    assert!(cfg.tests.is_empty());
}

#[test]
fn test_config_parse_test_entry() {
    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: ./.c2rust/default
tests:
  - c_test: test_add
    source_file: tests/c/test_math.c
    status: ported
    module: mod_math
    symbols:
      - add
    rust_tests:
      - test_add_ported
  - c_test: test_sub
    status: pending
  - c_test: test_deprecated
    status: skipped
    notes: "removed from upstream"
  - c_test: test_platform_specific
    status: not_applicable
    notes: "Linux only"
"#;
    let tmp = tempdir();
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();

    let cfg = crate::config::load_config(&cfg_path).expect("parse should succeed");
    assert_eq!(cfg.tests.len(), 4);

    let t0 = &cfg.tests[0];
    assert_eq!(t0.c_test, "test_add");
    assert_eq!(t0.status, crate::config::TestStatus::Ported);
    assert_eq!(t0.module.as_deref(), Some("mod_math"));
    assert_eq!(t0.symbols, vec!["add"]);
    assert_eq!(t0.rust_tests, vec!["test_add_ported"]);

    let t1 = &cfg.tests[1];
    assert_eq!(t1.c_test, "test_sub");
    assert_eq!(t1.status, crate::config::TestStatus::Pending);

    let t2 = &cfg.tests[2];
    assert_eq!(t2.status, crate::config::TestStatus::Skipped);
    assert_eq!(t2.notes.as_deref(), Some("removed from upstream"));

    let t3 = &cfg.tests[3];
    assert_eq!(t3.status, crate::config::TestStatus::NotApplicable);
}

#[test]
fn test_config_parse_with_derived_roots() {
    let yaml = r#"
version: 1
project:
  feature: default
discovery:
  paths:
    - tests/c
tests: []
"#;
    let tmp = tempdir();
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();

    let cfg = crate::config::load_config(&cfg_path).expect("parse should succeed");
    assert_eq!(cfg.project.root, "");
    assert!(cfg.feature_source.is_default());
}

#[test]
fn test_cli_accepts_new_commands() {
    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "discover"]).unwrap();
    match cli.command {
        crate::Command::Discover { config } => assert_eq!(config, PathBuf::from("migration.yml")),
        _ => panic!("expected discover command"),
    }

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "validate"]).unwrap();
    match cli.command {
        crate::Command::Validate { config } => assert_eq!(config, PathBuf::from("migration.yml")),
        _ => panic!("expected validate command"),
    }

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "status"]).unwrap();
    match cli.command {
        crate::Command::Status { config } => assert_eq!(config, PathBuf::from("migration.yml")),
        _ => panic!("expected status command"),
    }

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "verify"]).unwrap();
    match cli.command {
        crate::Command::Verify { config } => assert_eq!(config, PathBuf::from("migration.yml")),
        _ => panic!("expected verify command"),
    }

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "inspect"]).unwrap();
    match cli.command {
        crate::Command::Inspect { config } => assert_eq!(config, PathBuf::from("migration.yml")),
        _ => panic!("expected inspect command"),
    }
}

#[test]
fn test_cli_legacy_commands_still_work_as_aliases() {
    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "collect"]).unwrap();
    assert!(matches!(cli.command, crate::Command::Discover { .. }));

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "lint"]).unwrap();
    assert!(matches!(cli.command, crate::Command::Validate { .. }));

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "report"]).unwrap();
    assert!(matches!(cli.command, crate::Command::Status { .. }));

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "check"]).unwrap();
    assert!(matches!(cli.command, crate::Command::Verify { .. }));

    let cli = crate::Cli::try_parse_from(["c2rust-tests-helper", "surface"]).unwrap();
    assert!(matches!(cli.command, crate::Command::Inspect { .. }));
}

#[test]
fn test_cli_help_shows_new_commands_and_legacy_aliases() {
    let mut cmd = crate::Cli::command();
    let mut help = Vec::new();
    cmd.write_long_help(&mut help).unwrap();
    let help = String::from_utf8(help).unwrap();

    assert!(help.contains("discover"));
    assert!(help.contains("validate"));
    assert!(help.contains("status"));
    assert!(help.contains("verify"));
    assert!(help.contains("inspect"));

    assert!(help.contains("aliases: collect"));
    assert!(help.contains("aliases: lint"));
    assert!(help.contains("aliases: report"));
    assert!(help.contains("aliases: check"));
    assert!(help.contains("aliases: surface"));
}

#[test]
fn test_validate_config_accepts_derived_project_and_feature_roots() {
    let tmp = tempdir();
    let project_root = tmp.join("project");
    let feature_root = project_root.join(".c2rust").join("default");
    fs::create_dir_all(feature_root.join("meta")).unwrap();
    fs::create_dir_all(feature_root.join("rust").join("src")).unwrap();
    fs::write(
        feature_root.join("meta").join("selected_files.json"),
        r#"["tests/c/test_math.c"]"#,
    )
    .unwrap();

    let cfg_path = project_root.join("migration.yml");
    fs::create_dir_all(&project_root).unwrap();
    fs::write(
        &cfg_path,
        r#"
version: 1
project:
  feature: default
tests: []
"#,
    )
    .unwrap();

    let cfg = crate::config::load_config(&cfg_path).unwrap();
    crate::validate_config(&cfg, &cfg_path).unwrap();
}

#[test]
fn test_cmd_collect_enriches_manifest_from_feature_surface() {
    let tmp = tempdir();
    let project_root = tmp.join("project");
    let tests_dir = project_root.join("tests").join("c");
    let feature_root = project_root.join(".c2rust").join("default");
    let module_dir = feature_root.join("rust").join("src").join("mod_tests_c_test_math");

    fs::create_dir_all(&tests_dir).unwrap();
    fs::create_dir_all(feature_root.join("meta")).unwrap();
    fs::create_dir_all(&module_dir).unwrap();

    fs::write(tests_dir.join("test_math.c"), "void test_add() {}\n").unwrap();
    fs::write(
        feature_root.join("meta").join("selected_files.json"),
        r#"["tests/c/test_math.c"]"#,
    )
    .unwrap();
    fs::write(module_dir.join("fun_test_add.rs"), "// translated test").unwrap();

    let cfg_path = project_root.join("migration.yml");
    fs::write(
        &cfg_path,
        r#"
version: 1
project:
  feature: default
discovery:
  paths:
    - tests/c
  extensions:
    - c
  patterns:
    - regex: 'void\s+(test_\w+)\s*\(\)'
      framework: custom
tests: []
"#,
    )
    .unwrap();

    crate::cmd_collect(&cfg_path).unwrap();

    let cfg = crate::config::load_config(&cfg_path).unwrap();
    assert_eq!(cfg.tests.len(), 1);
    let entry = &cfg.tests[0];
    assert_eq!(entry.c_test, "test_add");
    assert_eq!(entry.source_file.as_deref(), Some("tests/c/test_math.c"));
    assert_eq!(entry.selected_file.as_deref(), Some("tests/c/test_math.c"));
    assert_eq!(entry.module.as_deref(), Some("mod_tests_c_test_math"));
    assert_eq!(entry.symbols, vec!["test_add"]);
}

#[test]
fn test_cmd_lint_accepts_inferred_manifest_fields() {
    let tmp = tempdir();
    let project_root = tmp.join("project");
    let feature_root = project_root.join(".c2rust").join("default");
    let module_dir = feature_root.join("rust").join("src").join("mod_tests_c_test_math");

    fs::create_dir_all(feature_root.join("meta")).unwrap();
    fs::create_dir_all(&module_dir).unwrap();
    fs::write(
        feature_root.join("meta").join("selected_files.json"),
        r#"["tests/c/test_math.c"]"#,
    )
    .unwrap();
    fs::write(module_dir.join("fun_test_add.rs"), "// translated test").unwrap();

    let cfg_path = project_root.join("migration.yml");
    fs::create_dir_all(&project_root).unwrap();
    fs::write(
        &cfg_path,
        r#"
version: 1
project:
  feature: default
tests:
  - c_test: test_add
    source_file: tests/c/test_math.c
    status: ported
    rust_tests:
      - test_add_ported
"#,
    )
    .unwrap();

    crate::cmd_lint(&cfg_path).unwrap();
}

// ── collect tests ─────────────────────────────────────────────────────────────

#[test]
fn test_collect_discovers_tests() {
    let tmp = tempdir();

    // Create project root with C test files.
    let project_root = tmp.join("project");
    let tests_dir = project_root.join("tests").join("c");
    fs::create_dir_all(&tests_dir).unwrap();
    fs::write(
        tests_dir.join("test_math.c"),
        "void test_add() {}\nvoid test_sub() {}\n",
    )
    .unwrap();

    // Create a minimal feature workspace.
    let feature_root = tmp.join("feature");
    let meta = feature_root.join("meta");
    fs::create_dir_all(&meta).unwrap();
    fs::write(meta.join("selected_files.json"), "[]").unwrap();
    fs::create_dir_all(feature_root.join("rust").join("src")).unwrap();

    let yaml = format!(
        r#"
version: 1
project:
  root: {project}
  feature: feature
feature_source:
  kind: c2rust_feature
  root: {feat}
discovery:
  paths:
    - tests/c
  extensions:
    - c
  patterns:
    - regex: 'void\s+(test_\w+)\s*\(\)'
      framework: custom
tests: []
"#,
        project = project_root.display(),
        feat = feature_root.display()
    );

    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, &yaml).unwrap();

    let mut cfg = crate::config::load_config(&cfg_path).unwrap();
    let added = crate::collect::collect_tests(&mut cfg, &cfg_path).unwrap();

    assert_eq!(added, 2, "should discover 2 new tests");
    let names: Vec<&str> = cfg.tests.iter().map(|t| t.c_test.as_str()).collect();
    assert!(names.contains(&"test_add"), "test_add should be found");
    assert!(names.contains(&"test_sub"), "test_sub should be found");
    // All new entries must carry a source_file.
    assert!(cfg.tests.iter().all(|t| t.source_file.is_some()));
}

#[test]
fn test_collect_is_idempotent() {
    let tmp = tempdir();

    let project_root = tmp.join("project");
    let tests_dir = project_root.join("tests").join("c");
    fs::create_dir_all(&tests_dir).unwrap();
    fs::write(tests_dir.join("test_math.c"), "void test_add() {}\n").unwrap();

    let feature_root = tmp.join("feature");
    let meta = feature_root.join("meta");
    fs::create_dir_all(&meta).unwrap();
    fs::write(meta.join("selected_files.json"), "[]").unwrap();
    fs::create_dir_all(feature_root.join("rust").join("src")).unwrap();

    // Pre-populate the manifest with test_add already present (with the same
    // source_file that collect would assign – the idempotency key is (c_test, source_file)).
    let yaml = format!(
        r#"
version: 1
project:
  root: {project}
  feature: feature
feature_source:
  kind: c2rust_feature
  root: {feat}
discovery:
  paths:
    - tests/c
  extensions:
    - c
  patterns:
    - regex: 'void\s+(test_\w+)\s*\(\)'
      framework: custom
tests:
  - c_test: test_add
    source_file: tests/c/test_math.c
    status: ported
    rust_tests:
      - test_add_rust
"#,
        project = project_root.display(),
        feat = feature_root.display()
    );

    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, &yaml).unwrap();

    let mut cfg = crate::config::load_config(&cfg_path).unwrap();
    let added = crate::collect::collect_tests(&mut cfg, &cfg_path).unwrap();

    assert_eq!(added, 0, "nothing new should be added on second run");
    assert_eq!(cfg.tests.len(), 1);
    // The existing entry must be untouched.
    assert_eq!(cfg.tests[0].status, crate::config::TestStatus::Ported);
    assert_eq!(cfg.tests[0].rust_tests, vec!["test_add_rust"]);
}

#[test]
fn test_collect_same_name_different_file_both_added() {
    let tmp = tempdir();

    let project_root = tmp.join("project");
    let dir_a = project_root.join("tests").join("a");
    let dir_b = project_root.join("tests").join("b");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();
    fs::write(dir_a.join("test_foo.c"), "void test_foo() {}\n").unwrap();
    fs::write(dir_b.join("test_foo.c"), "void test_foo() {}\n").unwrap();

    let feature_root = tmp.join("feature");
    let meta = feature_root.join("meta");
    fs::create_dir_all(&meta).unwrap();
    fs::write(meta.join("selected_files.json"), "[]").unwrap();
    fs::create_dir_all(feature_root.join("rust").join("src")).unwrap();

    let yaml = format!(
        r#"
version: 1
project:
  root: {project}
  feature: feature
feature_source:
  kind: c2rust_feature
  root: {feat}
discovery:
  paths:
    - tests/a
    - tests/b
  extensions:
    - c
  patterns:
    - regex: 'void\s+(test_\w+)\s*\(\)'
      framework: custom
tests: []
"#,
        project = project_root.display(),
        feat = feature_root.display()
    );

    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, &yaml).unwrap();

    let mut cfg = crate::config::load_config(&cfg_path).unwrap();
    let added = crate::collect::collect_tests(&mut cfg, &cfg_path).unwrap();

    // test_foo from a/ and test_foo from b/ are distinct (different source_file).
    assert_eq!(added, 2, "same name in different files should both be added");
    let source_files: Vec<_> = cfg.tests.iter().filter_map(|t| t.source_file.as_deref()).collect();
    assert!(source_files.iter().any(|f| f.contains("tests/a")), "a/ entry missing");
    assert!(source_files.iter().any(|f| f.contains("tests/b")), "b/ entry missing");
}

#[test]
fn test_collect_no_capture_group_warns() {
    // A regex with no capture group should not panic – it just discovers nothing.
    let tmp = tempdir();

    let project_root = tmp.join("project");
    let tests_dir = project_root.join("tests").join("c");
    fs::create_dir_all(&tests_dir).unwrap();
    fs::write(tests_dir.join("test.c"), "void test_add() {}\n").unwrap();

    let feature_root = tmp.join("feature");
    let meta = feature_root.join("meta");
    fs::create_dir_all(&meta).unwrap();
    fs::write(meta.join("selected_files.json"), "[]").unwrap();
    fs::create_dir_all(feature_root.join("rust").join("src")).unwrap();

    let yaml = format!(
        r#"
version: 1
project:
  root: {project}
  feature: feature
feature_source:
  kind: c2rust_feature
  root: {feat}
discovery:
  paths:
    - tests/c
  extensions:
    - c
  patterns:
    - regex: 'void\s+test_\w+\s*\(\)'
      framework: custom
tests: []
"#,
        project = project_root.display(),
        feat = feature_root.display()
    );

    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, &yaml).unwrap();

    let mut cfg = crate::config::load_config(&cfg_path).unwrap();
    let added = crate::collect::collect_tests(&mut cfg, &cfg_path).unwrap();
    // No capture group → no tests discovered, but no crash.
    assert_eq!(added, 0);
}

#[test]
fn test_collect_sorted_output() {
    let tmp = tempdir();

    let project_root = tmp.join("project");
    let tests_dir = project_root.join("tests").join("c");
    fs::create_dir_all(&tests_dir).unwrap();
    // Write in non-alphabetical order; collect should sort the new entries.
    fs::write(
        tests_dir.join("test_z.c"),
        "void test_zzz() {}\nvoid test_aaa() {}\n",
    )
    .unwrap();

    let feature_root = tmp.join("feature");
    let meta = feature_root.join("meta");
    fs::create_dir_all(&meta).unwrap();
    fs::write(meta.join("selected_files.json"), "[]").unwrap();
    fs::create_dir_all(feature_root.join("rust").join("src")).unwrap();

    let yaml = format!(
        r#"
version: 1
project:
  root: {project}
  feature: feature
feature_source:
  kind: c2rust_feature
  root: {feat}
discovery:
  paths:
    - tests/c
  extensions:
    - c
  patterns:
    - regex: 'void\s+(test_\w+)\s*\(\)'
      framework: custom
tests: []
"#,
        project = project_root.display(),
        feat = feature_root.display()
    );

    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, &yaml).unwrap();

    let mut cfg = crate::config::load_config(&cfg_path).unwrap();
    crate::collect::collect_tests(&mut cfg, &cfg_path).unwrap();

    // Within the same source_file the entries should be alphabetically sorted.
    let names: Vec<&str> = cfg.tests.iter().map(|t| t.c_test.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "collect should append new entries in sorted order");
}

// ── lint tests ────────────────────────────────────────────────────────────────

#[test]
fn test_lint_passes_on_valid_manifest() {
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
tests:
  - c_test: test_add
    status: ported
    selected_file: src/alpha.c
    module: mod_src_alpha
    symbols:
      - add
    rust_tests:
      - rust_test_add
  - c_test: test_skip
    status: skipped
    notes: "not applicable"
  - c_test: test_na
    status: not_applicable
    notes: "Linux only"
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    assert!(crate::lint::lint(&cfg, &index).is_ok());
}

#[test]
fn test_lint_catches_missing_module() {
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
tests:
  - c_test: test_foo
    module: mod_nonexistent
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let err = crate::lint::lint(&cfg, &index).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("lint error"), "expected lint error, got: {msg}");
}

#[test]
fn test_lint_catches_ported_without_rust_tests() {
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
tests:
  - c_test: test_add
    status: ported
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let err = crate::lint::lint(&cfg, &index).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("lint error"), "expected lint error, got: {msg}");
}

#[test]
fn test_lint_catches_skipped_without_notes() {
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
tests:
  - c_test: test_skip
    status: skipped
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let err = crate::lint::lint(&cfg, &index).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("lint error"), "expected lint error, got: {msg}");
}

#[test]
fn test_lint_catches_not_applicable_without_notes() {
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
tests:
  - c_test: test_na
    status: not_applicable
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let err = crate::lint::lint(&cfg, &index).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("lint error"), "expected lint error, got: {msg}");
}

#[test]
fn test_lint_catches_symbols_without_module() {
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
tests:
  - c_test: test_add
    symbols:
      - add
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let err = crate::lint::lint(&cfg, &index).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("lint error"), "expected lint error, got: {msg}");
}

#[test]
fn test_lint_catches_duplicate_entries() {
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
tests:
  - c_test: test_add
    source_file: tests/c/math.c
    status: pending
  - c_test: test_add
    source_file: tests/c/math.c
    status: pending
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let err = crate::lint::lint(&cfg, &index).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("lint error"), "expected lint error, got: {msg}");
}

// ── check tests ───────────────────────────────────────────────────────────────

#[test]
fn test_check_lint_fails_skips_test_commands() {
    // A manifest with a lint error: ported entry missing rust_tests.
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
test_commands:
  c: "true"
  rust: "true"
tests:
  - c_test: test_broken
    status: ported
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let project_root = tmp.as_path();
    let summary = crate::check::run(&cfg, &index, project_root);

    assert_eq!(summary.lint, crate::check::StepResult::Failed);
    // Test commands must be skipped when lint fails.
    assert_eq!(summary.c_tests, crate::check::StepResult::Skipped);
    assert_eq!(summary.rust_tests, crate::check::StepResult::Skipped);
    assert_eq!(summary.feature_rust, None);
    assert!(!summary.overall_passed());
}

#[test]
fn test_check_no_commands_configured() {
    // Valid manifest, no test_commands → everything skipped except lint.
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
tests: []
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let project_root = tmp.as_path();
    let summary = crate::check::run(&cfg, &index, project_root);

    assert_eq!(summary.lint, crate::check::StepResult::Passed);
    assert_eq!(summary.c_tests, crate::check::StepResult::Skipped);
    assert_eq!(summary.rust_tests, crate::check::StepResult::Skipped);
    assert_eq!(summary.feature_rust, None);
    assert!(summary.overall_passed());
}

#[test]
fn test_check_passing_commands() {
    // Valid manifest with commands that succeed (using shell `true`).
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
test_commands:
  c: "true"
  rust: "true"
tests: []
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let project_root = tmp.as_path();
    let summary = crate::check::run(&cfg, &index, project_root);

    assert_eq!(summary.lint, crate::check::StepResult::Passed);
    assert_eq!(summary.c_tests, crate::check::StepResult::Passed);
    assert_eq!(summary.rust_tests, crate::check::StepResult::Passed);
    assert_eq!(summary.feature_rust, None);
    assert!(summary.overall_passed());
}

#[test]
fn test_check_feature_rust_configured_and_failing() {
    // Legacy-compatible feature_rust still participates in overall pass/fail.
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
test_commands:
  c: "true"
  rust: "true"
  feature_rust: "false"
tests: []
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let project_root = tmp.as_path();
    let summary = crate::check::run(&cfg, &index, project_root);

    assert_eq!(summary.lint, crate::check::StepResult::Passed);
    assert_eq!(summary.c_tests, crate::check::StepResult::Passed);
    assert_eq!(summary.rust_tests, crate::check::StepResult::Passed);
    assert_eq!(summary.feature_rust, Some(crate::check::StepResult::Failed));
    assert!(!summary.overall_passed());
}

#[test]
fn test_check_failing_command() {
    // Valid manifest but a test command exits non-zero.
    let tmp = tempdir();
    let root = make_feature_workspace(&tmp);
    let index = crate::feature::loader::load(&root).unwrap();

    let yaml = r#"
version: 1
project:
  root: .
  feature: default
feature_source:
  kind: c2rust_feature
  root: .
test_commands:
  c: "false"
tests: []
"#;
    let cfg_path = tmp.join("migration.yml");
    fs::write(&cfg_path, yaml).unwrap();
    let cfg = crate::config::load_config(&cfg_path).unwrap();

    let project_root = tmp.as_path();
    let summary = crate::check::run(&cfg, &index, project_root);

    assert_eq!(summary.lint, crate::check::StepResult::Passed);
    assert_eq!(summary.c_tests, crate::check::StepResult::Failed);
    assert!(!summary.overall_passed());
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn tempdir() -> PathBuf {
    let p = std::env::temp_dir()
        .join(format!("c2rust_helper_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()));
    fs::create_dir_all(&p).unwrap();
    p
}
