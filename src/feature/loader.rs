use anyhow::{bail, Context, Result};
use std::path::Path;

use super::index::{FeatureIndex, FeatureModule};

/// Load a `FeatureIndex` from a c2rust-demo feature workspace root.
///
/// Expected layout:
/// ```text
/// <feature_root>/
///   meta/
///     selected_files.json
///   rust/
///     src/
///       mod_<name>/
///         fun_*.rs
///         decl_*.rs
///         var_*.rs
/// ```
pub fn load(feature_root: &Path) -> Result<FeatureIndex> {
    validate_layout(feature_root)?;

    let selected_files = load_selected_files(feature_root)?;
    let modules = load_modules(feature_root)?;

    Ok(FeatureIndex {
        feature_root: feature_root.to_path_buf(),
        selected_files,
        modules,
    })
}

// ── validation ────────────────────────────────────────────────────────────────

fn validate_layout(feature_root: &Path) -> Result<()> {
    if !feature_root.exists() {
        bail!(
            "feature source root does not exist: {}",
            feature_root.display()
        );
    }

    let meta_dir = feature_root.join("meta");
    if !meta_dir.exists() {
        bail!(
            "expected `meta/` directory inside feature root: {}",
            feature_root.display()
        );
    }

    let selected_json = meta_dir.join("selected_files.json");
    if !selected_json.exists() {
        bail!(
            "expected `meta/selected_files.json` inside feature root: {}",
            feature_root.display()
        );
    }

    let rust_src = feature_root.join("rust").join("src");
    if !rust_src.exists() {
        bail!(
            "expected `rust/src/` directory inside feature root: {}",
            feature_root.display()
        );
    }

    Ok(())
}

// ── selected files ────────────────────────────────────────────────────────────

fn load_selected_files(feature_root: &Path) -> Result<Vec<String>> {
    let path = feature_root.join("meta").join("selected_files.json");
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let files: Vec<String> = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(files)
}

// ── modules ───────────────────────────────────────────────────────────────────

fn load_modules(feature_root: &Path) -> Result<Vec<FeatureModule>> {
    let rust_src = feature_root.join("rust").join("src");
    let mut modules = Vec::new();

    let entries = std::fs::read_dir(&rust_src)
        .with_context(|| format!("failed to read directory: {}", rust_src.display()))?;

    let mut mod_dirs: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                && e.file_name()
                    .to_string_lossy()
                    .starts_with("mod_")
        })
        .collect();

    mod_dirs.sort_by_key(|e| e.file_name());

    for entry in mod_dirs {
        let mod_name = entry.file_name().to_string_lossy().into_owned();
        let mod_path = entry.path();

        let functions = collect_stems(&mod_path, "fun_")?;
        let decls = collect_stems(&mod_path, "decl_")?;
        let vars = collect_stems(&mod_path, "var_")?;

        modules.push(FeatureModule {
            name: mod_name,
            functions,
            decls,
            vars,
        });
    }

    Ok(modules)
}

/// Collect the file stems (without extension) of `*.rs` files whose names start
/// with `prefix` inside `dir`.
fn collect_stems(dir: &Path, prefix: &str) -> Result<Vec<String>> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory: {}", dir.display()))?;

    let mut stems: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            if s.starts_with(prefix) && s.ends_with(".rs") {
                // Strip the prefix and the `.rs` extension to get a pure symbol name.
                // e.g. "fun_add.rs" with prefix "fun_" → "add"
                Some(s[prefix.len()..s.len() - 3].to_owned())
            } else {
                None
            }
        })
        .collect();

    stems.sort();
    Ok(stems)
}
