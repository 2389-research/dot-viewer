// ABOUTME: DOT graph format exporter for Dippin workflows.
// ABOUTME: Converts an IR Workflow into a valid DOT digraph string for Graphviz rendering.

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::ir::*;

/// Options controlling the DOT output format.
#[derive(Debug, Clone, Default)]
pub struct ExportOptions {
    /// Include full prompt/command text as node attributes.
    pub include_prompts: bool,
    /// Graph layout direction: "LR" or "TB". Defaults to "TB".
    pub rank_dir: String,
    /// Apply a distinct fill color to nodes with GoalGate: true.
    pub highlight_goal_gates: bool,
    /// Ordered list of node IDs to highlight as an execution path.
    pub execution_path: Vec<String>,
}

/// Render a workflow as a DOT language string.
pub fn export_dot(w: &Workflow, opts: &ExportOptions) -> String {
    let mut b = String::new();

    write_dot_header(&mut b, w, opts);

    for n in &w.nodes {
        write_node_dot(&mut b, n, w, opts);
    }

    b.push('\n');

    for e in &w.edges {
        write_edge_dot(&mut b, e);
    }

    b.push_str("}\n");
    b
}

/// Write the digraph opening and global attributes.
fn write_dot_header(b: &mut String, w: &Workflow, opts: &ExportOptions) {
    let rank_dir = if opts.rank_dir.is_empty() {
        "TB"
    } else {
        &opts.rank_dir
    };
    let graph_name = if w.name.is_empty() {
        "workflow"
    } else {
        &w.name
    };
    let _ = writeln!(b, "digraph {} {{", dot_id(graph_name));
    let _ = writeln!(b, "  rankdir={};", rank_dir);
    b.push_str("  node [fontname=\"Helvetica\"];\n");
    b.push_str("  edge [fontname=\"Helvetica\"];\n");
}

/// Map NodeKind to the corresponding DOT shape attribute.
fn node_shape(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Agent => "box",
        NodeKind::Human => "hexagon",
        NodeKind::Tool => "parallelogram",
        NodeKind::Parallel => "component",
        NodeKind::FanIn => "tripleoctagon",
        NodeKind::Subgraph => "tab",
    }
}

/// Resolve the DOT shape for a node, with start/exit overrides.
fn resolve_node_shape(n: &Node, w: &Workflow) -> &'static str {
    if n.id == w.start {
        return "Mdiamond";
    }
    if n.id == w.exit {
        return "Msquare";
    }
    node_shape(&n.kind)
}

/// Write a single DOT node statement.
fn write_node_dot(b: &mut String, n: &Node, w: &Workflow, opts: &ExportOptions) {
    let mut attrs = BTreeMap::new();
    attrs.insert("shape".to_string(), resolve_node_shape(n, w).to_string());

    let base_label = if n.label.is_empty() {
        n.id.clone()
    } else {
        n.label.clone()
    };
    let label = if let Some(idx) = opts
        .execution_path
        .iter()
        .position(|p| p == &n.id)
    {
        format!("[{}] {}", idx + 1, base_label)
    } else {
        base_label
    };
    attrs.insert("label".to_string(), label);

    if opts.execution_path.iter().any(|p| p == &n.id) {
        attrs.insert("style".to_string(), "bold,filled".to_string());
        attrs.insert("fillcolor".to_string(), "#e0f0ff".to_string());
    }

    if opts.highlight_goal_gates {
        if let NodeConfig::Agent(cfg) = &n.config {
            if cfg.goal_gate {
                attrs.insert("style".to_string(), "filled".to_string());
                attrs.insert("fillcolor".to_string(), "#ffcccc".to_string());
            }
        }
    }

    if opts.include_prompts {
        apply_config_attrs(&mut attrs, &n.config);
    }

    let _ = writeln!(b, "  {} {};", dot_id(&n.id), format_dot_attrs(&attrs));
}

/// Add config-specific attributes to a node's attribute map.
fn apply_config_attrs(attrs: &mut BTreeMap<String, String>, cfg: &NodeConfig) {
    match cfg {
        NodeConfig::Agent(c) => {
            if !c.prompt.is_empty() {
                attrs.insert("prompt".to_string(), escape_newlines(&c.prompt));
            }
            if !c.model.is_empty() {
                attrs.insert("model".to_string(), c.model.clone());
            }
            if !c.provider.is_empty() {
                attrs.insert("provider".to_string(), c.provider.clone());
            }
        }
        NodeConfig::Tool(c) => {
            if !c.command.is_empty() {
                attrs.insert("tool_command".to_string(), escape_newlines(&c.command));
            }
            if !c.timeout.is_zero() {
                attrs.insert("timeout".to_string(), c.timeout.to_string());
            }
        }
        NodeConfig::Human(c) => {
            if !c.mode.is_empty() {
                attrs.insert("mode".to_string(), c.mode.clone());
            }
            if !c.default.is_empty() {
                attrs.insert("default".to_string(), c.default.clone());
            }
        }
        NodeConfig::Subgraph(c) => {
            if !c.ref_path.is_empty() {
                attrs.insert("ref".to_string(), c.ref_path.clone());
            }
        }
        NodeConfig::Parallel(c) => {
            if !c.targets.is_empty() {
                attrs.insert("targets".to_string(), c.targets.join(","));
            }
        }
        NodeConfig::FanIn(c) => {
            if !c.sources.is_empty() {
                attrs.insert("sources".to_string(), c.sources.join(","));
            }
        }
    }
}

