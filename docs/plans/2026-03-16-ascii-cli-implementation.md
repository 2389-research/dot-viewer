# ASCII CLI Renderer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `dot-viewer ascii` CLI command that renders DOT files as Unicode box diagrams, using Graphviz for layout and the shared parser for attribute extraction.

**Architecture:** Graphviz renders DOT to `plain` format (positioned coordinates). The dot-parser extracts node/edge attributes. A grid mapper converts float coordinates to character cells. An ASCII renderer draws boxes, lines, and arrows to stdout.

**Tech Stack:** Rust, dot-parser (extended with `attributes` feature), dot-core (Graphviz `plain` format), clap (CLI args)

---

### Task 1: Add `attributes` feature flag to dot-parser

**Files:**
- Modify: `dot-parser/Cargo.toml`
- Modify: `dot-parser/src/lib.rs`

**Step 1: Write the failing test**

Add to the `#[cfg(test)]` module at the bottom of `dot-parser/src/lib.rs`:

```rust
#[test]
fn test_parse_node_attributes() {
    let dot = r#"digraph G { A [shape=box label="Hello"] }"#;
    let graph = parse_dot(dot);
    let node = graph.statements.iter().find(|s| matches!(s, DotStatement::NodeDefinition { id, .. } if id == "A"));
    assert!(node.is_some());
    if let Some(DotStatement::NodeDefinition { attributes, .. }) = node {
        assert!(attributes.iter().any(|(k, v)| k == "shape" && v == "box"));
        assert!(attributes.iter().any(|(k, v)| k == "label" && v == "Hello"));
    } else {
        panic!("expected NodeDefinition");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd dot-parser && cargo test test_parse_node_attributes -- --nocapture`
Expected: FAIL — `DotStatement::NodeDefinition` has no `attributes` field

**Step 3: Add the `attributes` feature and field**

In `dot-parser/Cargo.toml`, add:

```toml
[features]
default = []
attributes = []
uniffi = ["dep:uniffi"]
serde = ["dep:serde"]
```

In `dot-parser/src/lib.rs`, update the `DotStatement` enum variants to conditionally include attributes:

```rust
pub enum DotStatement {
    NodeDefinition {
        id: String,
        source_range: SourceRange,
        #[cfg(feature = "attributes")]
        attributes: Vec<(String, String)>,
    },
    Edge {
        from: String,
        to: String,
        source_range: SourceRange,
        from_range: SourceRange,
        to_range: SourceRange,
        #[cfg(feature = "attributes")]
        attributes: Vec<(String, String)>,
    },
    GraphAttribute {
        source_range: SourceRange,
        #[cfg(feature = "attributes")]
        attributes: Vec<(String, String)>,
    },
}
```

Add a helper function `parse_attributes` that extracts key=value pairs from a `[...]` block within a source range:

```rust
#[cfg(feature = "attributes")]
fn parse_attributes(bytes: &[u8], start: usize, end: usize) -> Vec<(String, String)> {
    // Find the '[' within the range
    // For each key=value or key="value" pair, extract and collect
    // Handle: bare values, quoted values, angle-bracket HTML values
    // Return empty vec if no '[' found
}
```

Call `parse_attributes` when constructing each statement variant (only when `attributes` feature is enabled). Pass the statement's byte range so it can find the `[...]` block.

Update all statement construction sites to include the `attributes` field when the feature is active, and ensure tests compile without it.

**Step 4: Run test to verify it passes**

Run: `cd dot-parser && cargo test --features attributes test_parse_node_attributes -- --nocapture`
Expected: PASS

**Step 5: Verify existing tests still pass without the feature**

Run: `cd dot-parser && cargo test`
Expected: All 31 existing tests PASS (attributes field not present)

**Step 6: Commit**

```bash
git add dot-parser/Cargo.toml dot-parser/src/lib.rs
git commit -m "feat(dot-parser): add attributes feature flag with attribute extraction"
```

---

### Task 2: Comprehensive attribute parsing tests

**Files:**
- Modify: `dot-parser/src/lib.rs` (test module)

