// ABOUTME: Renders positioned graph elements as Unicode box-drawing characters.
// ABOUTME: Takes GridNode/GridEdge structs and produces a terminal-ready string.

use crate::grid::{GridEdge, GridNode};
use dot_parser::Attribute;
use std::collections::HashMap;

/// Map a Graphviz shape name to a Unicode icon character.
/// Returns None for shapes that use the default box rendering with no icon.
fn shape_icon(shape: &str) -> Option<char> {
    match shape {
        // Box variants — no icon needed, they render as boxes already.
        "box" | "rect" | "rectangle" | "square" | "record" | "Mrecord" => None,

        // Ellipse variants
        "ellipse" | "oval" => Some('⬭'),
        "circle" => Some('○'),
        "doublecircle" => Some('◎'),
        "point" => Some('●'),

        // Diamond variants
        "diamond" => Some('◇'),
        "Mdiamond" => Some('◆'),

        // Square variants
        "Msquare" => Some('■'),
        "plaintext" | "plain" | "none" => Some('☐'),

        // Triangles
        "triangle" => Some('△'),
        "invtriangle" => Some('▽'),

        // Trapezoids
        "trapezium" => Some('⏢'),
        "invtrapezium" => Some('⏥'),

        // Parallelograms
        "parallelogram" => Some('▱'),

        // Polygons and stars
        "pentagon" => Some('⬠'),
        "hexagon" => Some('⬡'),
        "septagon" | "heptagon" => Some('⬡'),
        "octagon" => Some('⯃'),
        "doubleoctagon" => Some('⯃'),
        "tripleoctagon" => Some('⯃'),
        "star" => Some('★'),

        // Special shapes
        "cylinder" => Some('⌸'),
        "note" => Some('♪'),
        "tab" => Some('⊟'),
        "folder" => Some('⊟'),
        "box3d" | "component" => Some('☐'),
        "house" => Some('⌂'),
        "invhouse" => Some('⌂'),
        "underline" => Some('_'),
        "cds" => Some('▷'),
        "lpromoter" => Some('◁'),
        "rpromoter" => Some('▷'),
        "assembly" => Some('⊞'),
        "signature" => Some('✎'),
        "insulator" => Some('⊘'),
        "ribosite" => Some('◯'),
        "rnastab" => Some('⊗'),
        "proteasesite" => Some('✂'),
        "proteinstab" => Some('⊕'),
        "primersite" => Some('▹'),
        "restrictionsite" => Some('▿'),
        "fivepoverhang" | "threepoverhang" => Some('⌐'),
        "noverhang" => Some('⊣'),
        "larrow" => Some('◁'),
        "rarrow" => Some('▷'),

        // Unknown shapes get a generic marker.
        _ => Some('◈'),
    }
}

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
    attrs: &HashMap<String, Vec<Attribute>>,
    options: &RenderOptions,
) -> String {
    let mut grid = vec![vec![' '; grid_width]; grid_height];

    // Track which cells belong to nodes so edges don't overwrite them.
    let mut node_cells = vec![vec![false; grid_width]; grid_height];

    for node in nodes {
        draw_node(&mut grid, &mut node_cells, node, attrs, options);
    }

    for edge in edges {
        draw_edge(&mut grid, &node_cells, edge);
    }

    // Convert grid to string, trimming trailing empty lines.
    let lines: Vec<String> = grid.iter().map(|row| row.iter().collect::<String>()).collect();
    let trimmed = lines
        .iter()
        .rev()
        .skip_while(|line| line.trim().is_empty())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .cloned()
        .collect::<Vec<String>>();
    trimmed.join("\n")
}

/// Draw a single node onto the grid, using shape-appropriate rendering.
fn draw_node(
    grid: &mut [Vec<char>],
    node_cells: &mut [Vec<bool>],
    node: &GridNode,
    attrs: &HashMap<String, Vec<Attribute>>,
    options: &RenderOptions,
) {
    // Build the label line, prefixed with a shape icon when the shape isn't a plain box.
    let label_line = match shape_icon(&node.shape) {
        Some(icon) => format!("{} {}", icon, node.label),
        None => node.label.clone(),
    };

    let mut content_lines: Vec<String> = vec![label_line];
    if options.verbose {
        if let Some(node_attrs) = attrs.get(&node.name) {
            for attr in node_attrs {
                content_lines.push(format!("{}: {}", attr.key, attr.value));
            }
        }
    }

    draw_box_node(grid, node_cells, node, &content_lines);
}

