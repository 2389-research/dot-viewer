// ABOUTME: Maps Graphviz floating-point coordinates to character grid positions.
// ABOUTME: Converts PlainGraph nodes and edges into grid-aligned structs for terminal rendering.

use crate::plain::{PlainEdge, PlainGraph, PlainNode};

/// Horizontal scale: characters per Graphviz inch.
const X_SCALE: f64 = 8.0;

/// Vertical scale: characters per Graphviz inch (terminal chars are ~2x taller than wide).
const Y_SCALE: f64 = 4.0;

/// Padding cells added around the grid edges.
const PADDING: usize = 2;

/// A node mapped to grid coordinates.
#[derive(Debug, Clone)]
pub struct GridNode {
    pub name: String,
    pub label: String,
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

/// Map a PlainGraph to grid-coordinate nodes and edges.
/// Returns (nodes, edges, grid_width, grid_height).
pub fn map_to_grid(graph: &PlainGraph) -> (Vec<GridNode>, Vec<GridEdge>, usize, usize) {
    let grid_w = (graph.width * X_SCALE).ceil() as usize + PADDING * 2;
    let grid_h = (graph.height * Y_SCALE).ceil() as usize + PADDING * 2;

    let nodes = graph
        .nodes
        .iter()
        .map(|n| map_node(n, graph.height))
        .collect();

    let edges = graph
        .edges
        .iter()
        .map(|e| map_edge(e, graph.height))
        .collect();

    (nodes, edges, grid_w, grid_h)
}

/// Convert a PlainNode to a GridNode by scaling coordinates and flipping y.
fn map_node(node: &PlainNode, graph_height: f64) -> GridNode {
    let center_col = (node.x * X_SCALE).round() as usize + PADDING;
    let center_row = ((graph_height - node.y) * Y_SCALE).round() as usize + PADDING;

    let scaled_w = (node.width * X_SCALE).round() as usize;
    let scaled_h = (node.height * Y_SCALE).round() as usize;

    // Minimum width: label + "| " + " |" = label.len() + 4
    let min_width = node.label.len() + 4;
    let width = scaled_w.max(min_width);

    // Minimum height: top border + content + bottom border
    let height = scaled_h.max(3);

    // Top-left corner from center
    let col = center_col.saturating_sub(width / 2);
    let row = center_row.saturating_sub(height / 2);

    GridNode {
        name: node.name.clone(),
        label: node.label.clone(),
        col,
        row,
        width,
        height,
    }
}

/// Convert a PlainEdge to a GridEdge by scaling and flipping each waypoint.
fn map_edge(edge: &PlainEdge, graph_height: f64) -> GridEdge {
    let points = edge
        .points
        .iter()
        .map(|(x, y)| {
            let col = (x * X_SCALE).round() as usize + PADDING;
            let row = ((graph_height - y) * Y_SCALE).round() as usize + PADDING;
            (col, row)
        })
        .collect();

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
                },
                PlainNode {
                    name: "b".into(),
                    x: 1.0,
                    y: 1.0,
                    width: 0.75,
                    height: 0.5,
                    label: "B".into(),
                },
            ],
            edges: vec![],
        };
        let (nodes, _, grid_w, grid_h) = map_to_grid(&graph);
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
            }],
            edges: vec![],
        };
        let (nodes, _, _, _) = map_to_grid(&graph);
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
                },
                PlainNode {
                    name: "b".into(),
                    x: 1.0,
                    y: 1.0,
                    width: 0.75,
                    height: 0.5,
                    label: "B".into(),
                },
            ],
            edges: vec![PlainEdge {
                from: "a".into(),
                to: "b".into(),
                points: vec![(1.0, 2.5), (1.0, 1.5)],
                label: None,
            }],
        };
        let (_, edges, _, _) = map_to_grid(&graph);
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
            }],
            edges: vec![],
        };
        let (nodes, _, _, _) = map_to_grid(&graph);
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
        let (_, _, grid_w, grid_h) = map_to_grid(&graph);
        // 2.0 * 8 + 4 = 20, 3.0 * 4 + 4 = 16
        assert_eq!(grid_w, 20);
        assert_eq!(grid_h, 16);
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
        let (_, edges, _, _) = map_to_grid(&graph);
        assert_eq!(edges[0].label.as_deref(), Some("yes"));
    }
}