**Step 1: Write additional attribute tests**

Add these tests to the `#[cfg(test)]` module (all gated behind `#[cfg(feature = "attributes")]`):

```rust
#[cfg(feature = "attributes")]
#[test]
fn test_parse_edge_attributes() {
    let dot = r#"digraph G { A -> B [label="goes to" color=red] }"#;
    let graph = parse_dot(dot);
    let edge = graph.statements.iter().find(|s| matches!(s, DotStatement::Edge { .. }));
    if let Some(DotStatement::Edge { attributes, .. }) = edge {
        assert!(attributes.iter().any(|(k, v)| k == "label" && v == "goes to"));
        assert!(attributes.iter().any(|(k, v)| k == "color" && v == "red"));
    } else {
        panic!("expected Edge with attributes");
    }
}

#[cfg(feature = "attributes")]
#[test]
fn test_parse_graph_attributes() {
    let dot = r#"digraph G { graph [rankdir=LR goal="test"] }"#;
    let graph = parse_dot(dot);
    let attr = graph.statements.iter().find(|s| matches!(s, DotStatement::GraphAttribute { .. }));
    if let Some(DotStatement::GraphAttribute { attributes, .. }) = attr {
        assert!(attributes.iter().any(|(k, v)| k == "rankdir" && v == "LR"));
        assert!(attributes.iter().any(|(k, v)| k == "goal" && v == "test"));
    } else {
        panic!("expected GraphAttribute with attributes");
    }
}

#[cfg(feature = "attributes")]
#[test]
fn test_node_without_attributes_has_empty_vec() {
    let dot = "digraph G { A -> B }";
    let graph = parse_dot(dot);
    if let Some(DotStatement::Edge { attributes, .. }) = graph.statements.first() {
        assert!(attributes.is_empty());
    }
}

#[cfg(feature = "attributes")]
#[test]
fn test_multiline_node_attributes() {
    let dot = "digraph G {\n  A [\n    shape=box,\n    label=\"Hello World\"\n  ]\n}";
    let graph = parse_dot(dot);
    if let Some(DotStatement::NodeDefinition { attributes, .. }) = graph.statements.first() {
        assert!(attributes.iter().any(|(k, v)| k == "shape" && v == "box"));
        assert!(attributes.iter().any(|(k, v)| k == "label" && v == "Hello World"));
    } else {
        panic!("expected NodeDefinition");
    }
}

#[cfg(feature = "attributes")]
#[test]
fn test_attribute_with_escaped_quotes() {
    let dot = r#"digraph G { A [label="say \"hello\""] }"#;
    let graph = parse_dot(dot);
    if let Some(DotStatement::NodeDefinition { attributes, .. }) = graph.statements.first() {
        assert!(attributes.iter().any(|(k, v)| k == "label" && v.contains("hello")));
    }
}

#[cfg(feature = "attributes")]
#[test]
fn test_real_pipeline_node() {
    let dot = r#"digraph G {
  PickSecret [
    shape=box,
    label="Pick a Secret",
    llm_provider="anthropic",
    llm_model="claude-sonnet-4-6",
    prompt="Pick a thing"
  ]
}"#;
    let graph = parse_dot(dot);
    if let Some(DotStatement::NodeDefinition { attributes, .. }) = graph.statements.first() {
        assert!(attributes.iter().any(|(k, v)| k == "shape" && v == "box"));
        assert!(attributes.iter().any(|(k, v)| k == "llm_model" && v == "claude-sonnet-4-6"));
        assert!(attributes.iter().any(|(k, v)| k == "llm_provider" && v == "anthropic"));
    } else {
        panic!("expected NodeDefinition");
    }
}
```

**Step 2: Run tests**

Run: `cd dot-parser && cargo test --features attributes`
Expected: All new + existing tests PASS

**Step 3: Commit**

```bash
git add dot-parser/src/lib.rs
git commit -m "test(dot-parser): add comprehensive attribute parsing tests"
```

---

### Task 3: Add `plain` format output to dot-core

