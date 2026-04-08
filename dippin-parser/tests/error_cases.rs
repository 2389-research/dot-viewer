// ABOUTME: Failure-mode tests for the dippin parser. Every diagnostic branch must be exercised here.
// ABOUTME: Each test asserts the expected DiagnosticKind appears in the error.

use dippin_parser::{parse, DiagnosticKind};

fn assert_kind(src: &str, predicate: impl Fn(&DiagnosticKind) -> bool) {
    let err = parse(src, "test.dip").expect_err("expected parse to fail");
    let diags = err.diagnostics();
    assert!(
        diags.iter().any(|d| predicate(&d.kind)),
        "expected matching diagnostic, got: {:?}",
        diags
    );
}

// Note: in Dippin's actual surface syntax, node declarations (`agent`, `human`, etc.)
// are nested INSIDE the `workflow` block (indented at col 2), with their fields at
// col 4. The original plan snippets used top-level `agent A` which the parser treats
// as outside the workflow scope (yielding UndefinedNodeReference instead of the
// targeted diagnostic). Tests below use the canonical nested form.

#[test]
fn test_unterminated_string() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: \"unterminated\n    model: m\n",
        |k| matches!(k, DiagnosticKind::UnterminatedString),
    );
}

#[test]
fn test_unknown_node_kind() {
    // There is no dedicated `UnknownNodeKind` variant; an unrecognized child
    // keyword inside the workflow body collapses to `DiagnosticKind::Other`.
    // Tighten this test once the parser grows a more specific diagnostic.
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  wizard A\n    prompt: x\n",
        |k| matches!(k, DiagnosticKind::Other),
    );
}

#[test]
fn test_unknown_defaults_field() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  defaults\n    bogus: 1\n  agent A\n    prompt: x\n    model: m\n    provider: p\n",
        |k| matches!(k, DiagnosticKind::UnknownField { scope, .. } if scope == "defaults"),
    );
}

#[test]
fn test_unknown_agent_field() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    bogus: 1\n    model: m\n    provider: p\n",
        |k| matches!(k, DiagnosticKind::UnknownField { scope, .. } if scope == "agent"),
    );
}

#[test]
fn test_invalid_integer() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  defaults\n    max_retries: not_a_number\n  agent A\n    prompt: x\n    model: m\n    provider: p\n",
        |k| matches!(k, DiagnosticKind::InvalidInteger { .. }),
    );
}

#[test]
fn test_invalid_float() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    compaction_threshold: abc\n    model: m\n    provider: p\n",
        |k| matches!(k, DiagnosticKind::InvalidFloat { .. }),
    );
}

#[test]
fn test_invalid_duration() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    cmd_timeout: 30\n    model: m\n    provider: p\n",
        |k| matches!(k, DiagnosticKind::InvalidDuration { .. }),
    );
}

#[test]
fn test_invalid_bool() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    goal_gate: yes\n    model: m\n    provider: p\n",
        |k| matches!(k, DiagnosticKind::InvalidBool { .. }),
    );
}

#[test]
fn test_missing_workflow_identifier() {
    // Pin `after = "workflow"` so a future regression that swallows this
    // diagnostic and leaves only the secondary `EmptyWorkflow` will fail
    // loudly instead of silently overlapping with `test_empty_file`.
    assert_kind(
        "workflow\n  start: A\n",
        |k| matches!(k, DiagnosticKind::MissingIdentifier { after } if after == "workflow"),
    );
}

#[test]
fn test_missing_agent_identifier() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  agent\n    prompt: x\n    model: m\n    provider: p\n",
        |k| matches!(k, DiagnosticKind::MissingIdentifier { after } if after == "agent"),
    );
}

#[test]
fn test_undefined_node_reference() {
    assert_kind(
        "workflow F\n  start: Missing\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n",
        |k| matches!(k, DiagnosticKind::UndefinedNodeReference(_)),
    );
}

#[test]
fn test_duplicate_workflow() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\nworkflow G\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n",
        |k| matches!(k, DiagnosticKind::DuplicateWorkflow),
    );
}

#[test]
fn test_empty_file() {
    assert_kind("", |k| matches!(k, DiagnosticKind::EmptyWorkflow));
}

#[test]
fn test_whitespace_only_file() {
    assert_kind("   \n\n  \n", |k| matches!(k, DiagnosticKind::EmptyWorkflow));
}

#[test]
fn test_comments_only_file() {
    assert_kind("# just a comment\n# another\n", |k| matches!(k, DiagnosticKind::EmptyWorkflow));
}

#[test]
fn test_mixed_indentation() {
    assert_kind(
        "workflow F\n\t  start: A\n",
        |k| matches!(k, DiagnosticKind::InvalidIndentation(_)),
    );
}

#[test]
fn test_invalid_dedent() {
    assert_kind(
        "workflow F\n    start: A\n  exit: A\n",
        |k| matches!(k, DiagnosticKind::InvalidIndentation(_)),
    );
}

#[test]
fn test_oversize_input() {
    let big = "a".repeat(dippin_parser::MAX_INPUT_SIZE + 1);
    assert!(parse(&big, "big.dip").is_err());
}

#[test]
fn undefined_start_diagnostic_points_at_start_line() {
    // `start: Missing` is on line 2 of the source.
    let src = "workflow F\n  start: Missing\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
    let err = parse(src, "test.dip").expect_err("expected validation failure");
    let diag = err
        .diagnostics()
        .iter()
        .find(|d| matches!(&d.kind, DiagnosticKind::UndefinedNodeReference(t) if t == "Missing"))
        .expect("undefined-node diagnostic for `Missing`");
    assert_eq!(
        diag.location.line, 2,
        "diagnostic should point at the `start:` line (2), got line {}",
        diag.location.line
    );
}

#[test]
fn undefined_exit_diagnostic_points_at_exit_line() {
    // `exit: Missing` is on line 3.
    let src = "workflow F\n  start: A\n  exit: Missing\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
    let err = parse(src, "test.dip").expect_err("expected validation failure");
    let diag = err
        .diagnostics()
        .iter()
        .find(|d| matches!(&d.kind, DiagnosticKind::UndefinedNodeReference(t) if t == "Missing"))
        .expect("undefined-node diagnostic for `Missing`");
    assert_eq!(diag.location.line, 3, "should point at exit line, got {}", diag.location.line);
}

#[test]
fn undefined_restart_target_diagnostic_points_at_defaults_field_line() {
    // `restart_target: Missing` is on line 5.
    let src = "workflow F\n  start: A\n  exit: A\n  defaults\n    restart_target: Missing\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
    let err = parse(src, "test.dip").expect_err("expected validation failure");
    let diag = err
        .diagnostics()
        .iter()
        .find(|d| matches!(&d.kind, DiagnosticKind::UndefinedNodeReference(t) if t == "Missing"))
        .expect("undefined-node diagnostic for `Missing`");
    assert_eq!(diag.location.line, 5, "should point at restart_target line, got {}", diag.location.line);
}
