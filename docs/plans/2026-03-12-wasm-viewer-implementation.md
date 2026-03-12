# WASM Web Viewer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a full web-based DOT editor with live SVG preview, powered by the Rust/Graphviz core compiled to WebAssembly.

**Architecture:** Cross-compile `dot-core` (Rust + vendored Graphviz C libraries) to `wasm32-unknown-emscripten` via Emscripten. Generate TypeScript bindings with `uniffi-bindgen-javascript`. Serve via a SvelteKit static site with CodeMirror 6 editor and inline SVG preview.

**Tech Stack:** Rust, Emscripten, uniffi-bindgen-javascript, SvelteKit, CodeMirror 6, Playwright

**Design doc:** `docs/plans/2026-03-10-wasm-viewer-design.md`

---

## Task 1: Port DotParser from Swift to Rust — Data Types

Port the `DotStatement` enum and `DotGraph` struct from `DotViewer/DotViewer/DotParser.swift` into Rust, exposed via UniFFI.

**Files:**
- Create: `dot-core/src/parser.rs`
- Modify: `dot-core/src/lib.rs`

**Step 1: Write the failing tests**

Add to `dot-core/src/parser.rs`:

```rust
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
}
```

**Step 2: Run tests to verify they fail**

Run: `cd dot-core && cargo test --lib parser`
Expected: FAIL — module `parser` doesn't exist yet

**Step 3: Write minimal data types**

In `dot-core/src/parser.rs`:

```rust
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
```

In `dot-core/src/lib.rs`, add:

```rust
mod parser;
pub use parser::{DotGraph, DotStatement, SourceRange};
```

**Step 4: Implement stub `parse_dot` function**

Add to `dot-core/src/parser.rs` (above the tests module):

```rust
/// Parse DOT source text into a structured graph model with source ranges.
#[uniffi::export]
pub fn parse_dot(_source: String) -> DotGraph {
    DotGraph { statements: vec![] }
}
```

**Step 5: Run tests to see partial results**

Run: `cd dot-core && cargo test --lib parser`
Expected: `test_empty_string_produces_no_statements` PASSES, others FAIL

**Step 6: Commit**

```bash
git add dot-core/src/parser.rs dot-core/src/lib.rs
git commit -m "feat(parser): add Rust DotParser data types and stub parse_dot"
```

---

## Task 2: Port DotParser — Core Parsing Logic

Implement the single-pass state machine parser, ported from `DotParser.swift`. The Swift version uses `NSString`/`unichar`; the Rust version uses byte offsets on UTF-8 strings.

**Reference:** `DotViewer/DotViewer/DotParser.swift:72-187` (the `parse` function and its helpers)

**Files:**
- Modify: `dot-core/src/parser.rs`

**Step 1: Add comprehensive tests**

Port all 29 tests from `DotViewerTests/DotParserTests.swift` into Rust. Add these to the `tests` module in `parser.rs`:

```rust
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
```

**Step 2: Run tests to verify they fail**

Run: `cd dot-core && cargo test --lib parser`
Expected: Most tests FAIL (only empty/whitespace pass from Task 1)

**Step 3: Implement the parser**

Port the Swift `parse` function logic to Rust in `dot-core/src/parser.rs`. The key translation:
- `NSString.character(at:)` → indexing into `&[u8]` (DOT is ASCII-safe)
- `NSRange(location:, length:)` → `SourceRange { location, length }`
- `unichar` constants → `u8` constants
- The state machine structure is identical