**Files:**
- Modify: `dot-core/src/lib.rs`
- Modify: `dot-core/src/graphviz.rs`

**Step 1: Write the failing test**

Add to `dot-core/src/lib.rs` test module:

```rust
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_render_plain_format() {
    let dot = "digraph { a -> b }".to_string();
    let plain = render_dot_plain(dot, LayoutEngine::Dot).unwrap();
    assert!(plain.starts_with("graph"));
    assert!(plain.contains("node a"));
    assert!(plain.contains("node b"));
    assert!(plain.contains("edge a b"));
    assert!(plain.contains("stop"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd dot-core && cargo test test_render_plain_format`
Expected: FAIL — `render_dot_plain` does not exist

**Step 3: Implement `render_dot_plain`**

In `dot-core/src/graphviz.rs`, add a new function `render_to_plain` that is identical to `render_to_svg` except it uses `"plain"` instead of `"svg"` for the format string passed to `gvRenderData`. Refactor: extract the common rendering logic into a private `render_to_format` function that both `render_to_svg` and `render_to_plain` call:

```rust
fn render_to_format(dot_source: &str, engine: &LayoutEngine, format: &str) -> Result<String, DotError> {
    // Same as current render_to_svg but uses `format` parameter
    // for the CString passed to gvRenderData
}

pub fn render_to_svg(dot_source: &str, engine: &LayoutEngine) -> Result<String, DotError> {
    render_to_format(dot_source, engine, "svg")
}

pub fn render_to_plain(dot_source: &str, engine: &LayoutEngine) -> Result<String, DotError> {
    render_to_format(dot_source, engine, "plain")
}
```

In `dot-core/src/lib.rs`, add the new public export:

```rust
#[cfg(not(target_arch = "wasm32"))]
#[uniffi::export]
pub fn render_dot_plain(dot_source: String, engine: LayoutEngine) -> Result<String, DotError> {
    graphviz::render_to_plain(&dot_source, &engine)
}
```

**Step 4: Run test to verify it passes**

Run: `cd dot-core && cargo test test_render_plain_format`
Expected: PASS

**Step 5: Verify all existing tests still pass**

Run: `cd dot-core && cargo test`
Expected: All tests PASS (render_to_svg unchanged)

**Step 6: Commit**

```bash
git add dot-core/src/lib.rs dot-core/src/graphviz.rs
git commit -m "feat(dot-core): add plain format rendering via render_dot_plain"
```

---

### Task 4: Scaffold the dot-viewer-cli crate

**Files:**
- Create: `dot-viewer-cli/Cargo.toml`
- Create: `dot-viewer-cli/src/main.rs`

**Step 1: Create Cargo.toml**

```toml
# ABOUTME: Cargo manifest for the dot-viewer CLI tool.
# ABOUTME: Provides ASCII rendering of DOT files using Graphviz layout.

[package]
name = "dot-viewer-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "dot-viewer"
path = "src/main.rs"

[dependencies]
dot-parser = { path = "../dot-parser", features = ["attributes"] }
dot-core = { path = "../dot-core" }
clap = { version = "4", features = ["derive"] }
```

**Step 2: Create minimal main.rs**

```rust
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
        Commands::Ascii { file, verbose, color, engine } => {
            let source = std::fs::read_to_string(&file)
                .unwrap_or_else(|e| {
                    eprintln!("Error reading {}: {}", file.display(), e);
                    std::process::exit(1);
                });
            println!("TODO: render {} ({} nodes)", file.display(), source.len());
        }
    }
}
```

**Step 3: Build and run**

Run: `cd dot-viewer-cli && cargo build`
Expected: Compiles successfully (this will also compile dot-core, which requires the Graphviz C build — may take a while on first build)

Run: `cd dot-viewer-cli && cargo run -- ascii ../dotmaker/docs/nested-graph-test/twenty_questions.dot`
Expected: Prints the TODO placeholder

**Step 4: Commit**

```bash
git add dot-viewer-cli/
git commit -m "feat(cli): scaffold dot-viewer-cli crate with clap arg parsing"
```

