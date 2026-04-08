# Dippin CLI UX & Test Coverage Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Bring `dot-viewer-cli`'s `.dip` UX up to the level of the existing `.dot` path (format override, stdin, examples, distinct exit codes), and close the test-coverage gaps in `dippin-parser` (failure modes, Go-reference parity fixtures, golden snapshots, round-trip, CLI binary tests, fuzz harness).

**Architecture:** Add `--format` and stdin (`-`) support to the CLI, expand `--help`, dump-converted-DOT flag, strict engine validation. Port the 16 upstream Go testdata files. Add a dedicated `tests/error_cases.rs` covering every diagnostic branch. Add a `proptest` "doesn't panic" harness. Wire `ask_and_execute.dot` as a golden file.

**Tech Stack:** Rust 2021, `clap`, `proptest`, `dippin-parser`, `dot-viewer-cli`, `dot-parser` (for round-trip validation).

**Prerequisites:** `docs/plans/2026-04-07-dippin-correctness.md` (typed errors, structural diagnostics) must be merged. `docs/plans/2026-04-07-dippin-api-polish.md` is recommended but not required — most tasks here can run in parallel with it.

---

## Phase 1: CLI ergonomics

### Task 1: Strict engine validation via clap `ValueEnum`

**Files:**
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Failing test**

```bash
cargo run -p dot-viewer-cli -- --engine bogus dippin-parser/testdata/valid_minimal.dip
```
Expected today: silent fallback to `dot`. Goal: clap rejects with usage error.

**Step 2: Implement**

```rust
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Engine {
    Dot,
    Neato,
    Fdp,
    Sfdp,
    Twopi,
    Circo,
    Patchwork,
    Osage,
}
```

Replace the existing `engine: String` with `engine: Engine` (with `value_enum` and `default_value_t = Engine::Dot`). Delete `parse_engine`.

**Step 3: Manual verification**

```bash
cargo run -p dot-viewer-cli -- --engine bogus dippin-parser/testdata/valid_minimal.dip
```
Expected: clap rejects with `error: invalid value 'bogus' for '--engine <ENGINE>'`.

**Step 4: Commit**

```bash
git add dot-viewer-cli/src/main.rs
git commit -m "fix(cli): strict --engine validation via clap ValueEnum"
```

---

### Task 2: Add `--format` flag

**Files:**
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Add the enum + flag**

```rust
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum Format {
    #[default]
    Auto,
    Dot,
    Dip,
}

// In Cli struct:
#[arg(long, value_enum, default_value_t = Format::Auto)]
format: Format,
```

**Step 2: Update `resolve_dot_source`**

```rust
fn resolve_dot_source(file: &Path, format: Format, raw_source: &str) -> Result<String, ...> {
    let use_dip = match format {
        Format::Dip => true,
        Format::Dot => false,
        Format::Auto => file.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("dip"))
            .unwrap_or(false),
    };
    if use_dip {
        dippin_parser::parse_to_dot(raw_source, file)
            .map_err(|e| ...)
    } else {
        Ok(raw_source.to_string())
    }
}
```

**Step 3: Manual verification**

```bash
cp dippin-parser/testdata/valid_minimal.dip /tmp/foo.txt
cargo run -p dot-viewer-cli -- --format dip /tmp/foo.txt
```
Expected: parses successfully (override defeats extension check).

**Step 4: Commit**

```bash
git add dot-viewer-cli/src/main.rs
git commit -m "feat(cli): add --format dot|dip|auto override"
```

---

### Task 3: Add stdin support via `-`

**Files:**
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Detect `-` as path**

```rust
let raw_source = if cli.file.as_os_str() == "-" {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)
        .map_err(|e| { eprintln!("error reading stdin: {}", e); std::process::exit(EX_NOINPUT); })
        .unwrap();
    if matches!(cli.format, Format::Auto) {
        eprintln!("error: --format is required when reading from stdin");
        std::process::exit(EX_USAGE);
    }
    buf
} else {
    std::fs::read_to_string(&cli.file).unwrap_or_else(|e| {
        eprintln!("error reading {}: {}", cli.file.display(), e);
        std::process::exit(EX_NOINPUT);
    })
};
```

