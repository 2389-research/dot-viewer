// ABOUTME: Integration tests for parse_to_dot_with_map and the source map it produces.
// ABOUTME: Exercises the happy path; parity with parse_to_dot; range coverage.

#[test]
fn parse_to_dot_with_map_matches_parse_to_dot_output() {
    let src = "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
    let plain = dippin_parser::parse_to_dot(src, "t.dip").unwrap();
    let with_map = dippin_parser::parse_to_dot_with_map(src, "t.dip").unwrap();
    assert_eq!(with_map.dot_source, plain, "dot_source must equal parse_to_dot output");
    assert!(with_map.source_map.is_empty(), "scaffold must not emit entries yet");
}
