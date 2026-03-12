// ABOUTME: DOT language parser that produces a structured model with source ranges.
// ABOUTME: Used for bidirectional mapping between editor cursor positions and graph elements.

/// A range of characters in the source text (offset + length).
#[derive(Debug, Clone, uniffi::Record)]
pub struct SourceRange {
    pub location: u32,
    pub length: u32,
}

/// A parsed DOT statement with source location tracking.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum DotStatement {
    NodeDefinition {
        id: String,
        source_range: SourceRange,
    },
    Edge {
        from: String,
        to: String,
        source_range: SourceRange,
        from_range: SourceRange,
        to_range: SourceRange,
    },
    GraphAttribute {
        source_range: SourceRange,
    },
}

/// The result of parsing a DOT source string.
#[derive(Debug, Clone, uniffi::Record)]
pub struct DotGraph {
    pub statements: Vec<DotStatement>,
}

// -- Scanning Helpers --

/// Returns true if the byte is a valid DOT identifier character (alphanumeric or underscore).
fn is_ident_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Skip whitespace (space, tab, newline, carriage return) and semicolons.
fn skip_whitespace_and_semicolons(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' | b';' => i += 1,
            _ => break,
        }
    }
    i
}

/// Skip whitespace only (space, tab, newline, carriage return).
fn skip_whitespace_only(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            _ => break,
        }
    }
    i
}

/// Skip to the end of the current line (past the newline character).
fn skip_to_end_of_line(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    if i < bytes.len() {
        i += 1; // skip the newline itself
    }
    i
}

/// Skip a block comment starting at `/*`. Returns position after `*/`.
fn skip_block_comment(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 2; // skip past /*
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return i + 2;
        }
        i += 1;
    }
    bytes.len() // unterminated comment
}

/// Extract an identifier starting at the given position. Handles both bare
/// identifiers (alphanumeric + underscore) and double-quoted identifiers.
/// Returns (id, id_range, position_after_identifier) or (None, None, start) if none found.
fn extract_identifier(bytes: &[u8], start: usize) -> (Option<String>, Option<SourceRange>, usize) {
    if start >= bytes.len() {
        return (None, None, start);
    }

    let ch = bytes[start];

    if ch == b'"' {
        // Quoted identifier
        let mut i = start + 1;
        while i < bytes.len() {
            let c = bytes[i];
            if c == b'\\' && i + 1 < bytes.len() {
                i += 2; // skip escaped character
                continue;
            }
            if c == b'"' {
                // The ID is the content without quotes for matching purposes
                let id_range = SourceRange {
                    location: start as u32,
                    length: (i + 1 - start) as u32,
                };
                let content = String::from_utf8_lossy(&bytes[start + 1..i]).to_string();
                return (Some(content), Some(id_range), i + 1);
            }
            i += 1;
        }
        // Unterminated quote — treat as not an identifier
        return (None, None, start);
    }

    if is_ident_char(ch) {
        let mut i = start;
        while i < bytes.len() && is_ident_char(bytes[i]) {
            i += 1;
        }
        let id_range = SourceRange {
            location: start as u32,
            length: (i - start) as u32,
        };
        let id = String::from_utf8_lossy(&bytes[start..i]).to_string();
        return (Some(id), Some(id_range), i);
    }

    (None, None, start)
}

/// Find the end of a statement starting at the given position.
/// Tracks bracket depth, string literals, and comments to find the boundary.
fn find_statement_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    let mut bracket_depth: i32 = 0;
    let mut in_string = false;

    while i < bytes.len() {
        let ch = bytes[i];

        // Handle string literals
        if ch == b'"' && !in_string {
            in_string = true;
            i += 1;
            continue;
        }
        if in_string {
            if ch == b'\\' && i + 1 < bytes.len() {
                i += 2; // skip escape
                continue;
            }
            if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Handle comments inside statements
        if ch == b'/' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            if next == b'/' {
                // Line comment ends the statement visually, but the statement
                // range should stop before the comment
                if bracket_depth == 0 {
                    return i;
                }
                i = skip_to_end_of_line(bytes, i);
                continue;
            }
            if next == b'*' {
                i = skip_block_comment(bytes, i);
                continue;
            }
        }

        // Track bracket depth
        if ch == b'[' {
            bracket_depth += 1;
            i += 1;
            continue;
        }
        if ch == b']' {
            bracket_depth -= 1;
            if bracket_depth <= 0 {
                return i + 1; // include the closing bracket
            }
            i += 1;
            continue;
        }

        // Statement boundaries (only at bracket depth 0)
        if bracket_depth == 0 {
            if ch == b'\n' || ch == b';' || ch == b'{' || ch == b'}' {
                return i;
            }
        }

        i += 1;
    }
    bytes.len()
}

