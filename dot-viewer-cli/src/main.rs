// ABOUTME: CLI entry point for dot-viewer, providing ASCII rendering of DOT files.
// ABOUTME: Uses Graphviz for layout and dot-parser for attribute extraction.

mod grid;
mod plain;
mod render;

use clap::{Parser, ValueEnum};
use dot_core::{render_dot_plain, LayoutEngine};
use dot_parser::{parse_dot, Attribute, DotStatement};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::grid::{map_to_grid, NodeContent};
use crate::plain::parse_plain;
use crate::render::{render_ascii, RenderOptions};

const EX_USAGE: i32 = 64;
const EX_DATAERR: i32 = 65;
const EX_NOINPUT: i32 = 66;

#[derive(Parser)]
#[command(
    name = "dot-viewer",
    about = "Render DOT and Dippin graph files as ASCII art in the terminal",
    long_about = "dot-viewer reads Graphviz DOT (.dot, .gv) and Dippin (.dip) files,\n\
                  lays them out with the chosen Graphviz engine, and renders the result\n\
                  as ASCII art in your terminal. The format is auto-detected from the\n\
                  file extension; override with --format.",
    after_help = "EXAMPLES:\n\
                  \x20\x20dot-viewer graph.dot\n\
                  \x20\x20dot-viewer workflow.dip --engine dot\n\
                  \x20\x20dot-viewer workflow.dip --show-dot\n\
                  \x20\x20cat workflow.dip | dot-viewer --format dip -"
)]
struct Cli {
    /// Path to a .dot or .dip file
    file: PathBuf,
    /// Show all node attributes
    #[arg(short, long)]
    verbose: bool,
    /// Graphviz layout engine
    #[arg(long, value_enum, default_value_t = Engine::Dot)]
    engine: Engine,
    /// Force input format. Auto detects from the file extension.
    #[arg(long, value_enum, default_value_t = Format::Auto)]
    format: Format,
    /// Print the converted DOT source to stdout instead of rendering it.
    #[arg(long)]
    show_dot: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum Format {
    #[default]
    Auto,
    Dot,
    Dip,
}

// NOTE: dot-core's LayoutEngine only supports these six engines, so Patchwork
// and Osage from the original plan are intentionally omitted here.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Engine {
    Dot,
    Neato,
    Fdp,
    Sfdp,
    Twopi,
    Circo,
}

impl From<Engine> for LayoutEngine {
    fn from(e: Engine) -> Self {
        match e {
            Engine::Dot => LayoutEngine::Dot,
            Engine::Neato => LayoutEngine::Neato,
            Engine::Fdp => LayoutEngine::Fdp,
            Engine::Sfdp => LayoutEngine::Sfdp,
            Engine::Twopi => LayoutEngine::Twopi,
            Engine::Circo => LayoutEngine::Circo,
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

/// Detect .dip files and convert to DOT format before rendering.
fn resolve_dot_source(file: &std::path::Path, raw_source: &str, format: Format) -> String {
    let is_dip = match format {
        Format::Dip => true,
        Format::Dot => false,
        Format::Auto => file
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("dip"))
            .unwrap_or(false),
    };
    if is_dip {
        match dippin_parser::parse_to_dot(raw_source, file) {
            Ok(s) => s,
            Err(e) => {
                for diag in e.diagnostics() {
                    eprintln!("{}", diag.render());
                }
                if e.diagnostics().is_empty() {
                    eprintln!("dippin parse error: {}", e);
                }
                std::process::exit(EX_DATAERR);
            }
        }
    } else {
        raw_source.to_string()
    }
}

fn main() {
    let cli = Cli::parse();

    let from_stdin = cli.file.as_os_str() == "-";

    let raw_source = if from_stdin {
        // Stdin doesn't have an extension to auto-detect from, so the user
        // must tell us what they're piping in.
        if matches!(cli.format, Format::Auto) {
            eprintln!("error: --format is required when reading from stdin");
            std::process::exit(EX_USAGE);
        }
        let mut buf = String::new();
        if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf) {
            eprintln!("error: cannot read stdin: {}", e);
            std::process::exit(EX_NOINPUT);
        }
        if buf.len() > dippin_parser::MAX_INPUT_SIZE {
            eprintln!(
                "error: input exceeds maximum size of {} bytes",
                dippin_parser::MAX_INPUT_SIZE
            );
            std::process::exit(EX_DATAERR);
        }
        buf
    } else {
        // Reject pathologically large inputs up front so the parser never sees
        // them. The limit matches dippin_parser::MAX_INPUT_SIZE.
        let metadata = std::fs::metadata(&cli.file).unwrap_or_else(|e| {
            eprintln!("error: cannot stat {}: {}", cli.file.display(), e);
            std::process::exit(EX_NOINPUT);
        });
        if metadata.len() as usize > dippin_parser::MAX_INPUT_SIZE {
            eprintln!(
                "error: file exceeds maximum size of {} bytes",
                dippin_parser::MAX_INPUT_SIZE
            );
            std::process::exit(EX_DATAERR);
        }

        std::fs::read_to_string(&cli.file).unwrap_or_else(|e| {
            eprintln!("Error reading {}: {}", cli.file.display(), e);
            std::process::exit(EX_NOINPUT);
        })
    };

    let source = resolve_dot_source(&cli.file, &raw_source, cli.format);

    if cli.show_dot {
        print!("{}", source);
        return;
    }

    let layout_engine: LayoutEngine = cli.engine.into();

    // Get Graphviz plain format layout
    let plain_output = render_dot_plain(source.clone(), layout_engine).unwrap_or_else(|e| {
        eprintln!("Graphviz error: {}", e);
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
        },
    );

    print!("{}", output);
}
