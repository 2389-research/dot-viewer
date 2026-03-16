// ABOUTME: CLI entry point for dot-viewer, providing ASCII rendering of DOT files.
// ABOUTME: Uses Graphviz for layout and dot-parser for attribute extraction.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dot-viewer", about = "View DOT graph files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a DOT file as ASCII art
    Ascii {
        /// Path to the .dot file
        file: PathBuf,
        /// Show all node attributes
        #[arg(short, long)]
        verbose: bool,
        /// Enable ANSI colors
        #[arg(long)]
        color: bool,
        /// Graphviz layout engine
        #[arg(long, default_value = "dot")]
        engine: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Ascii {
            file,
            verbose,
            color,
            engine,
        } => {
            let source = std::fs::read_to_string(&file).unwrap_or_else(|e| {
                eprintln!("Error reading {}: {}", file.display(), e);
                std::process::exit(1);
            });
            println!(
                "TODO: render {} ({} bytes, verbose={}, color={}, engine={})",
                file.display(),
                source.len(),
                verbose,
                color,
                engine
            );
        }
    }
}
