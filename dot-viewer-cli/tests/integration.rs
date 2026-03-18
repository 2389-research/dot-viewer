// ABOUTME: Integration tests for the dot-viewer CLI ASCII rendering pipeline.
// ABOUTME: Verifies end-to-end rendering from DOT source to terminal output.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn run_cli(args: &[&str], input_dot: &str) -> String {
    let unique_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let tmp = std::env::temp_dir().join(format!("dot_viewer_test_{}_{}.dot", pid, unique_id));
    std::fs::write(&tmp, input_dot).unwrap();

    let mut cmd_args = vec![tmp.to_str().unwrap()];
    cmd_args.extend_from_slice(args);

    let output = Command::new(env!("CARGO_BIN_EXE_dot-viewer"))
        .args(&cmd_args)
        .output()
        .expect("Failed to run dot-viewer");

    let stderr = String::from_utf8(output.stderr).unwrap();
    if !stderr.is_empty() {
        eprintln!("CLI stderr: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout).unwrap();
    let _ = std::fs::remove_file(&tmp);
    stdout
}

#[test]
fn test_simple_linear_graph() {
    let output = run_cli(&[], "digraph { a -> b -> c }");
    assert!(output.contains("a"), "Should contain node a, got: {}", output);
    assert!(output.contains("b"), "Should contain node b");
    assert!(output.contains("c"), "Should contain node c");
    assert!(output.contains("┌"), "Should contain box-drawing chars");
    assert!(
        output.contains("▼") || output.contains("│"),
        "Should contain edge chars"
    );
}

#[test]
fn test_verbose_mode_shows_attributes() {
    let output = run_cli(
        &["-v"],
        r#"digraph { A [shape=box llm_model="sonnet"] }"#,
    );
    assert!(output.contains("A"), "Should contain node A, got: {}", output);
    assert!(
        output.contains("shape: box"),
        "Verbose should show shape attribute, got: {}",
        output
    );
    assert!(
        output.contains("llm_model: sonnet"),
        "Verbose should show llm_model, got: {}",
        output
    );
}

#[test]
fn test_labeled_nodes() {
    let output = run_cli(
        &[],
        r#"digraph { A [label="Hello World"]; B; A -> B }"#,
    );
    assert!(output.contains("Hello World"), "Should show node label");
    assert!(output.contains("B"), "Should show node B");
}

#[test]
fn test_empty_graph() {
    let output = run_cli(&[], "digraph { }");
    // Empty graph should produce minimal or empty output
    assert!(
        output.trim().is_empty() || output.len() < 100,
        "Empty graph should produce minimal output, got {} bytes: {}",
        output.len(),
        output
    );
}
