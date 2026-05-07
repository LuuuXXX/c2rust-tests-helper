use anyhow::{bail, Context, Result};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

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
        let selected_file = infer_selected_file_for_module(feature_root, &mod_name, selected_files);

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

fn infer_selected_file_for_module(
    feature_root: &Path,
    mod_name: &str,
    selected_files: &[String],
) -> Option<String> {
    let matches = selected_files
        .iter()
        .filter(|selected_file| {
            module_name_matches_selected_file(feature_root, mod_name, selected_file)
        });
    let matches: Vec<_> = matches.take(2).collect();
    if matches.len() == 1 {
        Some(matches[0].clone())
    } else {
        None
    }
}

fn module_name_matches_selected_file(feature_root: &Path, mod_name: &str, selected_file: &str) -> bool {
    module_name_for_selected_file(feature_root, selected_file)
        .map(|candidate| candidate == mod_name)
        .unwrap_or(false)
}

fn module_name_for_selected_file(feature_root: &Path, selected_file: &str) -> Option<String> {
    let suffix = selected_file_module_key(feature_root, selected_file)?
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(sanitize_component(&part.to_string_lossy())),
            _ => None,
        })
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    if suffix.is_empty() {
        None
    } else {
        Some(format!("mod_{suffix}"))
    }
}

pub(crate) fn selected_file_source_key(feature_root: &Path, selected_file: &str) -> Option<String> {
    let path = selected_file_module_key(feature_root, selected_file)?;
    let extension = path.extension().and_then(OsStr::to_str);
    let path = if extension.is_some() {
        path.with_extension("")
    } else {
        path
    };

    let key = path
        .iter()
        .map(|component| component.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");

    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

fn selected_file_module_key(feature_root: &Path, selected_file: &str) -> Option<PathBuf> {
    let normalized = selected_file.replace('\\', "/");
    let path = Path::new(&normalized);
    let relative = strip_feature_c_prefix(feature_root, path)
        .map(Path::to_path_buf)
        .or_else(|| strip_embedded_feature_c_prefix(feature_root, path))
        .or_else(|| strip_literal_c_prefix(path).map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf());

    let without_c2rust = strip_c2rust_suffix(&relative);
    let module_key = match without_c2rust.extension() {
        Some(_) => without_c2rust.with_extension(""),
        None => without_c2rust,
    };
    Some(module_key)
}

fn strip_feature_c_prefix<'a>(feature_root: &Path, path: &'a Path) -> Option<&'a Path> {
    path.strip_prefix(feature_root.join("c")).ok()
}

fn strip_embedded_feature_c_prefix(feature_root: &Path, path: &Path) -> Option<PathBuf> {
    let feature = feature_root.file_name()?;
    let components: Vec<_> = path.components().collect();
    let needle = [
        OsStr::new(".c2rust"),
        feature,
        OsStr::new("c"),
    ];

    let start = components.windows(needle.len()).position(|window| {
        window
            .iter()
            .zip(needle.iter())
            .all(|(component, needle)| component.as_os_str() == *needle)
    })?;

    let suffix = components.get(start + needle.len()..)?;
    if suffix.is_empty() {
        None
    } else {
        Some(suffix.iter().fold(PathBuf::new(), |mut acc, component| {
            acc.push(component.as_os_str());
            acc
        }))
    }
}

fn strip_literal_c_prefix(path: &Path) -> Option<&Path> {
    path.strip_prefix("c").ok()
}

fn strip_c2rust_suffix(path: &Path) -> PathBuf {
    let file_name = match path.file_name().and_then(OsStr::to_str) {
        Some(name) => name,
        None => return path.to_path_buf(),
    };

    if let Some(stem) = file_name.strip_suffix(".c2rust") {
        let mut result = path.to_path_buf();
        result.set_file_name(stem);
        result
    } else {
        path.to_path_buf()
    }
}

fn sanitize_component(component: &str) -> String {
    let mut out = String::new();
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

    let start = out.find(|ch| ch != '_');
    let end = out.rfind(|ch| ch != '_');

    match (start, end) {
        (Some(start), Some(end)) if start == 0 && end + 1 == out.len() => out,
        (Some(start), Some(end)) => out[start..=end].to_string(),
        _ => String::new(),
    }
}
