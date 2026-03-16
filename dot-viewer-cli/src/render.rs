// ABOUTME: Renders positioned graph elements as Unicode box-drawing characters.
// ABOUTME: Takes GridNode/GridEdge structs and produces a terminal-ready string.

use crate::grid::{GridEdge, GridNode};
use dot_parser::Attribute;
use std::collections::HashMap;

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

/// Draw a single node box onto the grid.
fn draw_node(
    grid: &mut [Vec<char>],
    node_cells: &mut [Vec<bool>],
    node: &GridNode,
    attrs: &HashMap<String, Vec<Attribute>>,
    options: &RenderOptions,
) {
    let col = node.col;
    let row = node.row;
    let w = node.width;
    let h = node.height;

    // Collect content lines: label first, then attributes if verbose.
    let mut content_lines: Vec<String> = vec![node.label.clone()];
    if options.verbose {
        if let Some(node_attrs) = attrs.get(&node.name) {
            for attr in node_attrs {
                content_lines.push(format!("{}: {}", attr.key, attr.value));
            }
        }
    }

    // Top border: ┌─────┐
    set_cell(grid, node_cells, row, col, '┌');
    for c in (col + 1)..(col + w - 1) {
        set_cell(grid, node_cells, row, c, '─');
    }
    set_cell(grid, node_cells, row, col + w - 1, '┐');

    // Middle rows: │ content │
    let inner_width = w.saturating_sub(2); // space between the two │ chars
    for r in (row + 1)..(row + h - 1) {
        set_cell(grid, node_cells, r, col, '│');
        set_cell(grid, node_cells, r, col + w - 1, '│');
        // Fill interior with spaces
        for c in (col + 1)..(col + w - 1) {
            set_cell(grid, node_cells, r, c, ' ');
        }
    }

    // Place content lines centered vertically and horizontally.
    let available_rows = h.saturating_sub(2); // rows between top and bottom border
    let start_content_row = row + 1 + available_rows.saturating_sub(content_lines.len()) / 2;
    for (i, line) in content_lines.iter().enumerate() {
        let r = start_content_row + i;
        if r >= row + h - 1 {
            break;
        }
        // Center horizontally within inner_width, with 1 cell padding on each side.
        let usable = inner_width.saturating_sub(2); // subtract 1 space padding each side
        let truncated: String = if line.len() > usable {
            line.chars().take(usable).collect()
        } else {
            line.clone()
        };
        let left_pad = (usable.saturating_sub(truncated.len())) / 2;
        for (ci, ch) in truncated.chars().enumerate() {
            let c = col + 1 + 1 + left_pad + ci; // +1 border, +1 padding
            if c < col + w - 1 {
                set_cell(grid, node_cells, r, c, ch);
            }
        }
    }

    // Bottom border: └─────┘
    set_cell(grid, node_cells, row + h - 1, col, '└');
    for c in (col + 1)..(col + w - 1) {
        set_cell(grid, node_cells, row + h - 1, c, '─');
    }
    set_cell(grid, node_cells, row + h - 1, col + w - 1, '┘');
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
            let corner = pick_corner(c1, r1, c2, r2, c3, r3);
            if let Some(ch) = corner {
                if r2 < grid.len() && c2 < grid[0].len() && !node_cells[r2][c2] {
                    grid[r2][c2] = ch;
                }
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
        if rl < grid.len() && cl < grid[0].len() && !node_cells[rl][cl] {
            grid[rl][cl] = arrow;
        }
    }
}

/// Draw a straight segment between two waypoints.
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
            if r < grid.len() && c1 < grid[0].len() && !node_cells[r][c1] {
                grid[r][c1] = '│';
            }
        }
    } else if r1 == r2 {
        // Horizontal segment
        let (cmin, cmax) = if c1 < c2 { (c1, c2) } else { (c2, c1) };
        for c in cmin..=cmax {
            if r1 < grid.len() && c < grid[0].len() && !node_cells[r1][c] {
                grid[r1][c] = '─';
            }
        }
    }
    // Diagonal segments are not supported; they would need Bresenham's.
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
                col: 5,
                row: 0,
                width: 5,
                height: 3,
            },
            GridNode {
                name: "b".into(),
                label: "B".into(),
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
    fn test_edges_do_not_overwrite_nodes() {
        let nodes = vec![GridNode {
            name: "a".into(),
            label: "A".into(),
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