```rust
impl DotGraph {
    fn new(statements: Vec<DotStatement>) -> Self {
        DotGraph { statements }
    }
}

#[uniffi::export]
pub fn parse_dot(source: String) -> DotGraph {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut statements = Vec::new();

    let global_keywords: &[&str] = &["digraph", "graph", "subgraph", "node", "edge", "strict"];

    let mut i = 0;

    while i < len {
        i = skip_whitespace_and_semicolons(bytes, i);
        if i >= len { break; }

        // Skip comments
        if bytes[i] == b'/' && i + 1 < len {
            if bytes[i + 1] == b'/' {
                i = skip_to_end_of_line(bytes, i);
                continue;
            } else if bytes[i + 1] == b'*' {
                i = skip_block_comment(bytes, i);
                continue;
            }
        }

        // Opening/closing braces
        if bytes[i] == b'{' || bytes[i] == b'}' {
            i += 1;
            continue;
        }

        let stmt_start = i;

        // Must start with ident char or double quote
        if !is_ident_char(bytes[i]) && bytes[i] != b'"' {
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

        // Check if keyword
        let lower = first_id.to_ascii_lowercase();
        if global_keywords.contains(&lower.as_str()) {
            if lower == "graph" || lower == "node" || lower == "edge" {
                let stmt_end = find_statement_end(bytes, stmt_start);
                statements.push(DotStatement::GraphAttribute {
                    source_range: SourceRange {
                        location: stmt_start as u32,
                        length: (stmt_end - stmt_start) as u32,
                    },
                });
                i = stmt_end;
            } else {
                i = find_statement_end(bytes, after_first);
            }
            continue;
        }

        // Look ahead for edge operator
        let mut scan = skip_whitespace_only(bytes, after_first);

        if scan + 1 < len {
            let is_arrow = (bytes[scan] == b'-' && bytes[scan + 1] == b'>')
                || (bytes[scan] == b'-' && bytes[scan + 1] == b'-');
            if is_arrow {
                let after_arrow = scan + 2;
                let post_arrow = skip_whitespace_only(bytes, after_arrow);
                let (second_id, second_id_range, _) = extract_identifier(bytes, post_arrow);

                let stmt_end = find_statement_end(bytes, stmt_start);
                let range = SourceRange {
                    location: stmt_start as u32,
                    length: (stmt_end - stmt_start) as u32,
                };

                if let (Some(second_id), Some(second_range)) = (second_id, second_id_range) {
                    if let Some(first_range) = first_id_range {
                        statements.push(DotStatement::Edge {
                            from: first_id,
                            to: second_id,
                            source_range: range,
                            from_range: first_range,
                            to_range: second_range,
                        });
                    }
                } else {
                    statements.push(DotStatement::NodeDefinition {
                        id: first_id,
                        source_range: range,
                    });
                }
                i = stmt_end;
                continue;
            }
        }

        // Node definition
        let stmt_end = find_statement_end(bytes, stmt_start);
        statements.push(DotStatement::NodeDefinition {
            id: first_id,
            source_range: SourceRange {
                location: stmt_start as u32,
                length: (stmt_end - stmt_start) as u32,
            },
        });
        i = stmt_end;
    }

    DotGraph::new(statements)
}

// -- Helper functions --

fn is_ident_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn skip_whitespace_and_semicolons(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' | b';' => i += 1,
            _ => break,
        }
    }
    i
}

fn skip_whitespace_only(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            _ => break,
        }
    }
    i
}

fn skip_to_end_of_line(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    if i < bytes.len() { i + 1 } else { i }
}

fn skip_block_comment(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return i + 2;
        }
        i += 1;
    }
    bytes.len()
}

fn extract_identifier(bytes: &[u8], start: usize) -> (Option<String>, Option<SourceRange>, usize) {
    if start >= bytes.len() {
        return (None, None, start);
    }

    if bytes[start] == b'"' {
        let mut i = start + 1;
        while i < bytes.len() {
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                let range = SourceRange {
                    location: start as u32,
                    length: (i + 1 - start) as u32,
                };
                let content = String::from_utf8_lossy(&bytes[start + 1..i]).into_owned();
                return (Some(content), Some(range), i + 1);
            }
            i += 1;
        }
        return (None, None, start);
    }

    if is_ident_char(bytes[start]) {
        let mut i = start;
        while i < bytes.len() && is_ident_char(bytes[i]) {
            i += 1;
        }
        let range = SourceRange {
            location: start as u32,
            length: (i - start) as u32,
        };
        let id = String::from_utf8_lossy(&bytes[start..i]).into_owned();
        return (Some(id), Some(range), i);
    }

    (None, None, start)
}

fn find_statement_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    let mut bracket_depth: i32 = 0;
    let mut in_string = false;

    while i < bytes.len() {
        let ch = bytes[i];

        if ch == b'"' && !in_string {
            in_string = true;
            i += 1;
            continue;
        }
        if in_string {
            if ch == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Comments
        if ch == b'/' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'/' {
                if bracket_depth == 0 { return i; }
                i = skip_to_end_of_line(bytes, i);
                continue;
            }
            if bytes[i + 1] == b'*' {
                i = skip_block_comment(bytes, i);
                continue;
            }
        }

        if ch == b'[' {
            bracket_depth += 1;
            i += 1;
            continue;
        }
        if ch == b']' {
            bracket_depth -= 1;
            if bracket_depth <= 0 {
                return i + 1;
            }
            i += 1;
            continue;
        }

        if bracket_depth == 0 {
            if ch == b'\n' || ch == b';' || ch == b'{' || ch == b'}' {
                return i;
            }
        }

        i += 1;
    }
    bytes.len()
}
```

**Step 4: Run tests to verify they pass**

Run: `cd dot-core && cargo test --lib parser`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add dot-core/src/parser.rs
git commit -m "feat(parser): implement DOT parser in Rust, ported from Swift"
```

---

## Task 3: Port DotParser — Query Methods

Add `statement_at`, `node_id_at`, and `definition_for_node` as UniFFI-exported functions (UniFFI doesn't support methods on Records, so these are free functions taking `DotGraph` as a parameter).

**Files:**
- Modify: `dot-core/src/parser.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_statement_at_offset_finds_correct_statement() {
    let dot = "digraph G {\n    A\n    B -> C\n}";
    let graph = parse_dot(dot.to_string());
    let b_offset = dot.find("B").unwrap() as u32;
    let stmt = statement_at(&graph, b_offset);
    assert!(stmt.is_some());
    match stmt.unwrap() {
        DotStatement::Edge { from, .. } => assert_eq!(from, "B"),
        _ => panic!("Expected edge statement at B's offset"),
    }
}

#[test]
fn test_statement_at_offset_returns_none_outside_statements() {
    let dot = "digraph G {\n\n\n    A\n}";
    let graph = parse_dot(dot.to_string());
    let stmt = statement_at(&graph, 13);
    assert!(stmt.is_none());
}

#[test]
fn test_node_id_at_offset_for_node_definition() {
    let dot = "digraph G {\n    A [label=\"Hello\"]\n}";
    let graph = parse_dot(dot.to_string());
    let a_offset = dot.find("A").unwrap() as u32;
    let stmt = statement_at(&graph, a_offset);
    assert!(stmt.is_some());
    assert_eq!(node_id_at(&stmt.unwrap(), a_offset), Some("A".to_string()));
}

#[test]
fn test_node_id_at_offset_in_attribute_area() {
    let dot = "digraph G {\n    A [label=\"Hello\"]\n}";
    let graph = parse_dot(dot.to_string());
    let label_offset = dot.find("label").unwrap() as u32;
    let stmt = statement_at(&graph, label_offset);
    assert!(stmt.is_some());
    assert_eq!(node_id_at(&stmt.unwrap(), label_offset), Some("A".to_string()));
}

