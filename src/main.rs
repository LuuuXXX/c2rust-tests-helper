mod check;
mod cli;
mod collect;
mod manifest;
mod report;
mod runner;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    let result = match cli.command {
        cli::Commands::Collect { config } => collect::run(&config),
        cli::Commands::Check { config, results } => check::run(&config, results),
        cli::Commands::Report { config, results } => report::run(&config, results),
    };

    if let Err(e) = result {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}