Add:
```rust
const EX_USAGE: i32 = 64;
```

**Step 2: Manual verification**

```bash
cat dippin-parser/testdata/valid_minimal.dip | cargo run -p dot-viewer-cli -- --format dip -
```
Expected: renders ASCII art.

**Step 3: Commit**

```bash
git add dot-viewer-cli/src/main.rs
git commit -m "feat(cli): support reading from stdin via `-`"
```

---

### Task 4: Expand `--help` text

**Files:**
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Add `long_about` and `after_help`**

```rust
#[command(
    name = "dot-viewer",
    about = "Render DOT and Dippin graph files as ASCII art in the terminal",
    long_about = "dot-viewer reads Graphviz DOT (.dot, .gv) and Dippin (.dip) files,\n\
                  lays them out with the chosen Graphviz engine, and renders the result\n\
                  as ASCII art in your terminal. The format is auto-detected from the\n\
                  file extension; override with --format.",
    after_help = "EXAMPLES:\n\
                  \x20\x20dot-viewer graph.dot\n\
                  \x20\x20dot-viewer workflow.dip --engine dot\n\
                  \x20\x20dot-viewer workflow.dip --show-dot\n\
                  \x20\x20cat workflow.dip | dot-viewer --format dip -"
)]
```

**Step 2: Verify**

```bash
cargo run -p dot-viewer-cli -- --help
```

**Step 3: Commit**

```bash
git add dot-viewer-cli/src/main.rs
git commit -m "docs(cli): expand --help with long description and examples"
```

---

### Task 5: Add `--show-dot` flag

**Files:**
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Add flag**

```rust
/// Print the converted DOT source to stdout instead of rendering it.
#[arg(long)]
show_dot: bool,
```

**Step 2: Wire it**

After `resolve_dot_source` returns, if `cli.show_dot`, `print!("{}", dot)` and `return Ok(())` instead of calling Graphviz.

**Step 3: Manual verify**

```bash
cargo run -p dot-viewer-cli -- --show-dot dippin-parser/testdata/valid_minimal.dip
```
Expected: prints `digraph Minimal { ... }`.

**Step 4: Commit**

```bash
git add dot-viewer-cli/src/main.rs
git commit -m "feat(cli): add --show-dot to dump converted DOT"
```

---

### Task 6: Add `--quiet` flag

**Files:**
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Add flag**

```rust
#[arg(short, long)]
quiet: bool,
```

**Step 2: Use it**

Suppress non-error stderr output (engine fallback warnings if any remain). Diagnostics still print.

**Step 3: Commit**

```bash
git add dot-viewer-cli/src/main.rs
git commit -m "feat(cli): add --quiet to suppress non-error stderr output"
```

---

### Task 7: CLI integration test for `.dip` happy path

**Files:**
- Modify: `dot-viewer-cli/tests/integration.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_renders_dip_file() {
    let tmp = std::env::temp_dir().join(format!("dot_viewer_test_{}.dip", std::process::id()));
    std::fs::write(&tmp, include_str!("../../dippin-parser/testdata/valid_minimal.dip")).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_dot-viewer"))
        .arg(&tmp)
        .output()
        .unwrap();

    let _ = std::fs::remove_file(&tmp);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Ask"));
    assert!(stdout.contains("Done"));
}
```

**Step 2: Run**

```bash
cargo test -p dot-viewer-cli test_renders_dip_file
```

**Step 3: Commit**

```bash
git add dot-viewer-cli/tests/integration.rs
git commit -m "test(cli): integration test for .dip file rendering"
```

---

### Task 8: CLI integration test for `.dip` error path

