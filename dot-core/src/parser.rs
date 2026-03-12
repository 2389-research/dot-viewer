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

/// Parse DOT source text into a structured graph model with source ranges.
#[uniffi::export]
pub fn parse_dot(_source: String) -> DotGraph {
    DotGraph { statements: vec![] }
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
    #[ignore] // Enabled in Task 2 when parse logic is implemented
    fn test_simple_digraph_parses_nodes() {
        let graph = parse_dot("digraph G {\n    A\n    B\n}".to_string());
        let node_ids: Vec<&str> = graph.statements.iter().filter_map(|s| {
            if let DotStatement::NodeDefinition { id, .. } = s { Some(id.as_str()) } else { None }
        }).collect();
        assert_eq!(node_ids, vec!["A", "B"]);
    }
}