#[test]
fn test_node_id_at_offset_for_edge_selects_closer_node() {
    let dot = "digraph G {\n    A -> B\n}";
    let graph = parse_dot(dot.to_string());
    let a_offset = dot[12..].find("A").unwrap() as u32 + 12;
    let b_offset = dot[12..].find("B").unwrap() as u32 + 12;
    let stmt_a = statement_at(&graph, a_offset).unwrap();
    let stmt_b = statement_at(&graph, b_offset).unwrap();
    assert_eq!(node_id_at(&stmt_a, a_offset), Some("A".to_string()));
    assert_eq!(node_id_at(&stmt_b, b_offset), Some("B".to_string()));
}

#[test]
fn test_definition_for_node_finds_node_definition() {
    let dot = "digraph G {\n    A [label=\"Hello\"]\n    A -> B\n}";
    let graph = parse_dot(dot.to_string());
    let stmt = definition_for_node(&graph, "A".to_string());
    assert!(stmt.is_some());
    match stmt.unwrap() {
        DotStatement::NodeDefinition { id, .. } => assert_eq!(id, "A"),
        _ => panic!("Expected node definition"),
    }
}

#[test]
fn test_definition_for_node_falls_back_to_edge() {
    let dot = "digraph G {\n    A -> B\n}";
    let graph = parse_dot(dot.to_string());
    let stmt = definition_for_node(&graph, "B".to_string());
    assert!(stmt.is_some());
    match stmt.unwrap() {
        DotStatement::Edge { to, .. } => assert_eq!(to, "B"),
        _ => panic!("Expected edge fallback"),
    }
}

#[test]
fn test_definition_for_node_returns_none_for_unknown() {
    let dot = "digraph G {\n    A -> B\n}";
    let graph = parse_dot(dot.to_string());
    assert!(definition_for_node(&graph, "Z".to_string()).is_none());
}

#[test]
fn test_graph_attribute_returns_none_node_id() {
    let dot = "digraph G { graph [rankdir=LR] }";
    let graph = parse_dot(dot.to_string());
    let graph_attr = graph.statements.iter().find(|s| matches!(s, DotStatement::GraphAttribute { .. }));
    assert!(graph_attr.is_some());
    let attr = graph_attr.unwrap();
    let range = match attr {
        DotStatement::GraphAttribute { source_range } => source_range,
        _ => unreachable!(),
    };
    assert!(node_id_at(attr, range.location).is_none());
}
```

**Step 2: Run tests to verify they fail**

Run: `cd dot-core && cargo test --lib parser`
Expected: FAIL — `statement_at`, `node_id_at`, `definition_for_node` don't exist

**Step 3: Implement the query functions**

Add to `dot-core/src/parser.rs`:

```rust
/// Find the statement containing the given character offset.
#[uniffi::export]
pub fn statement_at(graph: &DotGraph, offset: u32) -> Option<DotStatement> {
    graph.statements.iter().find(|s| {
        let range = s.source_range();
        offset >= range.location && offset < range.location + range.length
    }).cloned()
}

/// Returns the node ID relevant to a given cursor offset within a statement.
#[uniffi::export]
pub fn node_id_at(statement: &DotStatement, offset: u32) -> Option<String> {
    match statement {
        DotStatement::NodeDefinition { id, .. } => Some(id.clone()),
        DotStatement::Edge { from, to, from_range, to_range, .. } => {
            let from_center = from_range.location + from_range.length / 2;
            let to_center = to_range.location + to_range.length / 2;
            let dist_from = (offset as i64 - from_center as i64).unsigned_abs() as u32;
            let dist_to = (offset as i64 - to_center as i64).unsigned_abs() as u32;
            if dist_to < dist_from { Some(to.clone()) } else { Some(from.clone()) }
        }
        DotStatement::GraphAttribute { .. } => None,
    }
}