**Files:**
- Modify: `dot-viewer-cli/tests/integration.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_dip_parse_error_renders_diagnostics() {
    let tmp = std::env::temp_dir().join(format!("dot_viewer_bad_{}.dip", std::process::id()));
    std::fs::write(&tmp, "workflow\n  start: nope\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_dot-viewer"))
        .arg(&tmp)
        .output()
        .unwrap();

    let _ = std::fs::remove_file(&tmp);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(65));
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Diagnostic should be in path:line:col form
    assert!(stderr.contains(".dip:"), "expected file path in diagnostic, got: {}", stderr);
}
```

**Step 2: Run**

```bash
cargo test -p dot-viewer-cli test_dip_parse_error_renders_diagnostics
```

**Step 3: Commit**

```bash
git add dot-viewer-cli/tests/integration.rs
git commit -m "test(cli): integration test for .dip parse error UX"
```

---

## Phase 2: Failure-mode test coverage

### Task 9: Create `tests/error_cases.rs`

**Files:**
- Create: `dippin-parser/tests/error_cases.rs`

**Step 1: Write the failure-case battery**

```rust
// ABOUTME: Failure-mode tests for the dippin parser. Every diagnostic branch must be exercised here.
// ABOUTME: Each test asserts the expected DiagnosticKind appears in the error.

use dippin_parser::{parse, DiagnosticKind, Error};

fn assert_kind(src: &str, predicate: impl Fn(&DiagnosticKind) -> bool) {
    let err = parse(src, "test.dip").expect_err("expected parse to fail");
    let diags = err.diagnostics();
    assert!(
        diags.iter().any(|d| predicate(&d.kind)),
        "expected matching diagnostic, got: {:?}",
        diags
    );
}

#[test]
fn test_unterminated_string() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\nagent A\n  prompt: \"unterminated\n  model: m\n",
        |k| matches!(k, DiagnosticKind::UnterminatedString),
    );
}

#[test]
fn test_unknown_node_kind() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\nwizard A\n  prompt: x\n",
        |k| matches!(k, DiagnosticKind::Other) || matches!(k, DiagnosticKind::UnexpectedToken { .. }),
    );
}

#[test]
fn test_unknown_defaults_field() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  defaults\n    bogus: 1\nagent A\n  prompt: x\n  model: m\n  provider: p\n",
        |k| matches!(k, DiagnosticKind::UnknownField { scope, .. } if scope == "defaults"),
    );
}

#[test]
fn test_unknown_agent_field() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\nagent A\n  prompt: x\n  bogus: 1\n  model: m\n  provider: p\n",
        |k| matches!(k, DiagnosticKind::UnknownField { scope, .. } if scope == "agent"),
    );
}

#[test]
fn test_invalid_integer() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\n  defaults\n    max_retries: not_a_number\nagent A\n  prompt: x\n  model: m\n  provider: p\n",
        |k| matches!(k, DiagnosticKind::InvalidInteger { .. }),
    );
}

#[test]
fn test_invalid_float() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\nagent A\n  prompt: x\n  compaction_threshold: abc\n  model: m\n  provider: p\n",
        |k| matches!(k, DiagnosticKind::InvalidFloat { .. }),
    );
}

#[test]
fn test_invalid_duration() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\nagent A\n  prompt: x\n  cmd_timeout: 30\n  model: m\n  provider: p\n",
        |k| matches!(k, DiagnosticKind::InvalidDuration { .. }),
    );
}

#[test]
fn test_invalid_bool() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\nagent A\n  prompt: x\n  goal_gate: yes\n  model: m\n  provider: p\n",
        |k| matches!(k, DiagnosticKind::InvalidBool { .. }),
    );
}

#[test]
fn test_missing_workflow_identifier() {
    assert_kind(
        "workflow\n  start: A\n",
        |k| matches!(k, DiagnosticKind::MissingIdentifier { .. }),
    );
}

#[test]
fn test_missing_agent_identifier() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\nagent\n  prompt: x\n  model: m\n  provider: p\n",
        |k| matches!(k, DiagnosticKind::MissingIdentifier { .. }),
    );
}

#[test]
fn test_undefined_node_reference() {
    assert_kind(
        "workflow F\n  start: Missing\n  exit: A\nagent A\n  prompt: x\n  model: m\n  provider: p\n",
        |k| matches!(k, DiagnosticKind::UndefinedNodeReference(_)),
    );
}

#[test]
fn test_duplicate_workflow() {
    assert_kind(
        "workflow F\n  start: A\n  exit: A\nagent A\n  prompt: x\n  model: m\n  provider: p\nworkflow G\n",
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
```

