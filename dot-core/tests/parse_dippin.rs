// ABOUTME: Integration tests for the parse_dippin UniFFI export.
// ABOUTME: Exercises the happy path and a syntax-error path.

#[test]
fn parse_dippin_happy_path() {
    let src = "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
    let result = dot_core::parse_dippin(src.to_string()).expect("should parse");
    assert!(result.dot_source.contains("digraph F {"));
    assert!(!result.source_map.is_empty());
    // Each entry's ranges must be non-empty.
    for entry in &result.source_map {
        assert!(entry.dot_end > entry.dot_start);
        assert!(entry.dip_end > entry.dip_start);
    }
}

#[test]
fn parse_dippin_reports_syntax_error() {
    let err = dot_core::parse_dippin("workflow\n".to_string())
        .expect_err("should fail");
    let msg = format!("{:?}", err);
    assert!(msg.to_lowercase().contains("syntax") || msg.contains(":1:"),
            "error should include line:col info, got: {}", msg);
}
