// ABOUTME: CLI entry point for dot-viewer, providing ASCII rendering of DOT files.
// ABOUTME: Uses Graphviz for layout and dot-parser for attribute extraction.

mod grid;
mod plain;
mod render;

use clap::Parser;
use dot_core::{render_dot_plain, LayoutEngine};
use dot_parser::{parse_dot, Attribute, DotStatement};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::grid::{map_to_grid, NodeContent};
use crate::plain::parse_plain;
use crate::render::{render_ascii, RenderOptions};

#[derive(Parser)]
#[command(name = "dot-viewer", about = "Render DOT graph files as ASCII art in the terminal")]
struct Cli {
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
}

fn parse_engine(name: &str) -> LayoutEngine {
    match name {
        "dot" => LayoutEngine::Dot,
        "neato" => LayoutEngine::Neato,
        "fdp" => LayoutEngine::Fdp,
        "circo" => LayoutEngine::Circo,
        "twopi" => LayoutEngine::Twopi,
        "sfdp" => LayoutEngine::Sfdp,
        other => {
            eprintln!("Unknown engine '{}', using dot", other);
            LayoutEngine::Dot
        }
    }
}

fn extract_node_attributes(source: &str) -> HashMap<String, Vec<Attribute>> {
    let graph = parse_dot(source);
    let mut attrs = HashMap::new();
    for stmt in &graph.statements {
        if let DotStatement::NodeDefinition { id, attributes, .. } = stmt {
            attrs.insert(id.clone(), attributes.clone());
        }
    }
    attrs
}

fn main() {
    let cli = Cli::parse();

    let source = std::fs::read_to_string(&cli.file).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", cli.file.display(), e);
        std::process::exit(1);
    });

    let layout_engine = parse_engine(&cli.engine);

    // Get Graphviz plain format layout
    let plain_output = render_dot_plain(source.clone(), layout_engine).unwrap_or_else(|e| {
        eprintln!("Graphviz error: {:?}", e);
        std::process::exit(1);
    });

    // Parse plain format into positioned elements
    let plain_graph = parse_plain(&plain_output).unwrap_or_else(|e| {
        eprintln!("Plain format parse error: {}", e);
        std::process::exit(1);
    });

    // Extract attributes from DOT source
    let node_attrs = extract_node_attributes(&source);

    // Build extra content for verbose mode
    let extra_content: HashMap<String, NodeContent> = if cli.verbose {
        node_attrs
            .iter()
            .map(|(id, attrs)| {
                let lines = attrs
                    .iter()
                    .map(|a| format!("{}: {}", a.key, a.value))
                    .collect();
                (id.clone(), NodeContent { lines })
            })
            .collect()
    } else {
        HashMap::new()
    };

    // Map to character grid
    let (nodes, edges, grid_w, grid_h) = map_to_grid(&plain_graph, &extra_content);

    // Render to ASCII
    let output = render_ascii(
        &nodes,
        &edges,
        grid_w,
        grid_h,
        &node_attrs,
        &RenderOptions {
            verbose: cli.verbose,
            color: cli.color,
        },
    );

    print!("{}", output);
}
