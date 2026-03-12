// ABOUTME: UniFFI export wrappers for the shared DOT parser.
// ABOUTME: Re-exports parser types and provides owned-value wrappers for the FFI boundary.

pub use dot_parser::{DotGraph, DotStatement, SourceRange};

/// Parse DOT source text into a structured graph model with source ranges.
#[uniffi::export]
pub fn parse_dot(source: String) -> DotGraph {
    dot_parser::parse_dot(&source)
}

/// Find the statement containing the given character offset.
#[uniffi::export]
pub fn statement_at(graph: &DotGraph, offset: u32) -> Option<DotStatement> {
    dot_parser::statement_at(graph, offset).cloned()
}

/// Returns the node ID relevant to a given cursor offset within a statement.
/// For node definitions, always returns the node ID.
/// For edges, returns whichever node the cursor is closest to.
#[uniffi::export]
pub fn node_id_at(statement: &DotStatement, offset: u32) -> Option<String> {
    dot_parser::node_id_at(statement, offset)
}

/// Find the first node definition for a given node ID, falling back to any edge referencing it.
#[uniffi::export]
pub fn definition_for_node(graph: &DotGraph, node_id: String) -> Option<DotStatement> {
    dot_parser::definition_for_node(graph, &node_id).cloned()
}
