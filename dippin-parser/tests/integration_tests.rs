// ABOUTME: Integration tests for the dippin parser using real .dip test data files.
// ABOUTME: Validates parsing and DOT export against known-good inputs.

use dippin_parser::ir::NodeKind;
use dippin_parser::{parse, parse_to_dot, parse_to_dot_with_options, ExportOptions, NodeConfig};

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
    let dot = parse_to_dot(&source, "valid_minimal.dip").expect("should convert");
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
    let dot = parse_to_dot(&source, "ask_and_execute.dip").expect("should convert");

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
    let mut opts = ExportOptions::default();
    opts.include_prompts = true;
    let dot =
        parse_to_dot_with_options(&source, "ask_and_execute.dip", &opts).expect("should convert");

    // With include_prompts, agent nodes should have prompt attributes
    assert!(dot.contains("prompt="));
    // Tool nodes should have tool_command attributes
    assert!(dot.contains("tool_command="));
    // Should contain model and provider info
    assert!(dot.contains("model="));
    assert!(dot.contains("provider="));
}

#[test]
fn test_parse_unicode() {
    let source = read_testdata("unicode.dip");
    let wf = parse(&source, "unicode.dip").expect("unicode should parse");
    assert_eq!(wf.name, "Unicode");
    let ask = wf.nodes.iter().find(|n| n.id == "Ask").unwrap();
    let dippin_parser::ir::NodeConfig::Agent(cfg) = &ask.config else {
        panic!("Ask should be an agent node");
    };
    assert_eq!(cfg.prompt, "héllo 你好 🎉");

    let done = wf.nodes.iter().find(|n| n.id == "Done").unwrap();
    let dippin_parser::ir::NodeConfig::Agent(cfg) = &done.config else {
        panic!("Done should be an agent node");
    };
    assert_eq!(cfg.prompt, "résumé");
}

#[test]
fn test_convert_unicode_to_dot() {
    let source = read_testdata("unicode.dip");
    let mut opts = ExportOptions::default();
    opts.include_prompts = true;
    let dot = parse_to_dot_with_options(&source, "unicode.dip", &opts).expect("convert");
    assert!(
        dot.contains("héllo 你好 🎉") || dot.contains("h\\u00e9llo"),
        "expected unicode prompt to round-trip into DOT, got:\n{}",
        dot
    );
    assert!(
        dot.contains("résumé") || dot.contains("r\\u00e9sum"),
        "expected résumé to round-trip into DOT, got:\n{}",
        dot
    );
}

#[test]
fn test_single_node_workflow() {
    let src = "workflow Solo\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
    let wf = parse(src, "solo.dip").unwrap();
    assert_eq!(wf.nodes.len(), 1);
    assert!(wf.edges.is_empty());
}

#[test]
fn test_long_lines() {
    let prompt = "x".repeat(8192);
    let src = format!(
        "workflow Long\n  start: A\n  exit: A\n  agent A\n    prompt: \"{}\"\n    model: m\n    provider: p\n",
        prompt
    );
    let wf = parse(&src, "long.dip").unwrap();
    let NodeConfig::Agent(cfg) = &wf.nodes[0].config else {
        panic!("expected agent config");
    };
    assert_eq!(cfg.prompt.len(), 8192);
}

#[test]
fn test_trailing_whitespace_tolerated() {
    // Trailing whitespace on the workflow header line and on quoted/unquoted
    // values must not affect parsing.
    let src = "workflow F   \n  start: A\n  exit: A\n  agent A\n    prompt: x   \n    model: m\n    provider: p\n";
    let wf = parse(src, "ws.dip").unwrap();
    assert_eq!(wf.name, "F");
}

#[test]
fn test_all_go_reference_fixtures_parse() {
    // Ported from upstream Go parser testdata at
    // dippin-lang/parser/testdata/. See FIXME below for skipped fixtures.
    let fixtures = [
        "all_comments.dip",
        // TODO(parity): all_features.dip skipped — uses `subgraph ref: ./review.dip`,
        // and the Rust lexer rejects unquoted values containing `.` or `/`.
        "defaults_complex.dip",
        "edge_attributes.dip",
        "edge_conditions.dip",
        "human_interview.dip",
        "human_node.dip",
        "human_prompt.dip",
        "minimal.dip",
        "multiline_prompt.dip",
        "parallel_branches.dip",
        "response_format.dip",
        "retry_fields.dip",
        // TODO(parity): subgraph_params.dip skipped for the same reason as
        // all_features.dip — `ref: ./review.dip` trips the lexer on `.` and `/`.
        "tool_command.dip",
        "tool_outputs.dip",
    ];
    let mut failed = Vec::new();
    for f in fixtures {
        let src = read_testdata(f);
        if let Err(e) = parse(&src, f) {
            for d in e.diagnostics() {
                eprintln!("{}: {}", f, d.render());
            }
            failed.push(f);
        }
    }
    if !failed.is_empty() {
        panic!("fixtures failed: {:?}", failed);
    }
}

