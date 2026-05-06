/// CLI definitions using clap.
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "c2rust-tests-helper",
    version,
    about = "A tool for managing the migration of C tests to Rust FFI-based tests.",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Scan C source files and merge discovered test entries into the manifest.
    Collect {
        /// Path to the manifest/config YAML file.
        #[arg(long, short, default_value = "helper.yml")]
        config: std::path::PathBuf,
    },
    /// Validate the manifest, run C and Rust test commands, and print a report.
    Check {
        /// Path to the manifest/config YAML file.
        #[arg(long, short, default_value = "helper.yml")]
        config: std::path::PathBuf,
        /// Path to write run results (default: <config-stem>-results.yml).
        #[arg(long)]
        results: Option<std::path::PathBuf>,
    },
    /// Print a summarized migration report from the manifest and last run results.
    Report {
        /// Path to the manifest/config YAML file.
        #[arg(long, short, default_value = "helper.yml")]
        config: std::path::PathBuf,
        /// Path to a run results file produced by `check` (optional).
        #[arg(long)]
        results: Option<std::path::PathBuf>,
    },
}