**Step 2: Run**

```bash
cargo test -p dippin-parser --test error_cases
```

**Step 3: Commit**

```bash
git add dippin-parser/tests/error_cases.rs
git commit -m "test(parser): comprehensive failure-mode test battery"
```

---

### Task 10: Edge-case happy-path tests (single node, deep nesting, long lines)

**Files:**
- Modify: `dippin-parser/tests/integration_tests.rs`

**Step 1: Add tests**

```rust
#[test]
fn test_single_node_workflow() {
    let src = "workflow Solo\n  start: A\n  exit: A\nagent A\n  prompt: x\n  model: m\n  provider: p\n";
    let wf = parse(src, "solo.dip").unwrap();
    assert_eq!(wf.nodes.len(), 1);
    assert!(wf.edges.is_empty());
}

#[test]
fn test_long_lines() {
    let prompt = "x".repeat(8192);
    let src = format!("workflow Long\n  start: A\n  exit: A\nagent A\n  prompt: \"{}\"\n  model: m\n  provider: p\n", prompt);
    let wf = parse(&src, "long.dip").unwrap();
    let dippin_parser::NodeConfig::Agent(cfg) = &wf.nodes[0].config else { panic!() };
    assert_eq!(cfg.prompt.len(), 8192);
}

#[test]
fn test_trailing_whitespace_tolerated() {
    let src = "workflow F   \n  start: A   \n  exit: A   \nagent A   \n  prompt: x   \n  model: m   \n  provider: p   \n";
    let wf = parse(src, "ws.dip").unwrap();
    assert_eq!(wf.name, "F");
}
```