---

### Task 5: Plain format parser

**Files:**
- Create: `dot-viewer-cli/src/plain.rs`
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Write tests for the plain format parser**

Create `dot-viewer-cli/src/plain.rs`:

```rust
// ABOUTME: Parser for Graphviz plain text output format.
// ABOUTME: Extracts positioned node and edge data for ASCII rendering.

/// A node with position and size from Graphviz plain format.
#[derive(Debug, Clone)]
pub struct PlainNode {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub label: String,
}

/// An edge with spline points from Graphviz plain format.
#[derive(Debug, Clone)]
pub struct PlainEdge {
    pub from: String,
    pub to: String,
    pub points: Vec<(f64, f64)>,
    pub label: Option<String>,
}

/// Parsed Graphviz plain format output.
#[derive(Debug)]
pub struct PlainGraph {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<PlainNode>,
    pub edges: Vec<PlainEdge>,
}

pub fn parse_plain(input: &str) -> Result<PlainGraph, String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_PLAIN: &str = "\
graph 1 2.75 4.5
node a 1.375 4.0694 0.75 0.5 a solid ellipse black lightgrey
node b 1.375 3.0694 0.75 0.5 b solid ellipse black lightgrey
edge a b 4 1.375 3.8195 1.375 3.7114 1.375 3.5813 1.375 3.4612 solid black
stop
";

    #[test]
    fn test_parse_plain_nodes() {
        let graph = parse_plain(SAMPLE_PLAIN).unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[0].name, "a");
        assert_eq!(graph.nodes[1].name, "b");
    }

    #[test]
    fn test_parse_plain_edges() {
        let graph = parse_plain(SAMPLE_PLAIN).unwrap();
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].from, "a");
        assert_eq!(graph.edges[0].to, "b");
        assert_eq!(graph.edges[0].points.len(), 4);
    }

    #[test]
    fn test_parse_plain_dimensions() {
        let graph = parse_plain(SAMPLE_PLAIN).unwrap();
        assert!((graph.width - 2.75).abs() < 0.001);
        assert!((graph.height - 4.5).abs() < 0.001);
    }

    #[test]
    fn test_parse_plain_with_labels() {
        let input = "\
graph 1 3 5
node a 1.5 4 0.75 0.5 \"Hello World\" solid box black lightgrey
edge a b 2 1.5 3.5 1.5 2.5 \"my label\" solid black
stop
";
        let graph = parse_plain(input).unwrap();
        assert_eq!(graph.nodes[0].label, "Hello World");
        assert_eq!(graph.edges[0].label.as_deref(), Some("my label"));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd dot-viewer-cli && cargo test test_parse_plain`
Expected: FAIL — `todo!()` panics

**Step 3: Implement `parse_plain`**

Parse line by line:
- `graph <scale> <width> <height>` — extract dimensions (multiply by scale)
- `node <name> <x> <y> <w> <h> <label> <style> <shape> <color> <fillcolor>` — extract node
- `edge <from> <to> <n> <x1> <y1> ... <xn> <yn> [label] <style> <color>` — extract edge with n spline points
- `stop` — end

Handle quoted labels (split respecting quotes). Multiply all coordinates by graph scale.

**Step 4: Run tests to verify they pass**

Run: `cd dot-viewer-cli && cargo test test_parse_plain`
Expected: All PASS

**Step 5: Commit**

```bash
git add dot-viewer-cli/src/plain.rs dot-viewer-cli/src/main.rs
git commit -m "feat(cli): add Graphviz plain format parser"
```

---

### Task 6: Grid mapper (float coordinates to character grid)

**Files:**
- Create: `dot-viewer-cli/src/grid.rs`
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Write tests**

Create `dot-viewer-cli/src/grid.rs`:

