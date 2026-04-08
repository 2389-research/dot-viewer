// ABOUTME: Integration tests for parse_to_dot_with_map and the source map it produces.
// ABOUTME: Exercises the happy path; parity with parse_to_dot; range coverage.

#[test]
fn parse_to_dot_with_map_matches_parse_to_dot_output() {
    let src = "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
    let plain = dippin_parser::parse_to_dot(src, "t.dip").unwrap();
    let with_map = dippin_parser::parse_to_dot_with_map(src, "t.dip").unwrap();
    assert_eq!(with_map.dot_source, plain, "dot_source must equal parse_to_dot output");
}

#[test]
fn source_map_contains_one_entry_per_node() {
    let src = "workflow F\n  start: A\n  exit: B\n  agent A\n    prompt: hi\n    model: m\n    provider: p\n  agent B\n    prompt: bye\n    model: m\n    provider: p\n";
    let conv = dippin_parser::parse_to_dot_with_map(src, "t.dip").unwrap();
    assert_eq!(conv.source_map.len(), 2, "expected 2 node entries (no edges)");

    // Entry 0 should be for agent A. Its dippin range must slice to text that
    // includes "agent A".
    let a = &conv.source_map[0];
    let slice = &src[a.dip_range.start..a.dip_range.end];
    assert!(slice.contains("agent A"), "entry 0 dip slice must contain 'agent A', got: {:?}", slice);
    assert!(!slice.contains("agent B"), "entry 0 must NOT reach into agent B, got: {:?}", slice);

    // Entry 1 for agent B.
    let b = &conv.source_map[1];
    let slice = &src[b.dip_range.start..b.dip_range.end];
    assert!(slice.contains("agent B"), "entry 1 dip slice must contain 'agent B', got: {:?}", slice);

    // DOT range must slice to text that mentions the node ID.
    let dot_slice_a = &conv.dot_source[a.dot_range.start..a.dot_range.end];
    assert!(dot_slice_a.contains("\"A\""), "dot slice for A must reference A, got: {:?}", dot_slice_a);
}
