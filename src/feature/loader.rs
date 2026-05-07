use anyhow::{bail, Context, Result};
use std::path::{Component, Path};

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
    let modules = load_modules(feature_root, &selected_files)?;

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

fn load_modules(feature_root: &Path, selected_files: &[String]) -> Result<Vec<FeatureModule>> {
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
        let selected_file = infer_selected_file_for_module(&mod_name, selected_files);

        modules.push(FeatureModule {
            name: mod_name,
            selected_file,
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
            // Strip the prefix and the `.rs` extension to get a pure symbol name.
            // e.g. "fun_add.rs" with prefix "fun_" → "add"
            s.strip_prefix(prefix)
                .and_then(|rest| rest.strip_suffix(".rs"))
                .map(str::to_owned)
        })
        .collect();

    stems.sort();
    Ok(stems)
}

fn infer_selected_file_for_module(mod_name: &str, selected_files: &[String]) -> Option<String> {
    let mut matches = selected_files
        .iter()
        .filter(|selected_file| module_name_matches_selected_file(mod_name, selected_file));
    let first = matches.next()?;
    if matches.next().is_some() {
        None
    } else {
        Some(first.clone())
    }
}

fn module_name_matches_selected_file(mod_name: &str, selected_file: &str) -> bool {
    module_name_for_selected_file(selected_file, false)
        .into_iter()
        .chain(module_name_for_selected_file(selected_file, true))
        .any(|candidate| candidate == mod_name)
}

fn module_name_for_selected_file(selected_file: &str, basename_only: bool) -> Option<String> {
    let normalized = selected_file.replace('\\', "/");
    let path = Path::new(&normalized);

    let components: Vec<String> = if basename_only {
        vec![path.file_stem()?.to_string_lossy().into_owned()]
    } else {
        let mut components: Vec<String> = path
            .components()
            .filter_map(|component| match component {
                Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        let last = components.last_mut()?;
        *last = Path::new(last).file_stem()?.to_string_lossy().into_owned();
        components
    };

    let suffix = components
        .into_iter()
        .map(|component| sanitize_component(&component))
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    if suffix.is_empty() {
        None
    } else {
        Some(format!("mod_{suffix}"))
    }
}

fn sanitize_component(component: &str) -> String {
    let mut out = String::with_capacity(component.len());
    let mut last_was_underscore = false;

    for ch in component.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_underscore = false;
        } else if !last_was_underscore {
            out.push('_');
            last_was_underscore = true;
        }
    }

    out.trim_matches('_').to_string()
}