```rust
// ABOUTME: Maps Graphviz floating-point coordinates to a character grid.
// ABOUTME: Handles coordinate scaling, node placement, and edge routing.

use crate::plain::{PlainGraph, PlainNode, PlainEdge};

/// A cell in the character grid.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Empty,
    NodeChar(char),       // Part of a node box
    EdgeChar(char),       // Part of an edge line
    Text(char),           // Label text
}

/// The character grid with positioned content.
#[derive(Debug)]
pub struct CharGrid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<Cell>>,
}

/// A node mapped to grid coordinates.
#[derive(Debug, Clone)]
pub struct GridNode {
    pub name: String,
    pub label: String,
    pub col: usize,       // left column
    pub row: usize,       // top row
    pub width: usize,     // columns wide
    pub height: usize,    // rows tall
}

/// An edge mapped to grid coordinates.
#[derive(Debug, Clone)]
pub struct GridEdge {
    pub from: String,
    pub to: String,
    pub points: Vec<(usize, usize)>,  // (col, row) waypoints
    pub label: Option<String>,
}

/// Map a PlainGraph to grid-coordinate nodes and edges.
pub fn map_to_grid(graph: &PlainGraph) -> (Vec<GridNode>, Vec<GridEdge>, usize, usize) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plain::*;

    #[test]
    fn test_two_vertical_nodes_mapped() {
        let graph = PlainGraph {
            width: 2.0,
            height: 4.0,
            nodes: vec![
                PlainNode { name: "a".into(), x: 1.0, y: 3.0, width: 0.75, height: 0.5, label: "A".into() },
                PlainNode { name: "b".into(), x: 1.0, y: 1.0, width: 0.75, height: 0.5, label: "B".into() },
            ],
            edges: vec![],
        };
        let (nodes, _, grid_w, grid_h) = map_to_grid(&graph);
        assert_eq!(nodes.len(), 2);
        // Node a should be above node b (lower row number = higher on screen)
        assert!(nodes[0].row < nodes[1].row, "a should be above b");
        assert!(grid_w > 0);
        assert!(grid_h > 0);
    }

    #[test]
    fn test_node_dimensions_reasonable() {
        let graph = PlainGraph {
            width: 3.0,
            height: 3.0,
            nodes: vec![
                PlainNode { name: "a".into(), x: 1.5, y: 1.5, width: 1.0, height: 0.5, label: "Hello".into() },
            ],
            edges: vec![],
        };
        let (nodes, _, _, _) = map_to_grid(&graph);
        // Node should be at least as wide as its label + border
        assert!(nodes[0].width >= 7); // "Hello" (5) + borders (2)
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd dot-viewer-cli && cargo test test_ -- grid`
Expected: FAIL — `todo!()` panics

**Step 3: Implement `map_to_grid`**

Key logic:
- Choose a scale factor: ~8 chars per Graphviz inch horizontally, ~4 chars per inch vertically (chars are ~2x taller than wide)
- Graphviz y-axis goes up; terminal y-axis goes down — flip y coordinates
- Map each node center to grid (col, row), compute box dimensions from width/height
- Ensure node box is at least wide enough for its label
- Map edge spline points to grid coordinates
- Simplify spline points to straight segments between waypoints
- Return total grid dimensions

**Step 4: Run tests to verify they pass**

Run: `cd dot-viewer-cli && cargo test test_ -- grid`
Expected: All PASS

**Step 5: Commit**

```bash
git add dot-viewer-cli/src/grid.rs dot-viewer-cli/src/main.rs
git commit -m "feat(cli): add coordinate-to-grid mapper"
```

---

### Task 7: ASCII renderer (grid to string output)

**Files:**
- Create: `dot-viewer-cli/src/render.rs`
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Write tests**

Create `dot-viewer-cli/src/render.rs`:

