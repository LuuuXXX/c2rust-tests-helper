use std::path::PathBuf;

/// Index of a c2rust-demo feature workspace.
#[derive(Debug)]
pub struct FeatureIndex {
    pub feature_root: PathBuf,
    pub selected_files: Vec<String>,
    pub modules: Vec<FeatureModule>,
}

/// A single generated Rust module within a feature workspace.
#[derive(Debug)]
pub struct FeatureModule {
    pub name: String,
    /// Best-effort source selected file from `meta/selected_files.json` that
    /// produced this module.  It is inferred from the actual c2rust-demo output
    /// so users do not need to duplicate it in `migration.yml`.
    pub selected_file: Option<String>,
    pub functions: Vec<String>,
    pub decls: Vec<String>,
    pub vars: Vec<String>,
}

impl FeatureIndex {
    /// Print a human-readable summary to stdout.
    pub fn print_summary(&self) {
        println!("Feature root : {}", self.feature_root.display());
        println!();

        println!("Selected source files ({}):", self.selected_files.len());
        for f in &self.selected_files {
            println!("  {f}");
        }
        println!();

        println!("Modules ({}):", self.modules.len());
        for m in &self.modules {
            println!("  [{}]", m.name);
            if let Some(selected_file) = &m.selected_file {
                println!("    selected file: {selected_file}");
            }
            if !m.functions.is_empty() {
                println!("    functions ({}):", m.functions.len());
                for f in &m.functions {
                    println!("      {f}");
                }
            }
            if !m.decls.is_empty() {
                println!("    decls ({}):", m.decls.len());
                for d in &m.decls {
                    println!("      {d}");
                }
            }
            if !m.vars.is_empty() {
                println!("    vars ({}):", m.vars.len());
                for v in &m.vars {
                    println!("      {v}");
                }
            }
        }
    }
}
