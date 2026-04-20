// ABOUTME: Verifies the optional `serde` feature compiles and round-trips.
// ABOUTME: Only compiled when `--features serde` is enabled.

#![cfg(feature = "serde")]

use dippin_parser::{Error, Workflow};

#[test]
fn test_workflow_roundtrips_through_json() {
    // Exercise IndexMap params, Duration fields, multiple node kinds, and edges
    // so the manual SourceLocation/Duration/Error serde impls all get hit.
    let src = "\
workflow F
  start: A
  exit: B
  agent A
    prompt: hello
    model: m
    provider: p
    cmd_timeout: 1h30m
    params:
      key1: v1
      key2: v2
  tool B
    command: echo hi
    timeout: 2m
  edges
    A -> B
";
    let wf: Workflow = dippin_parser::parse(src, "t.dip").unwrap();
    let json = serde_json::to_string(&wf).unwrap();
    let back: Workflow = serde_json::from_str(&json).unwrap();
    assert_eq!(wf, back);
}

#[test]
fn test_error_roundtrips_through_json() {
    // Bad source: undefined node reference triggers a structured Error::Parse
    // diagnostic, exercising the manual Error / SourceLocation serde impls.
    let bad = "workflow F\n  start: A\n  exit: A\n  edges\n    A -> Missing\n";
    let err = dippin_parser::parse(bad, "t.dip").unwrap_err();
    let json = serde_json::to_string(&err).unwrap();
    let back: Error = serde_json::from_str(&json).unwrap();
    assert_eq!(err.diagnostics(), back.diagnostics());
    // Compare the rendered Display strings to confirm `file` round-tripped too.
    assert_eq!(err.to_string(), back.to_string());
}