```rust
// ABOUTME: Renders positioned graph elements as Unicode box-drawing characters.
// ABOUTME: Supports node boxes, edge lines with arrows, and optional ANSI colors.

use crate::grid::{GridNode, GridEdge};

/// Rendering options.
pub struct RenderOptions {
    pub verbose: bool,
    pub color: bool,
}

/// Render nodes and edges to a string of Unicode box-drawing characters.
pub fn render_ascii(
    nodes: &[GridNode],
    edges: &[GridEdge],
    grid_width: usize,
    grid_height: usize,
    _attrs: &std::collections::HashMap<String, Vec<(String, String)>>,
    options: &RenderOptions,
) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_single_node() {
        let nodes = vec![GridNode {
            name: "A".into(),
            label: "Hello".into(),
            col: 2,
            row: 1,
            width: 9,   // "│ Hello │" = 9
            height: 3,  // top border + content + bottom border
        }];
        let output = render_ascii(&nodes, &[], 15, 5, &Default::default(), &RenderOptions { verbose: false, color: false });
        assert!(output.contains("Hello"));
        assert!(output.contains("┌"));
        assert!(output.contains("┘"));
    }

    #[test]
    fn test_render_edge_arrow() {
        let nodes = vec![
            GridNode { name: "a".into(), label: "A".into(), col: 5, row: 0, width: 5, height: 3 },
            GridNode { name: "b".into(), label: "B".into(), col: 5, row: 6, width: 5, height: 3 },
        ];
        let edges = vec![GridEdge {
            from: "a".into(),
            to: "b".into(),
            points: vec![(7, 3), (7, 4), (7, 5)],
            label: None,
        }];
        let output = render_ascii(&nodes, &edges, 15, 10, &Default::default(), &RenderOptions { verbose: false, color: false });
        // Should contain vertical edge characters
        assert!(output.contains("│") || output.contains("|"));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd dot-viewer-cli && cargo test test_render`
Expected: FAIL — `todo!()` panics

**Step 3: Implement `render_ascii`**

Key logic:
1. Create a 2D `Vec<Vec<char>>` grid filled with spaces
2. Draw each node as a box:
   - `┌─────┐` top border
   - `│ Label │` content line(s)
   - `└─────┘` bottom border
   - Special shapes: `◆` for Mdiamond, `■` for Msquare, `◇` for diamond (render inline instead of box)
3. Draw each edge:
   - Walk through the waypoints
   - Use `│` for vertical segments, `─` for horizontal
   - Use `┌ ┐ └ ┘` for corners
   - Place arrow heads: `▼` `▲` `►` `◄` at the terminal point
4. Convert the grid to a `String` with newlines
5. If `options.color` is true, wrap node borders and labels in ANSI escape codes
6. If `options.verbose` is true, add extra content lines inside node boxes for attributes

**Step 4: Run tests to verify they pass**

Run: `cd dot-viewer-cli && cargo test test_render`
Expected: All PASS

**Step 5: Commit**

```bash
git add dot-viewer-cli/src/render.rs dot-viewer-cli/src/main.rs
git commit -m "feat(cli): add ASCII box-drawing renderer"
```

---

### Task 8: Wire everything together in main.rs

**Files:**
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Implement the full pipeline**

Update `main.rs` to wire the components:

```rust
use dot_core::{render_dot_plain, LayoutEngine};
use dot_parser::parse_dot;

// In the Ascii command handler:
// 1. Read the file
// 2. Parse with dot-parser (for attributes)
// 3. Render to plain format with dot-core
// 4. Parse the plain output
// 5. Map to grid
// 6. Render ASCII
// 7. Print to stdout
```

Map the `--engine` string to `LayoutEngine` enum. Collect attributes from the parsed `DotGraph` into a `HashMap<String, Vec<(String, String)>>` keyed by node ID, pass to the renderer.

Handle errors gracefully: file not found, parse errors, Graphviz errors — print to stderr and exit with code 1.

**Step 2: Test end-to-end**

Run: `cd dot-viewer-cli && cargo run -- ascii ../../dotmaker/docs/nested-graph-test/twenty_questions.dot`
Expected: ASCII rendering of the twenty questions pipeline

Run: `cd dot-viewer-cli && echo "digraph { a -> b -> c }" > /tmp/test.dot && cargo run -- ascii /tmp/test.dot`
Expected: Simple three-node ASCII graph