/// Find the first node definition for a given node ID, falling back to any edge referencing it.
#[uniffi::export]
pub fn definition_for_node(graph: &DotGraph, node_id: String) -> Option<DotStatement> {
    // Priority 1: explicit node definition
    for stmt in &graph.statements {
        if let DotStatement::NodeDefinition { id, .. } = stmt {
            if id == &node_id { return Some(stmt.clone()); }
        }
    }
    // Priority 2: first edge referencing this node
    for stmt in &graph.statements {
        if let DotStatement::Edge { from, to, .. } = stmt {
            if from == &node_id || to == &node_id { return Some(stmt.clone()); }
        }
    }
    None
}
```

Add a helper method on `DotStatement`:

```rust
impl DotStatement {
    fn source_range(&self) -> &SourceRange {
        match self {
            DotStatement::NodeDefinition { source_range, .. } => source_range,
            DotStatement::Edge { source_range, .. } => source_range,
            DotStatement::GraphAttribute { source_range } => source_range,
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cd dot-core && cargo test --lib parser`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add dot-core/src/parser.rs
git commit -m "feat(parser): add statement_at, node_id_at, definition_for_node query functions"
```

---

## Task 4: Update macOS App to Use Rust Parser

Replace the Swift `DotParser` with calls to the Rust `parse_dot`, `statement_at`, `node_id_at`, and `definition_for_node` functions. The Swift `DotParser.swift` file remains but becomes a thin wrapper calling the Rust functions.

**Files:**
- Modify: `DotViewer/DotViewer/DotParser.swift`
- Run: `make generate-bindings` (regenerates Swift bindings with new parser API)

**Step 1: Regenerate Swift bindings**

Run: `make generate-bindings`
Expected: `DotViewer/DotViewer/Generated/dot_core.swift` now contains `parseDot`, `statementAt`, `nodeIdAt`, `definitionForNode`

**Step 2: Run existing Swift parser tests to confirm they still pass**

Run: `xcodebuild test -scheme DotViewer -configuration Debug -destination 'platform=macOS' -only-testing:DotViewerTests`
Expected: All 29 tests PASS (still using the Swift parser)

**Step 3: Update DotParser.swift to wrap Rust functions**

Replace the implementation in `DotViewer/DotViewer/DotParser.swift` with a wrapper that translates between the Rust types (`dot_core.DotGraph`, `dot_core.SourceRange`) and the existing Swift types (`DotStatement`, `DotGraph` with `NSRange`). The existing Swift types and their API remain unchanged so the rest of the app doesn't need to change.

Key translations:
- `dot_core.SourceRange { location, length }` → `NSRange(location: Int(location), length: Int(length))`
- `dot_core.DotStatement.nodeDefinition { id, sourceRange }` → `DotStatement.nodeDefinition(id:, sourceRange:)`
- `dot_core.parseDot(source:)` replaces the Swift `parse` state machine

**Step 4: Run Swift parser tests again**

Run: `xcodebuild test -scheme DotViewer -configuration Debug -destination 'platform=macOS' -only-testing:DotViewerTests`
Expected: All 29 tests PASS (now using Rust parser through wrapper)

**Step 5: Commit**

```bash
git add DotViewer/DotViewer/DotParser.swift
git commit -m "refactor(parser): replace Swift DotParser with Rust implementation via UniFFI"
```

---

## Task 5: Emscripten Cross-Compilation of Graphviz

Modify `build.rs` to detect the `wasm32` target and use Emscripten's CMake toolchain instead of the native one. This is the hardest build task — Graphviz's CMake needs Emscripten-specific configuration.

**Prerequisites:**
- Install Emscripten: `brew install emscripten` (or via `emsdk`)
- Install Rust WASM target: `rustup target add wasm32-unknown-emscripten`

**Files:**
- Modify: `dot-core/build.rs`
- Modify: `dot-core/Cargo.toml`

**Step 1: Add `wasm32` detection to `build.rs`**

At the top of `main()` in `build.rs`, add target detection:

```rust
let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
let is_wasm = target_arch == "wasm32";
```

**Step 2: Conditional CMake toolchain**

When `is_wasm` is true, configure CMake to use Emscripten:

```rust
let mut cmake_cfg = cmake::Config::new("graphviz-vendor");

if is_wasm {
    // Emscripten provides its own cmake toolchain file
    let emsdk = env::var("EMSDK").expect("EMSDK environment variable must be set for WASM builds");
    let toolchain = format!("{}/upstream/emscripten/cmake/Modules/Platform/Emscripten.cmake", emsdk);
    cmake_cfg.define("CMAKE_TOOLCHAIN_FILE", &toolchain);
    // Emscripten's bison/flex from the system are fine for code generation
    // (they run on the host, not the target)
} else {
    // Native build — use Homebrew bison/flex as before
    cmake_cfg.define("BISON_EXECUTABLE", &bison_exe);
    cmake_cfg.define("FLEX_EXECUTABLE", &flex_exe);
}
```

The rest of the CMake defines (`BUILD_SHARED_LIBS=OFF`, `ENABLE_LTDL=OFF`, etc.) stay the same.

**Step 3: Conditional system library linking**

```rust
if is_wasm {
    // Emscripten provides its own libc, zlib, etc.
    // expat may need to be compiled via emscripten ports
    println!("cargo:rustc-link-lib=static=expat");
} else {
    println!("cargo:rustc-link-lib=expat");
    println!("cargo:rustc-link-lib=z");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");
    #[cfg(not(target_os = "macos"))]
    println!("cargo:rustc-link-lib=stdc++");
}
```

**Step 4: Test the WASM build**

Run: `cd dot-core && cargo build --target wasm32-unknown-emscripten --release`
Expected: Compiles successfully (this may require iterating on CMake/Emscripten issues)

**Note:** This step is the most likely to need debugging. Common issues:
- Emscripten may not find `bison`/`flex` — ensure they're in PATH
- Graphviz may have C features unsupported by Emscripten (e.g., `fork`, `popen`) — these should already be disabled via our CMake flags
- Expat may need Emscripten's ports system: add `-s USE_EXPAT=1` to emcc flags
- If `zlib` is needed, Emscripten provides it via `-s USE_ZLIB=1`

**Step 5: Commit**

```bash
git add dot-core/build.rs dot-core/Cargo.toml
git commit -m "feat(wasm): add Emscripten cross-compilation support to build.rs"
```

---

## Task 6: UniFFI JavaScript Bindings

Set up `uniffi-bindgen-javascript` to generate TypeScript bindings from the Rust library's UniFFI annotations.

**Files:**
- Modify: `dot-core/Cargo.toml`
- Create: `scripts/generate-wasm-bindings.sh`
- Modify: `Makefile`

**Step 1: Add uniffi-bindgen-javascript dependency**

In `dot-core/Cargo.toml`, add under `[build-dependencies]`:

```toml
[build-dependencies]
cmake = "0.1"
bindgen = "0.71"

# For generating JS/TS bindings (run manually via scripts/generate-wasm-bindings.sh)
# uniffi-bindgen-javascript is installed as a cargo tool, not a build dep
```

**Step 2: Create the binding generation script**

Create `scripts/generate-wasm-bindings.sh`:

```bash
#!/usr/bin/env bash
# ABOUTME: Generates TypeScript bindings and WASM module from dot-core for the web app.
# ABOUTME: Requires Emscripten SDK and uniffi-bindgen-javascript to be installed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "==> Building dot-core for wasm32-unknown-emscripten..."
cd "$PROJECT_DIR/dot-core"
cargo build --target wasm32-unknown-emscripten --release

echo "==> Generating TypeScript bindings..."
cargo install uniffi-bindgen-javascript || true
uniffi-bindgen-javascript \
    --library target/wasm32-unknown-emscripten/release/libdot_core.a \
    --out-dir "$PROJECT_DIR/web/src/lib/dot-core"

echo "==> Done. Bindings written to web/src/lib/dot-core/"
```

**Step 3: Add Makefile targets**

Add to `Makefile`:

```makefile
build-wasm:
	bash scripts/generate-wasm-bindings.sh

clean-wasm:
	rm -rf web/src/lib/dot-core/generated
```

**Step 4: Test binding generation**

Run: `make build-wasm`
Expected: TypeScript bindings appear in `web/src/lib/dot-core/`

**Note:** The exact `uniffi-bindgen-javascript` CLI flags may differ from what's shown above — check its README for the current API. The tool is being actively developed and renamed from `uniffi-bindgen-react-native`.

**Step 5: Commit**

```bash
git add scripts/generate-wasm-bindings.sh Makefile dot-core/Cargo.toml
git commit -m "feat(wasm): add uniffi-bindgen-javascript binding generation"
```

---

## Task 7: SvelteKit Project Scaffold

Create the SvelteKit project with adapter-static.

**Files:**
- Create: `web/` directory (SvelteKit scaffold)

**Step 1: Scaffold the project**

```bash
cd /path/to/dot-viewer
npx sv create web --template minimal --types ts
cd web
npm install
npm install -D @sveltejs/adapter-static
```

**Step 2: Configure adapter-static**

Edit `web/svelte.config.js`:

```javascript
// ABOUTME: SvelteKit configuration for the Dot Viewer web app.
// ABOUTME: Uses adapter-static for host-agnostic static file deployment.

import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/kit/vite';

/** @type {import('@sveltejs/kit').Config} */
const config = {
    preprocess: vitePreprocess(),
    kit: {
        adapter: adapter({
            pages: 'build',
            assets: 'build',
            fallback: 'index.html',
            precompress: true,
        }),
    },
};

export default config;
```

**Step 3: Add prerender config**

Create/edit `web/src/routes/+layout.ts`:

```typescript
// ABOUTME: Root layout config enabling static prerendering for all routes.
// ABOUTME: Required for adapter-static to generate static HTML files.

export const prerender = true;
export const ssr = false;
```

**Step 4: Verify it builds**

Run: `cd web && npm run build`
Expected: Static files in `web/build/`

**Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): scaffold SvelteKit project with adapter-static"
```

---

## Task 8: WASM Loader Module

Create a TypeScript module that loads the WASM binary lazily and provides typed wrappers around the generated UniFFI bindings.

**Files:**
- Create: `web/src/lib/wasm.ts`

**Step 1: Create the WASM loader**

```typescript
// ABOUTME: Lazy-loading wrapper for the dot-core WASM module.
// ABOUTME: Provides typed async API for DOT rendering and parsing.

import type { DotGraph, DotStatement, LayoutEngine } from './dot-core';

let wasmModule: typeof import('./dot-core') | null = null;
let loadingPromise: Promise<typeof import('./dot-core')> | null = null;

export async function loadWasm(): Promise<typeof import('./dot-core')> {
    if (wasmModule) return wasmModule;
    if (loadingPromise) return loadingPromise;

    loadingPromise = import('./dot-core').then((mod) => {
        wasmModule = mod;
        return mod;
    });

    return loadingPromise;
}

export function isLoaded(): boolean {
    return wasmModule !== null;
}

export async function renderDot(source: string, engine: LayoutEngine): Promise<string> {
    const mod = await loadWasm();
    return mod.renderDot(source, engine);
}

export async function parseDot(source: string): Promise<DotGraph> {
    const mod = await loadWasm();
    return mod.parseDot(source);
}

export { type DotGraph, type DotStatement, type LayoutEngine };
```

**Note:** The exact import path and API shape depends on what `uniffi-bindgen-javascript` generates. Adjust after Task 6 produces the actual bindings.

**Step 2: Commit**

```bash
git add web/src/lib/wasm.ts
git commit -m "feat(web): add lazy WASM loader module"
```

---

## Task 9: Editor Component with CodeMirror 6

Create the code editor component with DOT syntax highlighting and debounced change events.

**Files:**
- Create: `web/src/lib/components/Editor.svelte`

**Step 1: Install CodeMirror**

```bash
cd web
npm install codemirror @codemirror/lang-javascript @codemirror/view @codemirror/state @codemirror/commands @codemirror/language
```

**Note:** There may be a community `codemirror-lang-dot` or `@codemirror/lang-dot` package. Search npm first — `npm search codemirror dot graphviz`. If none exists, use a simple Lezer grammar or plain text mode for now.

**Step 2: Create the Editor component**

```svelte
<!-- ABOUTME: CodeMirror 6 editor component with DOT syntax support. -->
<!-- ABOUTME: Emits debounced change events for live preview rendering. -->

<script lang="ts">
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import { EditorState } from '@codemirror/state';
    import { EditorView, keymap, lineNumbers, highlightActiveLine } from '@codemirror/view';
    import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
    import { bracketMatching } from '@codemirror/language';

    export let value = '';
    export let debounceMs = 300;

    const dispatch = createEventDispatcher<{ change: string }>();

    let container: HTMLDivElement;
    let view: EditorView;
    let debounceTimer: ReturnType<typeof setTimeout>;

    onMount(() => {
        const state = EditorState.create({
            doc: value,
            extensions: [
                lineNumbers(),
                highlightActiveLine(),
                bracketMatching(),
                history(),
                keymap.of([...defaultKeymap, ...historyKeymap]),
                EditorView.updateListener.of((update) => {
                    if (update.docChanged) {
                        const newValue = update.state.doc.toString();
                        clearTimeout(debounceTimer);
                        debounceTimer = setTimeout(() => {
                            dispatch('change', newValue);
                        }, debounceMs);
                    }
                }),
            ],
        });

        view = new EditorView({ state, parent: container });
    });

    onDestroy(() => {
        clearTimeout(debounceTimer);
        view?.destroy();
    });

    export function setCursorPosition(offset: number) {
        if (!view) return;
        view.dispatch({ selection: { anchor: offset } });
        view.focus();
    }

    export function scrollToOffset(offset: number) {
        if (!view) return;
        view.dispatch({
            selection: { anchor: offset },
            effects: EditorView.scrollIntoView(offset, { y: 'center' }),
        });
        view.focus();
    }
</script>

<div bind:this={container} class="editor-container" />

<style>
    .editor-container {
        height: 100%;
        overflow: auto;
    }
    .editor-container :global(.cm-editor) {
        height: 100%;
    }
</style>
```

**Step 3: Commit**

```bash
git add web/src/lib/components/Editor.svelte web/package.json web/package-lock.json
git commit -m "feat(web): add CodeMirror 6 editor component"
```

---

## Task 10: Preview Component

Create the SVG preview component with pan/zoom and node click handling.

**Files:**
- Create: `web/src/lib/components/Preview.svelte`

**Step 1: Install panzoom library**

```bash
cd web
npm install panzoom
```

**Step 2: Create the Preview component**

```svelte
<!-- ABOUTME: SVG preview component with pan/zoom and clickable nodes. -->
<!-- ABOUTME: Renders DOT output as inline SVG with bidirectional navigation support. -->

<script lang="ts">
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import panzoom from 'panzoom';

    export let svg = '';
    export let error = '';
    export let loading = false;

    const dispatch = createEventDispatcher<{ nodeClick: string }>();

    let container: HTMLDivElement;
    let svgContainer: HTMLDivElement;
    let panzoomInstance: ReturnType<typeof panzoom> | null = null;

    onMount(() => {
        if (svgContainer) {
            panzoomInstance = panzoom(svgContainer, {
                maxZoom: 10,
                minZoom: 0.1,
                smoothScroll: false,
            });
        }
    });

    onDestroy(() => {
        panzoomInstance?.dispose();
    });

    function handleClick(event: MouseEvent) {
        // Walk up from click target to find an SVG node group
        let el = event.target as Element | null;
        while (el && el !== svgContainer) {
            if (el.classList?.contains('node')) {
                const title = el.querySelector('title');
                if (title?.textContent) {
                    dispatch('nodeClick', title.textContent);
                    return;
                }
            }
            el = el.parentElement;
        }
    }
</script>

<div class="preview-container" bind:this={container}>
    {#if loading}
        <div class="loading">Loading Graphviz...</div>
    {/if}

    {#if error}
        <div class="error-bar">{error}</div>
    {/if}

    <div
        class="svg-container"
        bind:this={svgContainer}
        on:click={handleClick}
    >
        {@html svg}
    </div>
</div>

<style>
    .preview-container {
        height: 100%;
        position: relative;
        overflow: hidden;
        background: white;
    }
    .svg-container {
        width: 100%;
        height: 100%;
    }
    .loading {
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        color: #666;
    }
    .error-bar {
        position: absolute;
        bottom: 0;
        left: 0;
        right: 0;
        padding: 8px 12px;
        background: #fee;
        color: #c00;
        font-size: 13px;
        z-index: 10;
    }
</style>
```

**Step 3: Commit**

```bash
git add web/src/lib/components/Preview.svelte web/package.json web/package-lock.json
git commit -m "feat(web): add SVG preview component with pan/zoom"
```

---

## Task 11: Toolbar Component

Create the toolbar with layout engine picker.

**Files:**
- Create: `web/src/lib/components/Toolbar.svelte`

**Step 1: Create the Toolbar component**

```svelte
<!-- ABOUTME: Toolbar component with layout engine selector dropdown. -->
<!-- ABOUTME: Mirrors the macOS app's toolbar controls for the web editor. -->

<script lang="ts">
    import { createEventDispatcher } from 'svelte';

    export let engine = 'dot';

    const engines = ['dot', 'neato', 'fdp', 'circo', 'twopi', 'sfdp'];
    const dispatch = createEventDispatcher<{ engineChange: string }>();

    function handleChange(event: Event) {
        const target = event.target as HTMLSelectElement;
        dispatch('engineChange', target.value);
    }
</script>

<div class="toolbar">
    <label>
        Engine:
        <select value={engine} on:change={handleChange}>
            {#each engines as eng}
                <option value={eng}>{eng}</option>
            {/each}
        </select>
    </label>
</div>

<style>
    .toolbar {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 8px 16px;
        border-bottom: 1px solid #ddd;
        background: #fafafa;
    }
    label {
        font-size: 14px;
        display: flex;
        align-items: center;
        gap: 6px;
    }
    select {
        padding: 4px 8px;
        border-radius: 4px;
        border: 1px solid #ccc;
    }
</style>
```

**Step 2: Commit**

```bash
git add web/src/lib/components/Toolbar.svelte
git commit -m "feat(web): add toolbar component with engine picker"
```

---

## Task 12: Main Page — Wire Everything Together

Connect editor, preview, toolbar, and WASM module on the main page.

**Files:**
- Modify: `web/src/routes/+page.svelte`

**Step 1: Create the main page**

```svelte
<!-- ABOUTME: Main page wiring the editor, preview, toolbar, and WASM module together. -->
<!-- ABOUTME: Implements debounced live preview with bidirectional editor-preview navigation. -->

<script lang="ts">
    import { onMount } from 'svelte';
    import Editor from '$lib/components/Editor.svelte';
    import Preview from '$lib/components/Preview.svelte';
    import Toolbar from '$lib/components/Toolbar.svelte';
    import { loadWasm, renderDot, parseDot, isLoaded } from '$lib/wasm';
    import type { LayoutEngine } from '$lib/wasm';

    let svg = '';
    let error = '';
    let loading = true;
    let engine = 'dot';
    let currentSource = 'digraph G {\n    A -> B\n    B -> C\n    C -> A\n}';
    let editor: Editor;

    const engineMap: Record<string, LayoutEngine> = {
        dot: 'Dot' as LayoutEngine,
        neato: 'Neato' as LayoutEngine,
        fdp: 'Fdp' as LayoutEngine,
        circo: 'Circo' as LayoutEngine,
        twopi: 'Twopi' as LayoutEngine,
        sfdp: 'Sfdp' as LayoutEngine,
    };

    onMount(async () => {
        await loadWasm();
        loading = false;
        await render(currentSource);
    });

    async function render(source: string) {
        if (!isLoaded()) return;
        try {
            svg = await renderDot(source, engineMap[engine]);
            error = '';
        } catch (e) {
            error = e instanceof Error ? e.message : String(e);
        }
    }

    function handleEditorChange(event: CustomEvent<string>) {
        currentSource = event.detail;
        render(currentSource);
    }

    function handleEngineChange(event: CustomEvent<string>) {
        engine = event.detail;
        render(currentSource);
    }

    async function handleNodeClick(event: CustomEvent<string>) {
        if (!isLoaded()) return;
        const graph = await parseDot(currentSource);
        // Use definition_for_node to find where this node is defined
        // Then scroll editor to that offset
        // (Adjust based on actual generated binding names)
    }
</script>

<div class="app">
    <Toolbar {engine} on:engineChange={handleEngineChange} />
    <div class="split-pane">
        <div class="editor-pane">
            <Editor
                bind:this={editor}
                value={currentSource}
                on:change={handleEditorChange}
            />
        </div>
        <div class="preview-pane">
            <Preview {svg} {error} {loading} on:nodeClick={handleNodeClick} />
        </div>
    </div>
</div>

<style>
    .app {
        display: flex;
        flex-direction: column;
        height: 100vh;
    }
    .split-pane {
        display: flex;
        flex: 1;
        overflow: hidden;
    }
    .editor-pane {
        flex: 1;
        border-right: 1px solid #ddd;
        overflow: hidden;
    }
    .preview-pane {
        flex: 1;
        overflow: hidden;
    }
</style>
```

**Step 2: Verify dev server works**

Run: `cd web && npm run dev`
Expected: App loads at `http://localhost:5173`, editor visible (preview won't work until WASM is built)

**Step 3: Commit**

```bash
git add web/src/routes/+page.svelte
git commit -m "feat(web): wire editor, preview, toolbar, and WASM on main page"
```

---

## Task 13: Bidirectional Navigation

Implement click-node-to-jump-to-editor and cursor-to-highlight-node using the Rust parser via WASM.

**Files:**
- Modify: `web/src/routes/+page.svelte`
- Modify: `web/src/lib/components/Preview.svelte`

**Step 1: Implement node click → editor jump**

In `+page.svelte`, flesh out `handleNodeClick`:

```typescript
async function handleNodeClick(event: CustomEvent<string>) {
    if (!isLoaded()) return;
    const nodeId = event.detail;
    const graph = await parseDot(currentSource);
    const stmt = definitionForNode(graph, nodeId);
    if (stmt) {
        // Get the source_range offset from the statement
        const offset = getStatementOffset(stmt);
        editor.scrollToOffset(offset);
    }
}
```

**Step 2: Implement SVG node highlighting**

Add CSS classes to highlight the active node in the SVG preview when the cursor moves in the editor:

```typescript
function highlightNodeInSvg(nodeId: string | null) {
    const svgEl = document.querySelector('.svg-container svg');
    if (!svgEl) return;
    // Remove previous highlights
    svgEl.querySelectorAll('.node.highlighted').forEach((el) => {
        el.classList.remove('highlighted');
    });
    if (!nodeId) return;
    // Find and highlight the matching node
    svgEl.querySelectorAll('.node title').forEach((title) => {
        if (title.textContent === nodeId) {
            title.parentElement?.classList.add('highlighted');
        }
    });
}
```

**Step 3: Commit**

```bash
git add web/src/routes/+page.svelte web/src/lib/components/Preview.svelte
git commit -m "feat(web): implement bidirectional editor-preview navigation"
```

---

## Task 14: Playwright E2E Tests

Set up Playwright and write end-to-end tests for the web app.

**Files:**
- Create: `web/playwright.config.ts`
- Create: `web/tests/app.test.ts`

**Step 1: Install Playwright**

```bash
cd web
npm install -D @playwright/test
npx playwright install chromium
```

**Step 2: Create Playwright config**

```typescript
// ABOUTME: Playwright configuration for E2E testing the Dot Viewer web app.
// ABOUTME: Tests against the built static site using Chromium.

import { defineConfig } from '@playwright/test';

export default defineConfig({
    testDir: 'tests',
    webServer: {
        command: 'npm run preview',
        port: 4173,
        reuseExistingServer: true,
    },
    use: {
        baseURL: 'http://localhost:4173',
    },
});
```

**Step 3: Write E2E tests**

```typescript
// ABOUTME: End-to-end tests for the Dot Viewer web app.
// ABOUTME: Verifies editor rendering, SVG preview, engine switching, and error handling.

import { test, expect } from '@playwright/test';

test('page loads with editor and preview', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.editor-container')).toBeVisible();
    await expect(page.locator('.preview-container')).toBeVisible();
});

test('editor accepts text input', async ({ page }) => {
    await page.goto('/');
    const editor = page.locator('.cm-editor .cm-content');
    await editor.click();
    await editor.fill('digraph Test { X -> Y }');
    // Wait for debounce + render
    await page.waitForTimeout(500);
    const svg = page.locator('.svg-container svg');
    await expect(svg).toBeVisible();
});

test('engine picker changes layout', async ({ page }) => {
    await page.goto('/');
    // Wait for WASM to load and initial render
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    const select = page.locator('.toolbar select');
    await select.selectOption('neato');
    // SVG should re-render (content will differ)
    await page.waitForTimeout(500);
    await expect(page.locator('.svg-container svg')).toBeVisible();
});

test('invalid DOT shows error', async ({ page }) => {
    await page.goto('/');
    const editor = page.locator('.cm-editor .cm-content');
    await editor.click();
    await editor.fill('not valid dot {{{');
    await page.waitForTimeout(500);
    await expect(page.locator('.error-bar')).toBeVisible();
});

test('clicking a node highlights in editor', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    // Click on a node in the SVG
    const node = page.locator('.svg-container .node').first();
    if (await node.count() > 0) {
        await node.click();
        // Editor should have focus after click
        await expect(page.locator('.cm-editor.cm-focused')).toBeVisible();
    }
});
```

**Step 4: Run tests**

Run: `cd web && npm run build && npx playwright test`
Expected: Tests pass (or fail specifically on WASM loading if bindings aren't generated yet)

**Step 5: Commit**

```bash
git add web/playwright.config.ts web/tests/ web/package.json web/package-lock.json
git commit -m "test(web): add Playwright E2E tests for web editor"
```

---

## Task 15: Makefile Integration and CI

Add top-level Makefile targets for the web build and a GitHub Actions workflow for WASM CI.

**Files:**
- Modify: `Makefile`
- Create: `.github/workflows/web.yml`

**Step 1: Add Makefile targets**

Add to `Makefile`:

```makefile
.PHONY: build-wasm web-dev web-build web-test

build-wasm:
	bash scripts/generate-wasm-bindings.sh

web-install:
	cd web && npm install

web-dev: build-wasm web-install
	cd web && npm run dev

web-build: build-wasm web-install
	cd web && npm run build

web-test: web-build
	cd web && npx playwright test
```

**Step 2: Create GitHub Actions workflow**

Create `.github/workflows/web.yml`:

```yaml
# ABOUTME: CI workflow for the Dot Viewer web app.
# ABOUTME: Builds WASM module and runs Playwright E2E tests.

name: Web Build & Test

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Clone Graphviz source
        run: |
          cd dot-core
          git clone --depth 1 --branch 12.2.1 https://gitlab.com/graphviz/graphviz.git graphviz-vendor

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-emscripten

      - name: Install Emscripten
        uses: mymindstorm/setup-emsdk@v14

      - name: Install bison and flex
        run: sudo apt-get install -y bison flex

      - name: Install Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 22

      - name: Build WASM module
        run: make build-wasm

      - name: Install web dependencies
        run: cd web && npm install

      - name: Build web app
        run: cd web && npm run build

      - name: Install Playwright browsers
        run: cd web && npx playwright install --with-deps chromium

      - name: Run Playwright tests
        run: cd web && npx playwright test
```

**Step 3: Commit**

```bash
git add Makefile .github/workflows/web.yml
git commit -m "ci(web): add Makefile targets and GitHub Actions workflow for WASM web app"
```

---

## Execution Order and Dependencies

```
Task 1: Parser data types ──→ Task 2: Parser logic ──→ Task 3: Query methods
                                                              │
Task 4: Update macOS app ←────────────────────────────────────┘
                                                              │
Task 5: Emscripten build ──→ Task 6: UniFFI JS bindings ─────┘
                                                              │
Task 7: SvelteKit scaffold                                    │
Task 8: WASM loader ←────────────────────────────────────────┘
Task 9: Editor component
Task 10: Preview component
Task 11: Toolbar component
Task 12: Main page (depends on 7-11)
Task 13: Bidirectional nav (depends on 12)
Task 14: Playwright tests (depends on 12)
Task 15: CI (depends on all)
```

Tasks 7, 9, 10, 11 can run in parallel. Tasks 1-3 can run in parallel with Tasks 5-6. Task 4 depends on Tasks 1-3. Task 12 depends on Tasks 7-11 and Task 8.