/// Write a single DOT edge statement.
fn write_edge_dot(b: &mut String, e: &Edge) {
    let mut attrs = BTreeMap::new();

    if !e.label.is_empty() {
        attrs.insert("label".to_string(), e.label.clone());
    }

    if let Some(cond) = &e.condition {
        let cond_str = cond.raw.clone();
        if !cond_str.is_empty() {
            let cond_str = lower_condition_namespaces(&cond_str);
            if e.label.is_empty() {
                attrs.insert("label".to_string(), cond_str.clone());
            }
            attrs.insert("condition".to_string(), cond_str);
        }
    }

    if e.weight != 0 {
        attrs.insert("weight".to_string(), e.weight.to_string());
    }
    if e.restart {
        attrs.insert("restart".to_string(), "true".to_string());
        attrs.insert("style".to_string(), "dashed".to_string());
    }

    let _ = write!(b, "  {} -> {}", dot_id(&e.from), dot_id(&e.to));
    if !attrs.is_empty() {
        let _ = write!(b, " {}", format_dot_attrs(&attrs));
    }
    b.push_str(";\n");
}

/// Format a BTreeMap of DOT attributes as a bracketed list.
fn format_dot_attrs(attrs: &BTreeMap<String, String>) -> String {
    if attrs.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = attrs
        .iter()
        .map(|(k, v)| format!("{}={}", k, dot_quote(v)))
        .collect();
    format!("[{}]", parts.join(", "))
}

/// Format a string as a valid DOT identifier.
fn dot_id(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }
    if is_simple_dot_id(s) {
        return s.to_string();
    }
    dot_quote(s)
}

/// Check if a string is a valid unquoted DOT identifier.
// Go parity: dippin-lang's parser does not quote DOT keyword identifiers.
fn is_simple_dot_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    if bytes[0].is_ascii_digit() {
        return false;
    }
    s.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// Wrap a string in double quotes, escaping internal quotes and backslashes.
/// Uses char iteration to correctly handle multi-byte UTF-8 sequences.
fn dot_quote(s: &str) -> String {
    let mut b = String::with_capacity(s.len() + 2);
    b.push('"');
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            b.push_str("\\\"");
        } else if ch == '\\' {
            if let Some(&next) = chars.peek() {
                if is_dot_escape_char(next) {
                    b.push('\\');
                    b.push(next);
                    chars.next();
                } else {
                    b.push_str("\\\\");
                }
            } else {
                b.push_str("\\\\");
            }
        } else {
            b.push(ch);
        }
    }
    b.push('"');
    b
}

/// Check if a character is a DOT escape sequence character (n, l, r).
fn is_dot_escape_char(ch: char) -> bool {
    ch == 'n' || ch == 'l' || ch == 'r'
}

/// Replace literal newlines with the DOT \n escape.
fn escape_newlines(s: &str) -> String {
    s.replace('\n', "\\n")
}

