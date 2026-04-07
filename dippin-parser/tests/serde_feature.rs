// ABOUTME: Verifies the optional `serde` feature compiles and round-trips.
// ABOUTME: Only compiled when `--features serde` is enabled.

#![cfg(feature = "serde")]

#[test]
fn test_workflow_roundtrips_through_json() {
    let src = "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
    let wf = dippin_parser::parse(src, "t.dip").unwrap();
    let json = serde_json::to_string(&wf).unwrap();
    let _back: dippin_parser::Workflow = serde_json::from_str(&json).unwrap();
}
