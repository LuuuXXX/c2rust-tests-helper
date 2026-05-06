use std::fs;
use std::path::PathBuf;

/// Create a minimal feature workspace under `base` and return its path.
fn make_feature_workspace(base: &PathBuf) -> PathBuf {
    let root = base.join("feature_root");
    let meta = root.join("meta");
    let rust_src = root.join("rust").join("src");
    let mod_a = rust_src.join("mod_alpha");

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
    assert_eq!(m.name, "mod_alpha");
    assert_eq!(m.functions, vec!["fun_add", "fun_sub"]);
    assert_eq!(m.decls, vec!["decl_foo"]);
    assert_eq!(m.vars, vec!["var_counter"]);
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
    assert_eq!(cfg.test_commands.get("rust").map(String::as_str), Some("cargo test"));
    assert_eq!(cfg.discovery.extensions, vec!["c"]);
    assert!(cfg.tests.is_empty());
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