/// Strip the ctx. prefix from condition variables for DOT output.
fn lower_condition_namespaces(cond: &str) -> String {
    cond.replace("ctx.", "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_id_simple() {
        assert_eq!(dot_id("hello"), "hello");
        assert_eq!(dot_id("Hello_World"), "Hello_World");
        assert_eq!(dot_id(""), "\"\"");
    }

    #[test]
    fn test_dot_id_needs_quoting() {
        assert_eq!(dot_id("hello world"), "\"hello world\"");
        assert_eq!(dot_id("hello-world"), "\"hello-world\"");
        assert_eq!(dot_id("123abc"), "\"123abc\"");
    }

    #[test]
    fn test_dot_quote() {
        assert_eq!(dot_quote("hello"), "\"hello\"");
        assert_eq!(dot_quote("he\"llo"), "\"he\\\"llo\"");
    }

    #[test]
    fn test_escape_newlines() {
        assert_eq!(escape_newlines("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_lower_condition_namespaces() {
        assert_eq!(
            lower_condition_namespaces("ctx.outcome = success"),
            "outcome = success"
        );
    }

    #[test]
    fn test_export_minimal_workflow() {
        let wf = Workflow {
            name: "Test".to_string(),
            start: "A".to_string(),
            exit: "B".to_string(),
            nodes: vec![
                Node {
                    id: "A".to_string(),
                    kind: NodeKind::Agent,
                    label: "Start".to_string(),
                    classes: Vec::new(),
                    config: NodeConfig::Agent(AgentConfig::default()),
                    retry: RetryConfig::default(),
                    io: NodeIO::default(),
                    source: SourceLocation::default(),
                },
                Node {
                    id: "B".to_string(),
                    kind: NodeKind::Agent,
                    label: "End".to_string(),
                    classes: Vec::new(),
                    config: NodeConfig::Agent(AgentConfig::default()),
                    retry: RetryConfig::default(),
                    io: NodeIO::default(),
                    source: SourceLocation::default(),
                },
            ],
            edges: vec![Edge {
                from: "A".to_string(),
                to: "B".to_string(),
                label: String::new(),
                condition: None,
                weight: 0,
                restart: false,
                source: SourceLocation::default(),
            }],
            ..Default::default()
        };

        let dot = export_dot(&wf, &ExportOptions::default());
        assert!(dot.contains("digraph Test {"));
        assert!(dot.contains("rankdir=TB;"));
        assert!(dot.contains("A [label=\"Start\", shape=\"Mdiamond\"]"));
        assert!(dot.contains("B [label=\"End\", shape=\"Msquare\"]"));
        assert!(dot.contains("A -> B;"));
    }

    #[test]
    fn test_export_edge_with_condition() {
        let wf = Workflow {
            name: "Test".to_string(),
            start: "A".to_string(),
            exit: "B".to_string(),
            nodes: vec![
                Node {
                    id: "A".to_string(),
                    kind: NodeKind::Agent,
                    label: String::new(),
                    classes: Vec::new(),
                    config: NodeConfig::Agent(AgentConfig::default()),
                    retry: RetryConfig::default(),
                    io: NodeIO::default(),
                    source: SourceLocation::default(),
                },
                Node {
                    id: "B".to_string(),
                    kind: NodeKind::Agent,
                    label: String::new(),
                    classes: Vec::new(),
                    config: NodeConfig::Agent(AgentConfig::default()),
                    retry: RetryConfig::default(),
                    io: NodeIO::default(),
                    source: SourceLocation::default(),
                },
            ],
            edges: vec![Edge {
                from: "A".to_string(),
                to: "B".to_string(),
                label: "pass".to_string(),
                condition: Some(Condition {
                    raw: "ctx.outcome = success".to_string(),
                }),
                weight: 0,
                restart: false,
                source: SourceLocation::default(),
            }],
            ..Default::default()
        };

        let dot = export_dot(&wf, &ExportOptions::default());
        assert!(dot.contains("condition="));
        assert!(dot.contains("outcome = success"));
        assert!(dot.contains("label=\"pass\""));
    }

    #[test]
    fn test_export_node_shapes() {
        assert_eq!(node_shape(&NodeKind::Agent), "box");
        assert_eq!(node_shape(&NodeKind::Human), "hexagon");
        assert_eq!(node_shape(&NodeKind::Tool), "parallelogram");
        assert_eq!(node_shape(&NodeKind::Parallel), "component");
        assert_eq!(node_shape(&NodeKind::FanIn), "tripleoctagon");
        assert_eq!(node_shape(&NodeKind::Subgraph), "tab");
    }

    #[test]
    fn test_dot_id_does_not_quote_dot_keywords() {
        // Go reference does not quote `node`, `edge`, etc.
        assert_eq!(dot_id("node"), "node");
        assert_eq!(dot_id("edge"), "edge");
        assert_eq!(dot_id("subgraph"), "subgraph");
    }

    #[test]
    fn test_export_with_execution_path() {
        let src = "workflow F\n  start: A\n  exit: B\n  agent A\n    prompt: x\n    model: m\n    provider: p\n  agent B\n    prompt: y\n    model: m\n    provider: p\n  edges\n    A -> B\n";
        let opts = ExportOptions {
            execution_path: vec!["A".into(), "B".into()],
            ..Default::default()
        };
        let dot = crate::convert_to_dot_with_options(src, "t.dip", &opts).unwrap();
        assert!(dot.contains("[1]"), "expected [1] in: {}", dot);
        assert!(dot.contains("[2]"), "expected [2] in: {}", dot);
        assert!(dot.contains("fillcolor"));
    }

    #[test]
    fn test_export_restart_edge() {
        let wf = Workflow {
            name: "Test".to_string(),
            nodes: Vec::new(),
            edges: vec![Edge {
                from: "A".to_string(),
                to: "B".to_string(),
                label: String::new(),
                condition: None,
                weight: 0,
                restart: true,
                source: SourceLocation::default(),
            }],
            ..Default::default()
        };

        let dot = export_dot(&wf, &ExportOptions::default());
        assert!(dot.contains("restart=\"true\""));
        assert!(dot.contains("style=\"dashed\""));
    }
}
