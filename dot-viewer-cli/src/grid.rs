// ABOUTME: Maps Graphviz floating-point coordinates to character grid positions.
// ABOUTME: Converts PlainGraph nodes and edges into grid-aligned structs for terminal rendering.

use crate::plain::{PlainEdge, PlainGraph, PlainNode};
use std::collections::HashMap;

/// Horizontal scale: characters per Graphviz inch.
const X_SCALE: f64 = 8.0;

/// Vertical scale: characters per Graphviz inch (terminal chars are ~2x taller than wide).
const Y_SCALE: f64 = 6.0;

/// Padding cells added around the grid edges.
const PADDING: usize = 2;

/// Maximum node width in characters (prevents verbose content from exploding boxes).
const MAX_NODE_WIDTH: usize = 40;

/// A node mapped to grid coordinates.
#[derive(Debug, Clone)]
pub struct GridNode {
    pub name: String,
    pub label: String,
    pub shape: String,
    pub col: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
}

/// An edge mapped to grid coordinates.
#[derive(Debug, Clone)]
pub struct GridEdge {
    pub from: String,
    pub to: String,
    pub points: Vec<(usize, usize)>,
    pub label: Option<String>,
}

/// Extra content to display inside a node beyond its label.
pub struct NodeContent {
    pub lines: Vec<String>,
}

/// Minimum vertical gap (in rows) between nodes after overlap resolution.
/// Provides space for edge arrows to be visible between boxes.
const MIN_NODE_GAP: usize = 2;

/// Map a PlainGraph to grid-coordinate nodes and edges.
/// Returns (nodes, edges, grid_width, grid_height).
/// `extra_content` provides additional lines per node for verbose display.
pub fn map_to_grid(
    graph: &PlainGraph,
    extra_content: &HashMap<String, NodeContent>,
) -> (Vec<GridNode>, Vec<GridEdge>, usize, usize) {
    let grid_w = (graph.width * X_SCALE).ceil() as usize + PADDING * 2;
    let grid_h = (graph.height * Y_SCALE).ceil() as usize + PADDING * 2;

    let mut nodes: Vec<GridNode> = graph
        .nodes
        .iter()
        .map(|n| {
            let content = extra_content.get(&n.name);
            let extra_lines = content.map_or(0, |c| c.lines.len());
            let max_line_width = content.map_or(0, |c| c.lines.iter().map(|l| l.len()).max().unwrap_or(0));
            map_node(n, graph.height, extra_lines, max_line_width)
        })
        .collect();

    let mut edges: Vec<GridEdge> = graph
        .edges
        .iter()
        .map(|e| map_edge(e, graph.height))
        .collect();

    // Resolve vertical overlaps caused by verbose mode expanding node heights
    // beyond what Graphviz allocated space for.
    resolve_overlaps(&mut nodes);

    // Snap edge endpoints to actual node boundaries so edges clearly connect
    // to their source and target boxes.
    snap_edges_to_nodes(&mut edges, &nodes);

    // Expand grid to fit all nodes (verbose mode may make nodes wider than Graphviz expects).
    let mut actual_w: usize = grid_w;
    let mut actual_h: usize = grid_h;
    for n in &nodes {
        actual_w = actual_w.max(n.col + n.width + PADDING);
        actual_h = actual_h.max(n.row + n.height + PADDING);
    }

    (nodes, edges, actual_w, actual_h)
}