/// Draw a node as a Unicode box with borders.
fn draw_box_node(
    grid: &mut [Vec<char>],
    node_cells: &mut [Vec<bool>],
    node: &GridNode,
    content_lines: &[String],
) {
    let col = node.col;
    let row = node.row;
    let w = node.width;
    let h = node.height;

    // Top border: ┌─────┐
    set_cell(grid, node_cells, row, col, '┌');
    for c in (col + 1)..(col + w - 1) {
        set_cell(grid, node_cells, row, c, '─');
    }
    set_cell(grid, node_cells, row, col + w - 1, '┐');

    // Middle rows: │ content │
    for r in (row + 1)..(row + h - 1) {
        set_cell(grid, node_cells, r, col, '│');
        set_cell(grid, node_cells, r, col + w - 1, '│');
        for c in (col + 1)..(col + w - 1) {
            set_cell(grid, node_cells, r, c, ' ');
        }
    }

    place_content(grid, node_cells, node, content_lines);

    // Bottom border: └─────┘
    set_cell(grid, node_cells, row + h - 1, col, '└');
    for c in (col + 1)..(col + w - 1) {
        set_cell(grid, node_cells, row + h - 1, c, '─');
    }
    set_cell(grid, node_cells, row + h - 1, col + w - 1, '┘');
}

/// Place content lines centered within a node's box area.
fn place_content(
    grid: &mut [Vec<char>],
    node_cells: &mut [Vec<bool>],
    node: &GridNode,
    content_lines: &[String],
) {
    let col = node.col;
    let row = node.row;
    let w = node.width;
    let h = node.height;
    let inner_width = w.saturating_sub(2);

    let available_rows = h.saturating_sub(2);
    let start_content_row = row + 1 + available_rows.saturating_sub(content_lines.len()) / 2;
    for (i, line) in content_lines.iter().enumerate() {
        let r = start_content_row + i;
        if r >= row + h - 1 {
            break;
        }
        let usable = inner_width.saturating_sub(2);
        let char_count = line.chars().count();
        let truncated: String = if char_count > usable {
            line.chars().take(usable).collect()
        } else {
            line.clone()
        };
        let truncated_chars = truncated.chars().count();
        let left_pad = (usable.saturating_sub(truncated_chars)) / 2;
        for (ci, ch) in truncated.chars().enumerate() {
            let c = col + 1 + 1 + left_pad + ci;
            if c < col + w - 1 {
                set_cell(grid, node_cells, r, c, ch);
            }
        }
    }
}

/// Set a cell in the grid if within bounds, and mark it as a node cell.
fn set_cell(
    grid: &mut [Vec<char>],
    node_cells: &mut [Vec<bool>],
    row: usize,
    col: usize,
    ch: char,
) {
    if row < grid.len() && col < grid[0].len() {
        grid[row][col] = ch;
        node_cells[row][col] = true;
    }
}

/// Draw an edge along its waypoints, with an arrow at the final point.
fn draw_edge(grid: &mut [Vec<char>], node_cells: &[Vec<bool>], edge: &GridEdge) {
    let points = &edge.points;
    if points.is_empty() {
        return;
    }

    // Draw line segments between consecutive waypoints.
    for i in 0..points.len().saturating_sub(1) {
        let (c1, r1) = points[i];
        let (c2, r2) = points[i + 1];
        draw_segment(grid, node_cells, c1, r1, c2, r2);

        // At direction changes, draw corner characters.
        if i + 2 < points.len() {
            let (c3, r3) = points[i + 2];
            if let Some(ch) = pick_corner(c1, r1, c2, r2, c3, r3) {
                set_edge_cell(grid, node_cells, r2, c2, ch);
            }
        }
    }

    // Place arrow at the last point based on direction of final segment.
    if points.len() >= 2 {
        let (cp, rp) = points[points.len() - 2];
        let (cl, rl) = points[points.len() - 1];
        let arrow = if rl > rp {
            '▼'
        } else if rl < rp {
            '▲'
        } else if cl > cp {
            '►'
        } else {
            '◄'
        };
        set_edge_cell(grid, node_cells, rl, cl, arrow);
    }
}

