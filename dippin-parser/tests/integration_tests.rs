// ABOUTME: Integration tests for the dippin parser using real .dip test data files.
// ABOUTME: Validates parsing and DOT export against known-good inputs.

use dippin_parser::ir::NodeKind;
use dippin_parser::{convert_to_dot, convert_to_dot_with_options, parse, ExportOptions};

fn testdata_path(name: &str) -> String {
    format!(
        "{}/testdata/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    )
}

fn read_testdata(name: &str) -> String {
    std::fs::read_to_string(testdata_path(name))
        .unwrap_or_else(|e| panic!("Failed to read testdata/{}: {}", name, e))
}

#[test]
fn test_parse_valid_minimal() {
    let source = read_testdata("valid_minimal.dip");
    let wf = parse(&source, "valid_minimal.dip").expect("should parse valid_minimal.dip");
    assert_eq!(wf.name, "Minimal");
    assert_eq!(wf.start, "Ask");
    assert_eq!(wf.exit, "Done");
    assert_eq!(wf.nodes.len(), 2);
    assert_eq!(wf.edges.len(), 1);

    let ask = wf.nodes.iter().find(|n| n.id == "Ask").unwrap();
    assert_eq!(ask.kind, NodeKind::Human);

    let done = wf.nodes.iter().find(|n| n.id == "Done").unwrap();
    assert_eq!(done.kind, NodeKind::Agent);
}

#[test]
fn test_parse_valid_minimal_v2() {
    let source = read_testdata("valid_minimal_v2.dip");
    let wf = parse(&source, "valid_minimal_v2.dip").expect("should parse valid_minimal_v2.dip");
    assert_eq!(wf.name, "Minimal");
    assert_eq!(wf.nodes.len(), 2);
}

#[test]
fn test_parse_multi_provider() {
    let source = read_testdata("multi_provider.dip");
    let wf = parse(&source, "multi_provider.dip").expect("should parse multi_provider.dip");
    assert_eq!(wf.name, "MultiProvider");
    assert_eq!(wf.nodes.len(), 4);
    assert_eq!(wf.edges.len(), 3);

    // Check that models are parsed correctly
    let think = wf.nodes.iter().find(|n| n.id == "Think").unwrap();
    let dippin_parser::ir::NodeConfig::Agent(cfg) = &think.config else {
        panic!("Think should be an agent node");
    };
    assert_eq!(cfg.model, "claude-sonnet-4-6");
    assert_eq!(cfg.provider, "anthropic");

    let gen = wf.nodes.iter().find(|n| n.id == "Generate").unwrap();
    let dippin_parser::ir::NodeConfig::Agent(cfg) = &gen.config else {
        panic!("Generate should be an agent node");
    };
    assert_eq!(cfg.model, "gpt-4.1-nano");
    assert_eq!(cfg.provider, "openai");
}

#[test]
fn test_parse_ask_and_execute() {
    let source = read_testdata("ask_and_execute.dip");
    let wf = parse(&source, "ask_and_execute.dip").expect("should parse ask_and_execute.dip");
    assert_eq!(wf.name, "AskAndExecute");
    assert_eq!(wf.start, "Start");
    assert_eq!(wf.exit, "Exit");

    // Check defaults
    assert_eq!(wf.defaults.max_retries, 3);
    assert_eq!(wf.defaults.fidelity, "summary:medium");

    // Check node counts - the file has many nodes
    assert!(wf.nodes.len() > 20, "should have many nodes, got {}", wf.nodes.len());
    assert!(wf.edges.len() > 30, "should have many edges, got {}", wf.edges.len());

    // Check specific node types
    let parallel_nodes: Vec<_> = wf
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Parallel)
        .collect();
    assert_eq!(parallel_nodes.len(), 3, "should have 3 parallel nodes");

    let fan_in_nodes: Vec<_> = wf
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::FanIn)
        .collect();
    assert_eq!(fan_in_nodes.len(), 3, "should have 3 fan_in nodes");

    let human_nodes: Vec<_> = wf
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Human)
        .collect();
    assert_eq!(human_nodes.len(), 2, "should have 2 human nodes");

    let tool_nodes: Vec<_> = wf
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Tool)
        .collect();
    assert_eq!(tool_nodes.len(), 2, "should have 2 tool nodes");

    // Check conditional edges
    let conditional_edges: Vec<_> = wf
        .edges
        .iter()
        .filter(|e| e.condition.is_some())
        .collect();
    assert!(
        conditional_edges.len() >= 5,
        "should have at least 5 conditional edges, got {}",
        conditional_edges.len()
    );

    // Check restart edges
    let restart_edges: Vec<_> = wf.edges.iter().filter(|e| e.restart).collect();
    assert!(
        restart_edges.len() >= 3,
        "should have at least 3 restart edges, got {}",
        restart_edges.len()
    );
}

#[test]
fn test_convert_valid_minimal_to_dot() {
    let source = read_testdata("valid_minimal.dip");
    let dot = convert_to_dot(&source, "valid_minimal.dip").expect("should convert");
    assert!(dot.contains("digraph Minimal {"));
    assert!(dot.contains("Ask"));
    assert!(dot.contains("Done"));
    assert!(dot.contains("Ask -> Done"));
    // Ask is a human node but also the start node, so it gets Mdiamond shape
    assert!(dot.contains("Mdiamond")); // start node shape
    assert!(dot.contains("Msquare")); // exit node shape
}

#[test]
fn test_convert_ask_and_execute_to_dot() {
    let source = read_testdata("ask_and_execute.dip");
    let dot = convert_to_dot(&source, "ask_and_execute.dip").expect("should convert");

    // Check basic structure
    assert!(dot.contains("digraph AskAndExecute {"));
    assert!(dot.contains("rankdir=TB"));

    // Check node shapes
    assert!(dot.contains("Mdiamond")); // start node
    assert!(dot.contains("Msquare")); // exit node
    assert!(dot.contains("hexagon")); // human nodes
    assert!(dot.contains("parallelogram")); // tool nodes
    assert!(dot.contains("component")); // parallel nodes
    assert!(dot.contains("tripleoctagon")); // fan_in nodes

    // Check edges exist
    assert!(dot.contains("Start -> SetupWorkspace"));
    assert!(dot.contains("AskUser -> InterpretRequest"));
    assert!(dot.contains("ApproveCommit -> Exit"));
}

#[test]
fn test_convert_ask_and_execute_with_prompts() {
    let source = read_testdata("ask_and_execute.dip");
    let opts = ExportOptions {
        include_prompts: true,
        ..Default::default()
    };
    let dot =
        convert_to_dot_with_options(&source, "ask_and_execute.dip", &opts).expect("should convert");

    // With include_prompts, agent nodes should have prompt attributes
    assert!(dot.contains("prompt="));
    // Tool nodes should have tool_command attributes
    assert!(dot.contains("tool_command="));
    // Should contain model and provider info
    assert!(dot.contains("model="));
    assert!(dot.contains("provider="));
}

#[test]
fn test_edge_condition_lowering() {
    let source = read_testdata("ask_and_execute.dip");
    let dot = convert_to_dot(&source, "ask_and_execute.dip").expect("should convert");

    // Conditions should have ctx. prefix removed
    assert!(dot.contains("outcome"));
    // Should not contain "ctx.outcome" in the DOT output
    assert!(
        !dot.contains("ctx.outcome"),
        "DOT output should not contain ctx. prefix"
    );
}