**Step 2: Run, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/tests/integration_tests.rs
git commit -m "test(parser): edge-case happy paths"
```

---

## Phase 3: Go-reference parity fixtures

### Task 11: Port the 16 Go reference testdata files

**Files:**
- Create: `dippin-parser/testdata/all_comments.dip`
- Create: `dippin-parser/testdata/all_features.dip`
- Create: `dippin-parser/testdata/defaults_complex.dip`
- Create: `dippin-parser/testdata/edge_attributes.dip`
- Create: `dippin-parser/testdata/edge_conditions.dip`
- Create: `dippin-parser/testdata/human_interview.dip`
- Create: `dippin-parser/testdata/human_node.dip`
- Create: `dippin-parser/testdata/human_prompt.dip`
- Create: `dippin-parser/testdata/minimal.dip`
- Create: `dippin-parser/testdata/multiline_prompt.dip`
- Create: `dippin-parser/testdata/parallel_branches.dip`
- Create: `dippin-parser/testdata/response_format.dip`
- Create: `dippin-parser/testdata/retry_fields.dip`
- Create: `dippin-parser/testdata/subgraph_params.dip`
- Create: `dippin-parser/testdata/tool_command.dip`
- Create: `dippin-parser/testdata/tool_outputs.dip`

**Step 1: Copy from upstream**

```bash
cp /Users/dylanr/work/2389/dippin-lang/parser/testdata/*.dip dippin-parser/testdata/
```

**Step 2: Verify they all parse**

Add a generated-style test to `tests/integration_tests.rs`:

```rust
#[test]
fn test_all_go_reference_fixtures_parse() {
    let fixtures = [
        "all_comments.dip",
        "all_features.dip",
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
        "subgraph_params.dip",
        "tool_command.dip",
        "tool_outputs.dip",
    ];
    for f in fixtures {
        let src = read_testdata(f);
        parse(&src, f).unwrap_or_else(|e| {
            for d in e.diagnostics() {
                eprintln!("{}", d.render());
            }
            panic!("fixture {} failed to parse", f);
        });
    }
}
```

**Step 3: Run**

```bash
cargo test -p dippin-parser test_all_go_reference_fixtures_parse
```

If any fixture fails, **investigate**: it likely surfaces a parity bug fixed in the correctness plan that wasn't fully covered. Fix and commit per fixture.

**Step 4: Commit**

```bash
git add dippin-parser/testdata/*.dip dippin-parser/tests/integration_tests.rs
git commit -m "test(parser): port 16 Go reference testdata fixtures"
```

---

### Task 12: Per-fixture assertion tests

**Files:**
- Modify: `dippin-parser/tests/integration_tests.rs`

**Step 1: Add focused assertions**

For each ported fixture, add a small test that checks the parsed `Workflow` has the expected key fields. Example:

```rust
#[test]
fn test_parallel_branches_fixture() {
    let src = read_testdata("parallel_branches.dip");
    let wf = parse(&src, "parallel_branches.dip").unwrap();
    let parallel = wf.nodes.iter().find(|n| n.kind == NodeKind::Parallel).unwrap();
    let dippin_parser::NodeConfig::Parallel(cfg) = &parallel.config else { panic!() };
    assert!(!cfg.branches.is_empty(), "parallel block form should have branches");
}

#[test]
fn test_subgraph_params_fixture() {
    let src = read_testdata("subgraph_params.dip");
    let wf = parse(&src, "subgraph_params.dip").unwrap();
    let sg = wf.nodes.iter().find(|n| n.kind == NodeKind::Subgraph).unwrap();
    let dippin_parser::NodeConfig::Subgraph(cfg) = &sg.config else { panic!() };
    assert!(!cfg.params.is_empty(), "subgraph should have params");
}

#[test]
fn test_tool_outputs_fixture() {
    let src = read_testdata("tool_outputs.dip");
    let wf = parse(&src, "tool_outputs.dip").unwrap();
    let tool = wf.nodes.iter().find(|n| n.kind == NodeKind::Tool).unwrap();
    let dippin_parser::NodeConfig::Tool(cfg) = &tool.config else { panic!() };
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
    let agent = wf.nodes.iter().find(|n| n.kind == NodeKind::Agent).unwrap();
    assert!(agent.retry.max_retries > 0);
}

#[test]
fn test_response_format_fixture() {
    let src = read_testdata("response_format.dip");
    let wf = parse(&src, "response_format.dip").unwrap();
    let agent = wf.nodes.iter().find(|n| n.kind == NodeKind::Agent).unwrap();
    let dippin_parser::NodeConfig::Agent(cfg) = &agent.config else { panic!() };
    assert!(!cfg.response_format.is_empty());
}

#[test]
fn test_multiline_prompt_fixture() {
    let src = read_testdata("multiline_prompt.dip");
    let wf = parse(&src, "multiline_prompt.dip").unwrap();
    let agent = wf.nodes.iter().find(|n| n.kind == NodeKind::Agent).unwrap();
    let dippin_parser::NodeConfig::Agent(cfg) = &agent.config else { panic!() };
    assert!(cfg.prompt.contains('\n'), "multiline prompt should preserve newlines");
}

#[test]
fn test_defaults_complex_fixture() {
    let src = read_testdata("defaults_complex.dip");
    let wf = parse(&src, "defaults_complex.dip").unwrap();
    assert!(!wf.defaults.fidelity.is_empty());
    assert!(wf.defaults.max_retries > 0);
}
```

**Step 2: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/tests/integration_tests.rs
git commit -m "test(parser): per-fixture field assertions for Go reference data"
```

---

## Phase 4: Round-trip & golden tests

### Task 13: Wire up `ask_and_execute.dot` as a golden snapshot

**Files:**
- Modify: `dippin-parser/tests/integration_tests.rs`

**Step 1: Add golden test**

```rust
#[test]
fn test_ask_and_execute_matches_golden() {
    let src = read_testdata("ask_and_execute.dip");
    let dot = parse_to_dot(&src, "ask_and_execute.dip").expect("convert");
    let golden = read_testdata("ask_and_execute.dot");
    if dot.trim() != golden.trim() {
        // Write the actual output to a sibling file for diffing.
        let actual_path = testdata_path("ask_and_execute.dot.actual");
        std::fs::write(&actual_path, &dot).unwrap();
        panic!(
            "DOT output does not match golden. See {} vs ask_and_execute.dot",
            actual_path
        );
    }
}
```

**Step 2: Run**

```bash
cargo test -p dippin-parser test_ask_and_execute_matches_golden
```

If it fails: inspect the diff. Either the golden is stale (update it) or the exporter has drifted (fix it).

**Step 3: Add `ask_and_execute.dot.actual` to `.gitignore`**

```bash
echo "dippin-parser/testdata/*.actual" >> .gitignore
git add .gitignore
```

**Step 4: Commit**

```bash
git add dippin-parser/tests/integration_tests.rs .gitignore
# possibly: git add dippin-parser/testdata/ask_and_execute.dot   # if updated
git commit -m "test(parser): wire ask_and_execute.dot as golden snapshot"
```

---

### Task 14: Round-trip test using `dot-parser`

**Files:**
- Modify: `dippin-parser/Cargo.toml` (dev-dep)
- Modify: `dippin-parser/tests/integration_tests.rs`

**Step 1: Add dev-dep**

```toml
[dev-dependencies]
dot-parser = { path = "../dot-parser" }
```

**Step 2: Add the round-trip test**

```rust
#[test]
fn test_dot_output_is_parseable_by_dot_parser() {
    for fixture in ["valid_minimal.dip", "multi_provider.dip", "ask_and_execute.dip"] {
        let src = read_testdata(fixture);
        let dot = parse_to_dot(&src, fixture).expect("convert");
        // dot-parser should be able to re-parse our output
        let parsed = dot_parser::parse(&dot)
            .unwrap_or_else(|e| panic!("dot-parser rejected output of {}: {:?}", fixture, e));
        assert!(!parsed.nodes().is_empty(), "round-tripped graph has no nodes");
    }
}
```

(Adjust the `dot_parser::parse` call to match the actual API in the workspace's `dot-parser` crate.)

**Step 3: Run, commit**

```bash
cargo test -p dippin-parser test_dot_output_is_parseable_by_dot_parser
git add dippin-parser/Cargo.toml dippin-parser/tests/integration_tests.rs
git commit -m "test(parser): round-trip parser output through dot-parser"
```

---

### Task 15: Stylesheet parsing test

**Files:**
- Create: `dippin-parser/testdata/stylesheet.dip`
- Modify: `dippin-parser/tests/integration_tests.rs`

**Step 1: Create fixture**

`dippin-parser/testdata/stylesheet.dip`:
```text
workflow Styled
  start: A
  exit: B

agent A
  prompt: x
  model: m
  provider: p

agent B
  prompt: y
  model: m
  provider: p

edges
  A -> B

stylesheet
  *
    fontname: Helvetica
  .important
    color: red
  #A
    shape: box
  agent
    fillcolor: lightblue
```

**Step 2: Add test**

```rust
#[test]
fn test_stylesheet_parses_all_selector_types() {
    let src = read_testdata("stylesheet.dip");
    let wf = parse(&src, "stylesheet.dip").expect("parse");
    assert_eq!(wf.stylesheet.len(), 4);
    use dippin_parser::StyleSelector;
    assert!(wf.stylesheet.iter().any(|r| matches!(r.selector, StyleSelector::Universal)));
    assert!(wf.stylesheet.iter().any(|r| matches!(&r.selector, StyleSelector::Class(c) if c == "important")));
    assert!(wf.stylesheet.iter().any(|r| matches!(&r.selector, StyleSelector::Id(id) if id == "A")));
    assert!(wf.stylesheet.iter().any(|r| matches!(&r.selector, StyleSelector::Kind(k) if k == "agent")));
}
```

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser test_stylesheet_parses_all_selector_types
git add dippin-parser/testdata/stylesheet.dip dippin-parser/tests/integration_tests.rs
git commit -m "test(parser): stylesheet selector coverage"
```

---

### Task 16: Edge weight + label coverage

**Files:**
- Modify: `dippin-parser/tests/integration_tests.rs`

**Step 1: Add test**

```rust
#[test]
fn test_edge_weight_and_label() {
    let src = "workflow F\n  start: A\n  exit: B\nagent A\n  prompt: x\n  model: m\n  provider: p\nagent B\n  prompt: y\n  model: m\n  provider: p\nedges\n  A -> B label: \"go\" weight: 5\n";
    let wf = parse(src, "ew.dip").unwrap();
    let edge = &wf.edges[0];
    assert_eq!(edge.label, "go");
    assert_eq!(edge.weight, 5);
}
```

**Step 2: Test, commit**

```bash
cargo test -p dippin-parser test_edge_weight_and_label
git add dippin-parser/tests/integration_tests.rs
git commit -m "test(parser): edge weight and label coverage"
```

---

### Task 17: All `WorkflowDefaults` field coverage

**Files:**
- Modify: `dippin-parser/tests/integration_tests.rs`

**Step 1: Add test**

```rust
#[test]
fn test_all_workflow_defaults_fields() {
    let src = r#"workflow F
  start: A
  exit: A
  defaults
    model: claude-sonnet-4-6
    provider: anthropic
    retry_policy: exponential
    max_retries: 5
    fidelity: summary:medium
    max_restarts: 2
    restart_target: A
    cache_tools: true
    compaction: auto
    on_resume: continue
agent A
  prompt: x
  model: m
  provider: p
"#;
    let wf = parse(src, "defaults.dip").unwrap();
    assert_eq!(wf.defaults.model, "claude-sonnet-4-6");
    assert_eq!(wf.defaults.provider, "anthropic");
    assert_eq!(wf.defaults.retry_policy, "exponential");
    assert_eq!(wf.defaults.max_retries, 5);
    assert_eq!(wf.defaults.fidelity, "summary:medium");
    assert_eq!(wf.defaults.max_restarts, 2);
    assert_eq!(wf.defaults.restart_target, "A");
    assert!(wf.defaults.cache_tools);
    assert_eq!(wf.defaults.compaction, "auto");
    assert_eq!(wf.defaults.on_resume, "continue");
}
```

**Step 2: Test, commit**

```bash
cargo test -p dippin-parser test_all_workflow_defaults_fields
git add dippin-parser/tests/integration_tests.rs
git commit -m "test(parser): cover every WorkflowDefaults field"
```

---

## Phase 5: Property testing

### Task 18: Add `proptest` "doesn't panic" harness

**Files:**
- Modify: `dippin-parser/Cargo.toml`
- Create: `dippin-parser/tests/proptest_smoke.rs`

**Step 1: Add dev-dep**

```toml
[dev-dependencies]
proptest = "1"
```

**Step 2: Write the smoke test**

```rust
// ABOUTME: Property test ensuring the parser never panics on arbitrary input.
// ABOUTME: Generates random ASCII strings and asserts parse() returns Ok or Err — never panic.

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn parse_does_not_panic_on_random_ascii(s in "[\\x20-\\x7e\\n\\t]{0,2048}") {
        let _ = dippin_parser::parse(&s, "fuzz.dip");
    }

    #[test]
    fn parse_does_not_panic_on_random_unicode(s in "\\PC{0,512}") {
        let _ = dippin_parser::parse(&s, "fuzz.dip");
    }
}
```

**Step 3: Run**

```bash
cargo test -p dippin-parser --test proptest_smoke
```

This will run 2,000 cases and surface any panic. If a panic is found, it'll print a minimal repro. Fix any panics found, then re-run.

**Step 4: Commit**

```bash
git add dippin-parser/Cargo.toml dippin-parser/tests/proptest_smoke.rs
git commit -m "test(parser): proptest smoke harness for panic-freedom"
```

---

### Task 19: Round-trip property test

**Files:**
- Modify: `dippin-parser/tests/proptest_smoke.rs`

**Step 1: Add round-trip test for valid fixtures**

```rust
proptest! {
    #[test]
    fn round_trip_preserves_node_count(seed in 0u64..10) {
        let fixtures = ["valid_minimal.dip", "multi_provider.dip", "ask_and_execute.dip"];
        let f = fixtures[seed as usize % fixtures.len()];
        let src = std::fs::read_to_string(format!("{}/testdata/{}", env!("CARGO_MANIFEST_DIR"), f)).unwrap();
        let wf1 = dippin_parser::parse(&src, f).unwrap();
        let dot = wf1.to_dot(&dippin_parser::ExportOptions::default());
        // Cannot re-parse DOT as .dip, so just assert structural invariants
        prop_assert!(!wf1.nodes.is_empty());
        prop_assert!(dot.contains("digraph"));
    }
}
```

**Step 2: Test, commit**

```bash
cargo test -p dippin-parser --test proptest_smoke
git add dippin-parser/tests/proptest_smoke.rs
git commit -m "test(parser): proptest round-trip invariants"
```

---

## Phase 6: Fresh-eyes coverage check

### Task 20: Verify diagnostic-branch coverage with `cargo-llvm-cov`

**Files:** none

**Step 1: Install if needed**

```bash
cargo install cargo-llvm-cov 2>&1 | tail
```

**Step 2: Run coverage**

```bash
cargo llvm-cov -p dippin-parser --html
open target/llvm-cov/html/index.html
```

**Step 3: Look for red lines in `parser.rs` and `lexer.rs`**

Note any uncovered diagnostic branches. For each one, add a targeted test in `tests/error_cases.rs`.

**Step 4: Commit any new tests**

```bash
git add dippin-parser/tests/error_cases.rs
git commit -m "test(parser): close coverage gaps surfaced by llvm-cov"
```

(Skip this task if `cargo-llvm-cov` install fails — it's a verification step, not blocking.)

---

## Final verification

### Task 21: Full sweep

```bash
cargo test -p dippin-parser
cargo test -p dippin-parser --features serde
cargo test -p dot-viewer-cli   # may fail without graphviz-vendor; tolerate that
cargo clippy -p dippin-parser -- -D warnings
cargo clippy -p dot-viewer-cli -- -D warnings 2>&1 | tail
cargo doc -p dippin-parser --no-deps
```

Fix anything that fails, recommit.

---

## Notes for the executing engineer

- Tasks in Phases 1 (CLI) and Phase 2+ (parser tests) are mostly independent — you can interleave or split across sessions.
- The `dot-viewer-cli` tests in Tasks 7–8 require the binary to actually build, which means `dot-core` and `graphviz-vendor` must be present. If running in a fresh worktree, expect a longer first build. If `graphviz-vendor` isn't cloned, those CLI tests will fail at the build step — clone it per project memory: `cd dot-core && git clone --depth 1 --branch 12.2.1 https://gitlab.com/graphviz/graphviz.git graphviz-vendor`.
- Tasks 11–12 (port Go fixtures) may surface latent parity bugs not caught by the correctness plan. **Investigate every failure** rather than rubber-stamping the test. Each failure is a discovery, not a defect in the plan.
- The `superpowers:test-driven-development` skill applies to behavior changes (Tasks 1–8). Tests-only tasks (Phases 2–6) follow red → green → commit per test.
- The `superpowers:scenario-testing` skill (real dependencies, no mocks) is enforced — every test here uses real files, real CLI invocations, real `dot-parser`.
