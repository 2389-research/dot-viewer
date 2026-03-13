// ABOUTME: WASM entry point for the DOT parser, used by the web editor.
// ABOUTME: Thin wasm-bindgen wrappers matching the same API as the UniFFI exports.

use wasm_bindgen::prelude::*;

pub use dot_parser::{DotGraph, DotStatement, SourceRange};

/// Parse DOT source text into a structured graph model.
#[wasm_bindgen(js_name = "parseDot")]
pub fn parse_dot_wasm(source: &str) -> Result<JsValue, JsValue> {
    let graph = dot_parser::parse_dot(source);
    serde_wasm_bindgen::to_value(&graph)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Find the statement containing the given character offset.
/// Returns the statement as a JS object, or null if the offset is outside all statements.
#[wasm_bindgen(js_name = "statementAt")]
pub fn statement_at_wasm(source: &str, offset: u32) -> Result<JsValue, JsValue> {
    let graph = dot_parser::parse_dot(source);
    match dot_parser::statement_at(&graph, offset) {
        Some(stmt) => serde_wasm_bindgen::to_value(stmt)
            .map_err(|e| JsValue::from_str(&e.to_string())),
        None => Ok(JsValue::NULL),
    }
}

/// Returns the node ID relevant to a given cursor offset within a statement.
/// For node definitions, always returns the node ID.
/// For edges, returns whichever node the cursor is closest to.
#[wasm_bindgen(js_name = "nodeIdAt")]
pub fn node_id_at_wasm(source: &str, statement_offset: u32, cursor_offset: u32) -> Option<String> {
    let graph = dot_parser::parse_dot(source);
    let stmt = dot_parser::statement_at(&graph, statement_offset)?;
    dot_parser::node_id_at(stmt, cursor_offset)
}

/// Find the first node definition for a given node ID, falling back to any edge referencing it.
/// Returns the statement as a JS object, or null if not found.
#[wasm_bindgen(js_name = "definitionForNode")]
pub fn definition_for_node_wasm(source: &str, node_id: &str) -> Result<JsValue, JsValue> {
    let graph = dot_parser::parse_dot(source);
    match dot_parser::definition_for_node(&graph, node_id) {
        Some(stmt) => serde_wasm_bindgen::to_value(stmt)
            .map_err(|e| JsValue::from_str(&e.to_string())),
        None => Ok(JsValue::NULL),
    }
}

/// Find the source range for the definition of a given node ID.
/// Returns [location, length] or null if not found.
#[wasm_bindgen(js_name = "definitionRangeForNode")]
pub fn definition_range_for_node_wasm(source: &str, node_id: &str) -> Option<Vec<u32>> {
    let graph = dot_parser::parse_dot(source);
    let range = dot_parser::definition_range_for_node(&graph, node_id)?;
    Some(vec![range.location, range.length])
}