/// Resolve vertical overlaps between nodes by shifting overlapping nodes down.
fn resolve_overlaps(nodes: &mut [GridNode]) {
    if nodes.len() < 2 {
        return;
    }

    // Sort node indices by row (top to bottom), breaking ties by column.
    let mut order: Vec<usize> = (0..nodes.len()).collect();
    order.sort_by_key(|&i| (nodes[i].row, nodes[i].col));

    // Walk top-to-bottom. For each node, check against all earlier nodes
    // for vertical overlap (they might be side-by-side and not actually overlap).
    for pass_idx in 1..order.len() {
        let curr = order[pass_idx];
        let mut max_needed_shift: usize = 0;

        for prev_idx in 0..pass_idx {
            let prev = order[prev_idx];

            // Check horizontal overlap: do these nodes share any columns?
            let prev_left = nodes[prev].col;
            let prev_right = nodes[prev].col + nodes[prev].width;
            let curr_left = nodes[curr].col;
            let curr_right = nodes[curr].col + nodes[curr].width;

            let h_overlap = prev_left < curr_right && curr_left < prev_right;
            if !h_overlap {
                continue;
            }

            // Check vertical overlap/closeness.
            let prev_bottom = nodes[prev].row + nodes[prev].height;
            let required_top = prev_bottom + MIN_NODE_GAP;

            if nodes[curr].row < required_top {
                let shift = required_top - nodes[curr].row;
                max_needed_shift = max_needed_shift.max(shift);
            }
        }

        if max_needed_shift > 0 {
            nodes[curr].row += max_needed_shift;
        }
    }
}

/// Snap edge endpoints to actual node boundaries so edges clearly originate
/// from the bottom of their source node and terminate at the top of their target.
fn snap_edges_to_nodes(edges: &mut [GridEdge], nodes: &[GridNode]) {
    let node_map: HashMap<&str, &GridNode> = nodes.iter().map(|n| (n.name.as_str(), n)).collect();

    for edge in edges.iter_mut() {
        if edge.points.is_empty() {
            continue;
        }

        let from_node = node_map.get(edge.from.as_str());
        let to_node = node_map.get(edge.to.as_str());

        // Determine if this is a top-to-bottom or bottom-to-top edge based
        // on relative node positions.
        let from_above = match (from_node, to_node) {
            (Some(f), Some(t)) => f.row <= t.row,
            _ => true,
        };

        // Snap start point to the source node boundary.
        if let Some(from) = from_node {
            let col = from.col + from.width / 2;
            let row = if from_above {
                from.row + from.height // bottom edge
            } else {
                from.row.saturating_sub(1) // top edge
            };
            edge.points[0] = (col, row);
        }

        // Snap end point to the target node boundary.
        if edge.points.len() >= 2 {
            if let Some(to) = to_node {
                let last = edge.points.len() - 1;
                let col = to.col + to.width / 2;
                let row = if from_above {
                    to.row.saturating_sub(1) // top edge (arrow points down into node)
                } else {
                    to.row + to.height // bottom edge (arrow points up into node)
                };
                edge.points[last] = (col, row);
            }
        }
    }
}

/// Convert a PlainNode to a GridNode by scaling coordinates and flipping y.
/// `extra_lines` adds rows for verbose attribute display.
/// `extra_width` is the max width of extra content lines.
fn map_node(node: &PlainNode, graph_height: f64, extra_lines: usize, extra_width: usize) -> GridNode {
    let center_col = (node.x * X_SCALE).round() as usize + PADDING;
    let center_row = ((graph_height - node.y) * Y_SCALE).round() as usize + PADDING;

    let scaled_w = (node.width * X_SCALE).round() as usize;
    let scaled_h = (node.height * Y_SCALE).round() as usize;

    // Minimum width: max of label and extra content lines, plus border padding.
    // Capped at MAX_NODE_WIDTH to prevent long attributes from exploding boxes.
    // Non-box shapes get a 2-char icon prefix ("X ") rendered by the renderer.
    let icon_width = match node.shape.as_str() {
        "box" | "rect" | "rectangle" | "square" | "record" | "Mrecord" => 0,
        _ => 2,
    };
    let content_width = (node.label.len() + icon_width).max(extra_width);
    let min_width = (content_width + 4).min(MAX_NODE_WIDTH); // "│ " + content + " │"
    let width = scaled_w.max(min_width).min(MAX_NODE_WIDTH);

    // Minimum height: top border + label + extra attribute lines + bottom border
    let min_height = 3 + extra_lines;
    let height = scaled_h.max(min_height);

    // Top-left corner from center
    let col = center_col.saturating_sub(width / 2);
    let row = center_row.saturating_sub(height / 2);

    GridNode {
        name: node.name.clone(),
        label: node.label.clone(),
        shape: node.shape.clone(),
        col,
        row,
        width,
        height,
    }
}

