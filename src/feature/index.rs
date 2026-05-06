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