/// Parse DOT source text into a structured graph model with source ranges.
#[uniffi::export]
pub fn parse_dot(source: String) -> DotGraph {
    let bytes = source.as_bytes();
    let length = bytes.len();
    let mut statements: Vec<DotStatement> = Vec::new();

    // Global keywords that are never node identifiers
    let attribute_keywords = ["graph", "node", "edge"];
    let skip_keywords = ["digraph", "subgraph", "strict"];

    let mut i = 0;

    while i < length {
        // Skip whitespace and semicolons between statements
        i = skip_whitespace_and_semicolons(bytes, i);
        if i >= length {
            break;
        }

        let ch = bytes[i];

        // Skip comments
        if ch == b'/' && i + 1 < length {
            let next = bytes[i + 1];
            if next == b'/' {
                // Line comment — skip to end of line
                i = skip_to_end_of_line(bytes, i);
                continue;
            } else if next == b'*' {
                // Block comment — skip to closing */
                i = skip_block_comment(bytes, i);
                continue;
            }
        }

        // Opening/closing braces — skip them as statement boundaries
        if ch == b'{' || ch == b'}' {
            i += 1;
            continue;
        }

        // Try to parse a statement starting at this position
        let stmt_start = i;

        // Extract the first identifier (or skip if not an identifier character)
        if !is_ident_char(bytes[i]) && bytes[i] != b'"' {
            // Not a statement start we recognize — skip to next boundary
            i = find_statement_end(bytes, i);
            continue;
        }

        let (first_id, first_id_range, after_first) = extract_identifier(bytes, i);
        let first_id = match first_id {
            Some(id) => id,
            None => {
                i = find_statement_end(bytes, i);
                continue;
            }
        };

        // Check if this is a keyword
        let keyword_lower = first_id.to_ascii_lowercase();

        if attribute_keywords.contains(&keyword_lower.as_str()) {
            // These can have attribute lists: `graph [rankdir=LR]`
            let stmt_end = find_statement_end(bytes, stmt_start);
            let range = SourceRange {
                location: stmt_start as u32,
                length: (stmt_end - stmt_start) as u32,
            };
            statements.push(DotStatement::GraphAttribute { source_range: range });
            i = stmt_end;
            continue;
        }

        if skip_keywords.contains(&keyword_lower.as_str()) {
            // digraph, subgraph, strict — skip to the brace or end
            i = find_statement_end(bytes, after_first);
            continue;
        }

        // We have a non-keyword identifier. Scan ahead to classify the statement.
        let scan_pos = skip_whitespace_only(bytes, after_first);

        // Check for edge operator (-> or --)
        if scan_pos + 1 < length {
            let c1 = bytes[scan_pos];
            let c2 = bytes[scan_pos + 1];
            let is_arrow = (c1 == b'-' && c2 == b'>') || (c1 == b'-' && c2 == b'-');
            if is_arrow {
                // Edge statement
                let after_arrow = scan_pos + 2;
                let post_arrow = skip_whitespace_only(bytes, after_arrow);
                let (second_id, second_id_range, _) = extract_identifier(bytes, post_arrow);

                let stmt_end = find_statement_end(bytes, stmt_start);
                let range = SourceRange {
                    location: stmt_start as u32,
                    length: (stmt_end - stmt_start) as u32,
                };

                if let (Some(second_id), Some(second_id_range), Some(first_id_range)) =
                    (second_id, second_id_range, first_id_range)
                {
                    statements.push(DotStatement::Edge {
                        from: first_id,
                        to: second_id,
                        source_range: range,
                        from_range: first_id_range,
                        to_range: second_id_range,
                    });
                } else {
                    // Malformed edge — treat as node definition
                    statements.push(DotStatement::NodeDefinition {
                        id: first_id,
                        source_range: range,
                    });
                }
                i = stmt_end;
                continue;
            }
        }

        // Not an edge — it's a node definition (possibly with attributes)
        let stmt_end = find_statement_end(bytes, stmt_start);
        let range = SourceRange {
            location: stmt_start as u32,
            length: (stmt_end - stmt_start) as u32,
        };
        statements.push(DotStatement::NodeDefinition {
            id: first_id,
            source_range: range,
        });
        i = stmt_end;
    }

    DotGraph { statements }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string_produces_no_statements() {
        let graph = parse_dot("".to_string());
        assert!(graph.statements.is_empty());
    }

    #[test]
    fn test_whitespace_only_produces_no_statements() {
        let graph = parse_dot("   \n\t\n  ".to_string());
        assert!(graph.statements.is_empty());
    }

    #[test]
    fn test_simple_digraph_parses_nodes() {
        let graph = parse_dot("digraph G {\n    A\n    B\n}".to_string());
        let node_ids: Vec<&str> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::NodeDefinition { id, .. } = s { Some(id.as_str()) } else { None }
        }).collect();
        assert_eq!(node_ids, vec!["A", "B"]);
    }

    #[test]
    fn test_simple_edge() {
        let graph = parse_dot("digraph G {\n    A -> B\n}".to_string());
        let edges: Vec<(&str, &str)> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::Edge { from, to, .. } = s { Some((from.as_str(), to.as_str())) } else { None }
        }).collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0], ("A", "B"));
    }

    #[test]
    fn test_undirected_edge() {
        let graph = parse_dot("graph G {\n    A -- B\n}".to_string());
        let edges: Vec<(&str, &str)> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::Edge { from, to, .. } = s { Some((from.as_str(), to.as_str())) } else { None }
        }).collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0], ("A", "B"));
    }

    #[test]
    fn test_node_with_attributes() {
        let graph = parse_dot("digraph G {\n    A [label=\"Hello\" shape=box]\n}".to_string());
        let node_ids: Vec<&str> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::NodeDefinition { id, .. } = s { Some(id.as_str()) } else { None }
        }).collect();
        assert_eq!(node_ids, vec!["A"]);
    }

    #[test]
    fn test_edge_with_attributes() {
        let graph = parse_dot("digraph G {\n    A -> B [label=\"edge\" color=red]\n}".to_string());
        let edges: Vec<(&str, &str)> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::Edge { from, to, .. } = s { Some((from.as_str(), to.as_str())) } else { None }
        }).collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0], ("A", "B"));
    }

    #[test]
    fn test_graph_attribute_statement() {
        let graph = parse_dot("digraph G {\n    graph [rankdir=LR]\n    A -> B\n}".to_string());
        let has_graph_attr = graph.statements.iter().any(|s| matches!(s, DotStatement::GraphAttribute { .. }));
        assert!(has_graph_attr);
    }

    #[test]
    fn test_node_keyword_as_graph_attribute() {
        let graph = parse_dot("digraph G {\n    node [shape=box]\n    A\n}".to_string());
        let has_graph_attr = graph.statements.iter().any(|s| matches!(s, DotStatement::GraphAttribute { .. }));
        assert!(has_graph_attr);
        let node_ids: Vec<&str> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::NodeDefinition { id, .. } = s { Some(id.as_str()) } else { None }
        }).collect();
        assert_eq!(node_ids, vec!["A"]);
    }

    #[test]
    fn test_quoted_node_identifier() {
        let graph = parse_dot("digraph G {\n    \"my node\" -> \"other node\"\n}".to_string());
        let edges: Vec<(&str, &str)> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::Edge { from, to, .. } = s { Some((from.as_str(), to.as_str())) } else { None }
        }).collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0], ("my node", "other node"));
    }

    #[test]
    fn test_underscored_node_name() {
        let graph = parse_dot("digraph G {\n    my_node -> other_node\n}".to_string());
        let edges: Vec<(&str, &str)> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::Edge { from, to, .. } = s { Some((from.as_str(), to.as_str())) } else { None }
        }).collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0], ("my_node", "other_node"));
    }

    #[test]
    fn test_line_comment_ignored() {
        let graph = parse_dot("digraph G {\n    // this is a comment\n    A -> B\n}".to_string());
        let edges: Vec<_> = graph.statements.iter().filter(|s| matches!(s, DotStatement::Edge { .. })).collect();
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_block_comment_ignored() {
        let graph = parse_dot("digraph G {\n    /* multi\n       line comment */\n    A -> B\n}".to_string());
        let edges: Vec<_> = graph.statements.iter().filter(|s| matches!(s, DotStatement::Edge { .. })).collect();
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_multiple_edges() {
        let graph = parse_dot("digraph G {\n    A -> B\n    B -> C\n    C -> A\n}".to_string());
        let edges: Vec<_> = graph.statements.iter().filter(|s| matches!(s, DotStatement::Edge { .. })).collect();
        assert_eq!(edges.len(), 3);
    }

    #[test]
    fn test_mixed_nodes_and_edges() {
        let graph = parse_dot("digraph G {\n    A [shape=box]\n    B [shape=circle]\n    A -> B [label=\"connects\"]\n}".to_string());
        let nodes: Vec<&str> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::NodeDefinition { id, .. } = s { Some(id.as_str()) } else { None }
        }).collect();
        let edges: Vec<_> = graph.statements.iter().filter(|s| matches!(s, DotStatement::Edge { .. })).collect();
        assert_eq!(nodes, vec!["A", "B"]);
        assert_eq!(edges.len(), 1);
    }

    #[test]
    fn test_semicolon_separated_statements() {
        let graph = parse_dot("digraph G { A; B; A -> B }".to_string());
        let node_ids: Vec<&str> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::NodeDefinition { id, .. } = s { Some(id.as_str()) } else { None }
        }).collect();
        assert!(node_ids.contains(&"A"));
        assert!(node_ids.contains(&"B"));
    }

    #[test]
    fn test_digraph_keyword_not_parsed_as_node() {
        let graph = parse_dot("digraph G { A }".to_string());
        let node_ids: Vec<&str> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::NodeDefinition { id, .. } = s { Some(id.as_str()) } else { None }
        }).collect();
        assert!(!node_ids.contains(&"digraph"));
        assert!(!node_ids.contains(&"G"));
        assert!(node_ids.contains(&"A"));
    }

    #[test]
    fn test_strict_keyword_ignored() {
        let graph = parse_dot("strict digraph G { A -> B }".to_string());
        let node_ids: Vec<&str> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::NodeDefinition { id, .. } = s { Some(id.as_str()) } else { None }
        }).collect();
        assert!(!node_ids.contains(&"strict"));
    }
}