/// Draw a segment between two waypoints. If not axis-aligned, routes as an
/// L-shape: vertical first, then horizontal, with a corner character.
fn draw_segment(
    grid: &mut [Vec<char>],
    node_cells: &[Vec<bool>],
    c1: usize,
    r1: usize,
    c2: usize,
    r2: usize,
) {
    if c1 == c2 {
        // Vertical segment
        let (rmin, rmax) = if r1 < r2 { (r1, r2) } else { (r2, r1) };
        for r in rmin..=rmax {
            set_edge_cell(grid, node_cells, r, c1, '│');
        }
    } else if r1 == r2 {
        // Horizontal segment
        let (cmin, cmax) = if c1 < c2 { (c1, c2) } else { (c2, c1) };
        for c in cmin..=cmax {
            set_edge_cell(grid, node_cells, r1, c, '─');
        }
    } else {
        // L-shaped routing: go vertical first, then horizontal.
        let (rmin, rmax) = if r1 < r2 { (r1, r2) } else { (r2, r1) };
        for r in rmin..=rmax {
            set_edge_cell(grid, node_cells, r, c1, '│');
        }
        let (cmin, cmax) = if c1 < c2 { (c1, c2) } else { (c2, c1) };
        for c in cmin..=cmax {
            set_edge_cell(grid, node_cells, r2, c, '─');
        }
        // Corner at the bend
        let corner = if r2 > r1 && c2 > c1 {
            '└'
        } else if r2 > r1 && c2 < c1 {
            '┘'
        } else if r2 < r1 && c2 > c1 {
            '┌'
        } else {
            '┐'
        };
        set_edge_cell(grid, node_cells, r2, c1, corner);
    }
}

/// Set an edge cell if within bounds and not occupied by a node.
fn set_edge_cell(grid: &mut [Vec<char>], node_cells: &[Vec<bool>], row: usize, col: usize, ch: char) {
    if row < grid.len() && col < grid[0].len() && !node_cells[row][col] {
        grid[row][col] = ch;
    }
}