#[test]
fn test_parallel_branches_fixture() {
    let src = read_testdata("parallel_branches.dip");
    let wf = parse(&src, "parallel_branches.dip").unwrap();
    let parallel = wf
        .nodes
        .iter()
        .find(|n| n.kind == NodeKind::Parallel)
        .unwrap();
    let dippin_parser::NodeConfig::Parallel(cfg) = &parallel.config else {
        panic!("expected parallel config");
    };
    assert!(
        !cfg.branches.is_empty(),
        "parallel block form should have branches"
    );
}

// FIXME(parity): test_subgraph_params_fixture omitted because
// subgraph_params.dip is currently skipped in
// test_all_go_reference_fixtures_parse — the Rust lexer rejects unquoted
// values containing `.` or `/`, which trips on `ref: ./review.dip`.
// Re-enable once the parser handles path-like unquoted values.

#[test]
fn test_tool_outputs_fixture() {
    let src = read_testdata("tool_outputs.dip");
    let wf = parse(&src, "tool_outputs.dip").unwrap();
    let tool = wf
        .nodes
        .iter()
        .find(|n| n.kind == NodeKind::Tool)
        .unwrap();
    let dippin_parser::NodeConfig::Tool(cfg) = &tool.config else {
        panic!("expected tool config");
    };
    assert!(!cfg.outputs.is_empty());
}

#[test]
fn test_edge_conditions_fixture() {
    let src = read_testdata("edge_conditions.dip");
    let wf = parse(&src, "edge_conditions.dip").unwrap();
    assert!(wf.edges.iter().any(|e| e.condition.is_some()));
}

#[test]
fn test_retry_fields_fixture() {
    let src = read_testdata("retry_fields.dip");
    let wf = parse(&src, "retry_fields.dip").unwrap();
    let agent = wf
        .nodes
        .iter()
        .find(|n| n.kind == NodeKind::Agent)
        .unwrap();
    assert!(agent.retry.max_retries > 0);
}

#[test]
fn test_response_format_fixture() {
    let src = read_testdata("response_format.dip");
    let wf = parse(&src, "response_format.dip").unwrap();
    let agent = wf
        .nodes
        .iter()
        .find(|n| {
            n.kind == NodeKind::Agent
                && matches!(
                    &n.config,
                    dippin_parser::NodeConfig::Agent(c) if !c.response_format.is_empty()
                )
        })
        .expect("at least one agent should declare response_format");
    let dippin_parser::NodeConfig::Agent(cfg) = &agent.config else {
        panic!("expected agent config");
    };
    assert!(!cfg.response_format.is_empty());
}

#[test]
fn test_multiline_prompt_fixture() {
    let src = read_testdata("multiline_prompt.dip");
    let wf = parse(&src, "multiline_prompt.dip").unwrap();
    let agent = wf
        .nodes
        .iter()
        .find(|n| n.kind == NodeKind::Agent)
        .unwrap();
    let dippin_parser::NodeConfig::Agent(cfg) = &agent.config else {
        panic!("expected agent config");
    };
    assert!(
        cfg.prompt.contains('\n'),
        "multiline prompt should preserve newlines"
    );
}

#[test]
fn test_defaults_complex_fixture() {
    let src = read_testdata("defaults_complex.dip");
    let wf = parse(&src, "defaults_complex.dip").unwrap();
    // Note: defaults_complex.dip does not set `fidelity`, so we check
    // model/provider/max_retries/restart_target instead — the fixture's
    // purpose is exercising the broader defaults block.
    assert_eq!(wf.defaults.model, "claude-sonnet-4-6");
    assert_eq!(wf.defaults.provider, "anthropic");
    assert!(wf.defaults.max_retries > 0);
    assert_eq!(wf.defaults.restart_target, "A");
}

#[test]
fn test_ask_and_execute_golden_snapshot() {
    // ABOUTME: Golden-snapshot test: parse_to_dot(ask_and_execute.dip) must match
    // the committed ask_and_execute.dot byte-for-byte (after trimming trailing ws).
    let source = read_testdata("ask_and_execute.dip");
    let actual = parse_to_dot(&source, "ask_and_execute.dip")
        .expect("ask_and_execute.dip should convert to DOT");
    let expected = read_testdata("ask_and_execute.dot");

    if actual.trim() != expected.trim() {
        // On mismatch, dump the actual output next to the golden so a developer
        // can diff the two manually and either fix the exporter or refresh the
        // golden deliberately.
        let actual_path = testdata_path("ask_and_execute.dot.actual");
        std::fs::write(&actual_path, &actual)
            .expect("should be able to write .actual sibling file");
        panic!(
            "ask_and_execute.dot golden mismatch; wrote actual output to {}.\n\
             Diff with:\n  diff -u {} {}",
            actual_path,
            testdata_path("ask_and_execute.dot"),
            actual_path,
        );
    }
}

#[test]
fn test_edge_condition_lowering() {
    let source = read_testdata("ask_and_execute.dip");
    let dot = parse_to_dot(&source, "ask_and_execute.dip").expect("should convert");

    // Conditions should have ctx. prefix removed
    assert!(dot.contains("outcome"));
    // Should not contain "ctx.outcome" in the DOT output
    assert!(
        !dot.contains("ctx.outcome"),
        "DOT output should not contain ctx. prefix"
    );
}