/// Convert a PlainEdge to a GridEdge by simplifying Bezier spline points
/// to just the start and end positions. The renderer draws straight lines
/// between these, which produces cleaner output than mapping every control point.
fn map_edge(edge: &PlainEdge, graph_height: f64) -> GridEdge {
    let map_point = |(x, y): &(f64, f64)| -> (usize, usize) {
        let col = (x * X_SCALE).round() as usize + PADDING;
        let row = ((graph_height - y) * Y_SCALE).round() as usize + PADDING;
        (col, row)
    };

    // Use only start and end of spline for clean straight-line routing.
    let points = if edge.points.len() >= 2 {
        let start = map_point(&edge.points[0]);
        let end = map_point(edge.points.last().unwrap());
        if start == end {
            vec![start]
        } else {
            vec![start, end]
        }
    } else {
        edge.points.iter().map(|p| map_point(p)).collect()
    };

    GridEdge {
        from: edge.from.clone(),
        to: edge.to.clone(),
        points,
        label: edge.label.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_vertical_nodes_mapped() {
        let graph = PlainGraph {
            width: 2.0,
            height: 4.0,
            nodes: vec![
                PlainNode {
                    name: "a".into(),
                    x: 1.0,
                    y: 3.0,
                    width: 0.75,
                    height: 0.5,
                    label: "A".into(),
                    shape: "ellipse".into(),
                },
                PlainNode {
                    name: "b".into(),
                    x: 1.0,
                    y: 1.0,
                    width: 0.75,
                    height: 0.5,
                    label: "B".into(),
                    shape: "ellipse".into(),
                },
            ],
            edges: vec![],
        };
        let (nodes, _, grid_w, grid_h) = map_to_grid(&graph, &HashMap::new());
        assert_eq!(nodes.len(), 2);
        // Node a has higher y in graphviz = lower row in terminal (higher on screen)
        assert!(nodes[0].row < nodes[1].row, "a should be above b");
        assert!(grid_w > 0);
        assert!(grid_h > 0);
    }

    #[test]
    fn test_node_dimensions_reasonable() {
        let graph = PlainGraph {
            width: 3.0,
            height: 3.0,
            nodes: vec![PlainNode {
                name: "a".into(),
                x: 1.5,
                y: 1.5,
                width: 1.0,
                height: 0.5,
                label: "Hello".into(),
                shape: "ellipse".into(),
            }],
            edges: vec![],
        };
        let (nodes, _, _, _) = map_to_grid(&graph, &HashMap::new());
        // Node should be at least as wide as its label + border
        assert!(nodes[0].width >= 7); // "Hello" (5) + borders (2)
        assert!(nodes[0].height >= 3); // top + content + bottom
    }

    #[test]
    fn test_edge_points_mapped() {
        let graph = PlainGraph {
            width: 2.0,
            height: 4.0,
            nodes: vec![
                PlainNode {
                    name: "a".into(),
                    x: 1.0,
                    y: 3.0,
                    width: 0.75,
                    height: 0.5,
                    label: "A".into(),
                    shape: "ellipse".into(),
                },
                PlainNode {
                    name: "b".into(),
                    x: 1.0,
                    y: 1.0,
                    width: 0.75,
                    height: 0.5,
                    label: "B".into(),
                    shape: "ellipse".into(),
                },
            ],
            edges: vec![PlainEdge {
                from: "a".into(),
                to: "b".into(),
                points: vec![(1.0, 2.5), (1.0, 1.5)],
                label: None,
            }],
        };
        let (_, edges, _, _) = map_to_grid(&graph, &HashMap::new());
        assert_eq!(edges.len(), 1);
        assert!(edges[0].points.len() >= 2);
    }

    #[test]
    fn test_y_axis_flipped() {
        let graph = PlainGraph {
            width: 2.0,
            height: 4.0,
            nodes: vec![PlainNode {
                name: "top".into(),
                x: 1.0,
                y: 4.0,
                width: 0.75,
                height: 0.5,
                label: "T".into(),
                shape: "ellipse".into(),
            }],
            edges: vec![],
        };
        let (nodes, _, _, _) = map_to_grid(&graph, &HashMap::new());
        // y=4.0 in a height=4.0 graph means graphviz top -> terminal row near 0
        assert!(nodes[0].row <= PADDING + 1);
    }

    #[test]
    fn test_grid_dimensions_include_padding() {
        let graph = PlainGraph {
            width: 2.0,
            height: 3.0,
            nodes: vec![],
            edges: vec![],
        };
        let (_, _, grid_w, grid_h) = map_to_grid(&graph, &HashMap::new());
        // 2.0 * 8 + 4 = 20, 3.0 * 6 + 4 = 22
        assert_eq!(grid_w, 20);
        assert_eq!(grid_h, 22);
    }

    #[test]
    fn test_overlapping_nodes_resolved() {
        // Two nodes positioned so close that with verbose content they'd overlap.
        let graph = PlainGraph {
            width: 2.0,
            height: 2.0,
            nodes: vec![
                PlainNode {
                    name: "a".into(),
                    x: 1.0,
                    y: 1.8,
                    width: 0.75,
                    height: 0.5,
                    label: "A".into(),
                    shape: "ellipse".into(),
                },
                PlainNode {
                    name: "b".into(),
                    x: 1.0,
                    y: 1.2,
                    width: 0.75,
                    height: 0.5,
                    label: "B".into(),
                    shape: "ellipse".into(),
                },
            ],
            edges: vec![],
        };
        // Give both nodes extra verbose content to make them tall.
        let mut extra = HashMap::new();
        extra.insert("a".into(), NodeContent {
            lines: vec!["attr1: val1".into(), "attr2: val2".into(), "attr3: val3".into()],
        });
        extra.insert("b".into(), NodeContent {
            lines: vec!["attr4: val4".into(), "attr5: val5".into()],
        });
        let (nodes, _, _, _) = map_to_grid(&graph, &extra);
        // Node a should be above node b with no overlap.
        let a = &nodes[0];
        let b = &nodes[1];
        let a_bottom = a.row + a.height;
        assert!(
            a_bottom + MIN_NODE_GAP <= b.row,
            "Nodes should not overlap: a bottom {} + gap {} > b top {}",
            a_bottom, MIN_NODE_GAP, b.row,
        );
    }

    #[test]
    fn test_edges_snap_to_node_boundaries() {
        let graph = PlainGraph {
            width: 2.0,
            height: 4.0,
            nodes: vec![
                PlainNode {
                    name: "a".into(),
                    x: 1.0,
                    y: 3.0,
                    width: 0.75,
                    height: 0.5,
                    label: "A".into(),
                    shape: "ellipse".into(),
                },
                PlainNode {
                    name: "b".into(),
                    x: 1.0,
                    y: 1.0,
                    width: 0.75,
                    height: 0.5,
                    label: "B".into(),
                    shape: "ellipse".into(),
                },
            ],
            edges: vec![PlainEdge {
                from: "a".into(),
                to: "b".into(),
                points: vec![(1.0, 2.5), (1.0, 1.5)],
                label: None,
            }],
        };
        let (nodes, edges, _, _) = map_to_grid(&graph, &HashMap::new());
        let a = &nodes[0];
        let b = &nodes[1];
        let edge = &edges[0];

        // Start point should be at the bottom center of node a.
        let (start_col, start_row) = edge.points[0];
        assert_eq!(start_col, a.col + a.width / 2, "start col should be center of node a");
        assert_eq!(start_row, a.row + a.height, "start row should be bottom of node a");

        // End point should be just above node b (arrow row).
        let (end_col, end_row) = edge.points[edge.points.len() - 1];
        assert_eq!(end_col, b.col + b.width / 2, "end col should be center of node b");
        assert_eq!(end_row, b.row.saturating_sub(1), "end row should be just above node b");
    }

    #[test]
    fn test_edge_label_preserved() {
        let graph = PlainGraph {
            width: 2.0,
            height: 2.0,
            nodes: vec![],
            edges: vec![PlainEdge {
                from: "a".into(),
                to: "b".into(),
                points: vec![(1.0, 1.0)],
                label: Some("yes".into()),
            }],
        };
        let (_, edges, _, _) = map_to_grid(&graph, &HashMap::new());
        assert_eq!(edges[0].label.as_deref(), Some("yes"));
    }
}