/// Pick a corner character at a waypoint where direction changes.
fn pick_corner(
    c1: usize,
    r1: usize,
    c2: usize,
    r2: usize,
    c3: usize,
    r3: usize,
) -> Option<char> {
    let going_down = r2 > r1;
    let going_up = r2 < r1;
    let going_right = c2 > c1;
    let going_left = c2 < c1;

    let then_down = r3 > r2;
    let then_up = r3 < r2;
    let then_right = c3 > c2;
    let then_left = c3 < c2;

    // Vertical then horizontal
    if going_down && then_right {
        Some('└')
    } else if going_down && then_left {
        Some('┘')
    } else if going_up && then_right {
        Some('┌')
    } else if going_up && then_left {
        Some('┐')
    // Horizontal then vertical
    } else if going_right && then_down {
        Some('┐')
    } else if going_right && then_up {
        Some('┘')
    } else if going_left && then_down {
        Some('┌')
    } else if going_left && then_up {
        Some('└')
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_single_node() {
        let nodes = vec![GridNode {
            name: "A".into(),
            label: "Hello".into(),
            shape: "box".into(),
            col: 2,
            row: 1,
            width: 9,
            height: 3,
        }];
        let output = render_ascii(
            &nodes,
            &[],
            15,
            5,
            &HashMap::new(),
            &RenderOptions {
                verbose: false,
                color: false,
            },
        );
        assert!(output.contains("Hello"));
        assert!(output.contains("┌"));
        assert!(output.contains("┘"));
    }

    #[test]
    fn test_render_edge_vertical() {
        let nodes = vec![
            GridNode {
                name: "a".into(),
                label: "A".into(),
                shape: "box".into(),
                col: 5,
                row: 0,
                width: 5,
                height: 3,
            },
            GridNode {
                name: "b".into(),
                label: "B".into(),
                shape: "box".into(),
                col: 5,
                row: 6,
                width: 5,
                height: 3,
            },
        ];
        let edges = vec![GridEdge {
            from: "a".into(),
            to: "b".into(),
            points: vec![(7, 3), (7, 4), (7, 5)],
            label: None,
        }];
        let output = render_ascii(
            &nodes,
            &edges,
            15,
            10,
            &HashMap::new(),
            &RenderOptions {
                verbose: false,
                color: false,
            },
        );
        assert!(output.contains('│') || output.contains('▼'));
    }

    #[test]
    fn test_render_verbose_shows_attributes() {
        let nodes = vec![GridNode {
            name: "A".into(),
            label: "Hello".into(),
            shape: "box".into(),
            col: 0,
            row: 0,
            width: 20,
            height: 5,
        }];
        let mut attrs = HashMap::new();
        attrs.insert(
            "A".to_string(),
            vec![Attribute {
                key: "shape".to_string(),
                value: "box".to_string(),
            }],
        );
        let output = render_ascii(
            &nodes,
            &[],
            25,
            7,
            &attrs,
            &RenderOptions {
                verbose: true,
                color: false,
            },
        );
        assert!(output.contains("shape: box"));
    }

    #[test]
    fn test_render_empty_grid() {
        let output = render_ascii(
            &[],
            &[],
            5,
            5,
            &HashMap::new(),
            &RenderOptions {
                verbose: false,
                color: false,
            },
        );
        // Empty grid should produce empty or whitespace-only output.
        assert!(output.trim().is_empty());
    }

    #[test]
    fn test_render_edge_arrow_direction() {
        let nodes = vec![];
        let edges = vec![GridEdge {
            from: "a".into(),
            to: "b".into(),
            points: vec![(3, 1), (3, 4)],
            label: None,
        }];
        let output = render_ascii(
            &nodes,
            &edges,
            8,
            6,
            &HashMap::new(),
            &RenderOptions {
                verbose: false,
                color: false,
            },
        );
        // Edge going down should end with ▼
        assert!(output.contains('▼'));
    }

    #[test]
    fn test_shape_icon_prefix_in_box() {
        let nodes = vec![GridNode {
            name: "start".into(),
            label: "Start".into(),
            shape: "diamond".into(),
            col: 0,
            row: 0,
            width: 14,
            height: 3,
        }];
        let output = render_ascii(
            &nodes,
            &[],
            20,
            5,
            &HashMap::new(),
            &RenderOptions {
                verbose: false,
                color: false,
            },
        );
        // Diamond shape should render as a box with ◇ prefix.
        assert!(output.contains("◇ Start"), "Should have icon prefix, got: {}", output);
        assert!(output.contains("┌"), "Should be in a box");
        assert!(output.contains("┘"), "Should be in a box");
    }

    #[test]
    fn test_box_shape_has_no_icon() {
        let nodes = vec![GridNode {
            name: "a".into(),
            label: "Hello".into(),
            shape: "box".into(),
            col: 0,
            row: 0,
            width: 12,
            height: 3,
        }];
        let output = render_ascii(
            &nodes,
            &[],
            15,
            5,
            &HashMap::new(),
            &RenderOptions {
                verbose: false,
                color: false,
            },
        );
        assert!(output.contains("Hello"), "Should contain label");
        assert!(!output.contains("◇"), "Box shape should have no icon prefix");
    }

    #[test]
    fn test_edges_do_not_overwrite_nodes() {
        let nodes = vec![GridNode {
            name: "a".into(),
            label: "A".into(),
            shape: "box".into(),
            col: 2,
            row: 0,
            width: 5,
            height: 3,
        }];
        // Edge passes through the node area.
        let edges = vec![GridEdge {
            from: "x".into(),
            to: "y".into(),
            points: vec![(4, 0), (4, 5)],
            label: None,
        }];
        let output = render_ascii(
            &nodes,
            &edges,
            10,
            7,
            &HashMap::new(),
            &RenderOptions {
                verbose: false,
                color: false,
            },
        );
        // The top border of the node should still contain ─, not │ from the edge.
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].contains('─'));
    }
}