Run: `cd dot-viewer-cli && cargo run -- ascii --color ../../dotmaker/docs/nested-graph-test/twenty_questions.dot`
Expected: Same graph with ANSI color codes

Run: `cd dot-viewer-cli && cargo run -- ascii -v ../../dotmaker/docs/nested-graph-test/twenty_questions.dot`
Expected: Same graph with full attribute details inside boxes

**Step 3: Commit**

```bash
git add dot-viewer-cli/src/main.rs
git commit -m "feat(cli): wire ASCII rendering pipeline end-to-end"
```

---

### Task 9: Update consumers for attributes feature

**Files:**
- Modify: `dot-core/Cargo.toml`
- Modify: `dot-core-wasm/Cargo.toml`
- Modify: `dot-core-wasm/src/lib.rs`
- Modify: `web/src/lib/wasm.ts`

**Step 1: Enable attributes in dot-core**

In `dot-core/Cargo.toml`, update the dot-parser dependency:

```toml
dot-parser = { path = "../dot-parser", features = ["uniffi", "attributes"] }
```

**Step 2: Enable attributes in dot-core-wasm**

In `dot-core-wasm/Cargo.toml`, update the dot-parser dependency:

```toml
dot-parser = { path = "../dot-parser", features = ["serde", "attributes"] }
```

**Step 3: Build and verify all crates compile**

Run: `cd dot-core && cargo build`
Expected: Compiles (UniFFI bindings may need regeneration if DotStatement shape changed)

Run: `cd dot-core-wasm && cargo build --target wasm32-unknown-unknown`
Expected: Compiles

**Step 4: Update WASM TypeScript types**

In `web/src/lib/wasm.ts`, add the `attributes` field to the TypeScript interfaces:

```typescript
export interface NodeDefinition {
    type: "NodeDefinition";
    id: string;
    source_range: SourceRange;
    attributes: [string, string][];
}

export interface Edge {
    type: "Edge";
    from: string;
    to: string;
    source_range: SourceRange;
    from_range: SourceRange;
    to_range: SourceRange;
    attributes: [string, string][];
}

export interface GraphAttribute {
    type: "GraphAttribute";
    source_range: SourceRange;
    attributes: [string, string][];
}
```

**Step 5: Verify web still builds and tests pass**

Run: `cd web && npm run build && npx playwright test`
Expected: All 13 tests PASS

**Step 6: Regenerate UniFFI bindings and verify macOS build**

Run: `cd dot-viewer && make generate-bindings && xcodebuild -project DotViewer/DotViewer.xcodeproj -scheme DotViewer -configuration Debug build`
Expected: Build succeeds

**Step 7: Commit**

```bash
git add dot-core/Cargo.toml dot-core-wasm/Cargo.toml dot-core-wasm/src/lib.rs web/src/lib/wasm.ts
git commit -m "feat: enable attributes feature across all consumers"
```

---

### Task 10: Integration tests and Makefile

**Files:**
- Modify: `Makefile`
- Modify: `dot-viewer-cli/src/main.rs` (or a tests/ directory)

**Step 1: Add snapshot integration test**

Create an integration test that renders a known DOT string and checks the output contains expected elements:

```rust
#[cfg(test)]
mod integration_tests {
    #[test]
    fn test_simple_graph_ascii_output() {
        // Use dot-core and the full pipeline to render a simple graph
        let dot = "digraph { a -> b -> c }";
        // ... run through pipeline ...
        // Assert output contains node labels and arrow characters
    }
}
```

**Step 2: Add Makefile targets**

Add to the project `Makefile`:

```makefile
build-cli:
	cd dot-viewer-cli && cargo build --release

test-cli:
	cd dot-viewer-cli && cargo test

install-cli:
	cd dot-viewer-cli && cargo install --path .
```

**Step 3: Run everything**

Run: `make test-cli`
Expected: All tests PASS

Run: `make build-cli`
Expected: Release binary builds

**Step 4: Commit**

```bash
git add Makefile dot-viewer-cli/
git commit -m "feat(cli): add integration tests and Makefile targets"
```
