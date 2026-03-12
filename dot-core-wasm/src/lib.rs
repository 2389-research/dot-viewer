// ABOUTME: WASM entry point for the DOT parser, used by the web editor.
// ABOUTME: Thin wasm-bindgen wrappers around the shared dot-parser crate.

use wasm_bindgen::prelude::*;

pub use dot_parser::{DotGraph, DotStatement, SourceRange};

/// Parse DOT source into a structured graph model. Returns a JS object with
/// a `statements` array containing tagged statement objects.
#[wasm_bindgen(js_name = "parseDot")]
pub fn parse_dot_wasm(source: &str) -> Result<JsValue, JsValue> {
    let graph = dot_parser::parse_dot(source);
    serde_wasm_bindgen::to_value(&graph)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Returns the node ID at the given cursor offset in the DOT source.
/// Re-parses the source, finds the containing statement, and returns
/// the closest node ID (or null if the offset is in a non-node context).
#[wasm_bindgen(js_name = "nodeIdAtOffset")]
pub fn node_id_at_offset_wasm(source: &str, offset: u32) -> Option<String> {
    let graph = dot_parser::parse_dot(source);
    let stmt = dot_parser::statement_at(&graph, offset)?;
    dot_parser::node_id_at(stmt, offset)
}

/// Returns the source offset of the definition for a given node ID.
/// Prefers explicit node definitions, falls back to the first edge referencing the node.
/// Returns null if the node is not found.
#[wasm_bindgen(js_name = "definitionOffsetForNode")]
pub fn definition_offset_for_node_wasm(source: &str, node_id: &str) -> Option<u32> {
    let graph = dot_parser::parse_dot(source);
    let stmt = dot_parser::definition_for_node(&graph, node_id)?;
    Some(stmt.source_range().location)
}
