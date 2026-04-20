# Dippin Parser API Polish & Documentation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Harden the public API of `dippin-parser` for semver evolution, replace stringly-typed config fields with real enums, complete the documentation surface, and fill out crate metadata.

**Architecture:** Make module visibility correct (`pub(crate)` for lexer/parser internals), mark every public type `#[non_exhaustive]`, replace `String` config sentinels with enums, wire up the declared `serde` feature, and write rustdoc + README + Cargo.toml metadata so the crate is publishable.

**Tech Stack:** Rust 2021, `dippin-parser` workspace crate, `indexmap`, `serde` (optional feature), rustdoc.

**Prerequisites:** `docs/plans/2026-04-07-dippin-correctness.md` must be merged first (this plan depends on the typed `Error`/`Diagnostic` types and the structured `Duration`).

**Companion plan:** `docs/plans/2026-04-07-dippin-ux-and-tests.md` (CLI UX + tests) — can run before, after, or in parallel.

---

## Phase 1: Module visibility & re-exports

### Task 1: Make `lexer` and `parser` modules `pub(crate)`

**Files:**
- Modify: `dippin-parser/src/lib.rs`
- Modify: `dippin-parser/src/lexer.rs`, `dippin-parser/src/parser.rs` (only if any consumer outside the crate needed direct access)

**Step 1: Change visibility**

In `lib.rs`:
```rust
pub(crate) mod lexer;
pub(crate) mod parser;
pub mod export_dot;
pub mod ir;
pub mod error;
pub mod duration;
pub mod validate;
```

**Step 2: Build**

```bash
cargo build -p dippin-parser
```

If anything fails because `dot-viewer-cli` or another consumer imported from `dippin_parser::lexer::*`, fix the import to use the public re-exports instead.

**Step 3: Test**

```bash
cargo test -p dippin-parser
cargo build -p dot-viewer-cli
```

**Step 4: Commit**

```bash
git add dippin-parser/src/lib.rs
git commit -m "refactor(api): make lexer and parser modules pub(crate)"
```

---

### Task 2: Re-export full IR from crate root

**Files:**
- Modify: `dippin-parser/src/lib.rs`

**Step 1: Add re-exports**

```rust
pub use ir::{
    AgentConfig, BranchConfig, Condition, Edge, FanInConfig, HumanConfig, Node, NodeConfig,
    NodeIO, NodeKind, ParallelConfig, RetryConfig, SourceLocation, StyleSelector,
    StylesheetRule, SubgraphConfig, ToolConfig, Workflow, WorkflowDefaults,
};
pub use export_dot::ExportOptions;
```

Remove the bare `pub use export_dot::{export_dot as export_dot_string, ExportOptions};` line from a previous task — `export_dot_string` is no longer needed (see Task 4).

**Step 2: Test**

```bash
cargo build -p dippin-parser
cargo test -p dippin-parser
```

**Step 3: Commit**

```bash
git add dippin-parser/src/lib.rs
git commit -m "feat(api): re-export complete IR from crate root"
```

---

### Task 3: Add `#[non_exhaustive]` to every public type

**Files:**
- Modify: `dippin-parser/src/ir.rs`
- Modify: `dippin-parser/src/export_dot.rs`
- Modify: `dippin-parser/src/error.rs`

**Step 1: Annotate every `pub struct` and `pub enum`**

For each type in `ir.rs`:
```rust
#[non_exhaustive]
pub struct Workflow { ... }
```

Apply to: `Workflow`, `WorkflowDefaults`, `Node`, `NodeKind`, `NodeConfig`, `AgentConfig`, `HumanConfig`, `ToolConfig`, `ParallelConfig`, `BranchConfig`, `FanInConfig`, `SubgraphConfig`, `RetryConfig`, `NodeIO`, `Edge`, `Condition`, `StylesheetRule`, `StyleSelector`, `SourceLocation`.

In `export_dot.rs`: `ExportOptions`.

In `error.rs`: `Error`, `DiagnosticKind`, `Diagnostic`, `Severity`.

**Step 2: Fix any internal struct-literal construction sites**

`#[non_exhaustive]` allows construction inside the defining crate; outside, callers must use `..Default::default()`. The internal tests already use `..Default::default()` patterns, so this should be a clean change.

**Step 3: Test**

```bash
cargo test -p dippin-parser
cargo build -p dot-viewer-cli
```

If `dot-viewer-cli` constructs any IR struct directly, switch to `..Default::default()`.

**Step 4: Commit**

```bash
git add dippin-parser/src/ir.rs dippin-parser/src/export_dot.rs dippin-parser/src/error.rs
git commit -m "feat(api): mark all public types non_exhaustive for semver"
```

---

## Phase 2: API ergonomics

### Task 4: Unify naming on `to_dot` / `parse_to_dot`

**Files:**
- Modify: `dippin-parser/src/lib.rs`
- Modify: `dippin-parser/src/export_dot.rs`

**Step 1: Add inherent method on `Workflow`**

In `export_dot.rs`:
```rust
impl crate::ir::Workflow {
    /// Render this workflow as a DOT graph.
    pub fn to_dot(&self, opts: &ExportOptions) -> String {
        export_dot(self, opts)
    }
}
```

**Step 2: Replace top-level functions**

In `lib.rs`, replace `convert_to_dot` and `convert_to_dot_with_options` with:

```rust
/// Parse and convert to DOT in a single call.
pub fn parse_to_dot(source: &str, filename: &str) -> Result<String> {
    parse_to_dot_with_options(source, filename, &ExportOptions::default())
}

/// Parse and convert to DOT with custom export options.
pub fn parse_to_dot_with_options(
    source: &str,
    filename: &str,
    opts: &ExportOptions,
) -> Result<String> {
    let wf = parse(source, filename)?;
    Ok(wf.to_dot(opts))
}
```

Keep `pub use convert_to_dot = parse_to_dot;` as a deprecated alias for one release? **No** — this is pre-1.0, drop the old names cleanly. Update all callers.

**Step 3: Update callers**

- `dippin-parser/tests/integration_tests.rs`
- `dippin-parser/src/lib.rs` inline tests
- `dot-viewer-cli/src/main.rs`

**Step 4: Test**

```bash
cargo test -p dippin-parser
cargo build -p dot-viewer-cli
```

**Step 5: Commit**

```bash
git add dippin-parser/src/lib.rs dippin-parser/src/export_dot.rs dippin-parser/tests/integration_tests.rs dot-viewer-cli/src/main.rs
git commit -m "refactor(api): rename convert_to_dot* to parse_to_dot*, add Workflow::to_dot"
```

---

### Task 5: Replace `ExportOptions.rank_dir: String` with `RankDir` enum

**Files:**
- Modify: `dippin-parser/src/export_dot.rs`

**Step 1: Define the enum**

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RankDir {
    #[default]
    TopBottom,
    LeftRight,
    BottomTop,
    RightLeft,
}

impl RankDir {
    pub fn as_dot(&self) -> &'static str {
        match self {
            RankDir::TopBottom => "TB",
            RankDir::LeftRight => "LR",
            RankDir::BottomTop => "BT",
            RankDir::RightLeft => "RL",
        }
    }
}
```

**Step 2: Use it in `ExportOptions`**

```rust
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ExportOptions {
    pub include_prompts: bool,
    pub rank_dir: RankDir,
    pub highlight_goal_gates: bool,
    pub execution_path: Vec<String>,
}
```

In `write_dot_header`, use `opts.rank_dir.as_dot()`.

**Step 3: Re-export `RankDir`**

```rust
// in lib.rs
pub use export_dot::{ExportOptions, RankDir};
```

**Step 4: Update callers (tests, CLI)**

**Step 5: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/export_dot.rs dippin-parser/src/lib.rs
git commit -m "refactor(api): RankDir enum replaces stringly-typed rank_dir"
```

---

### Task 6: Replace `StyleSelector { kind: String }` with enum

**Files:**
- Modify: `dippin-parser/src/ir.rs`, `dippin-parser/src/parser.rs`, `dippin-parser/src/export_dot.rs`

**Step 1: Define**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StyleSelector {
    Universal,
    Class(String),
    Id(String),
    Kind(String),
}
```

**Step 2: Update parser**

In `parse_selector`, return the typed enum directly instead of a string-stringly struct.

**Step 3: Update any consumer**

If `export_dot.rs` reads stylesheet selectors, switch to a `match`.

**Step 4: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/ir.rs dippin-parser/src/parser.rs dippin-parser/src/export_dot.rs
git commit -m "refactor(ir): StyleSelector is now an enum"
```

---

### Task 7: Use `IndexMap` for params

**Files:**
- Modify: `dippin-parser/Cargo.toml`, `dippin-parser/src/ir.rs`, `dippin-parser/src/parser.rs`, `dippin-parser/src/export_dot.rs`

**Step 1: Add dep**

```toml
indexmap = "2"
```

**Step 2: Switch types**

In `ir.rs`:
- `AgentConfig.params: HashMap<String, String>` → `IndexMap<String, String>`
- `SubgraphConfig.params: HashMap<String, String>` → `IndexMap<String, String>`
- `StylesheetRule.properties: HashMap<String, String>` → `IndexMap<String, String>`

**Step 3: Update construction sites**

`HashMap::new()` → `IndexMap::new()`.

**Step 4: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/Cargo.toml dippin-parser/src/ir.rs dippin-parser/src/parser.rs dippin-parser/src/export_dot.rs
git commit -m "refactor(ir): use IndexMap to preserve source order in params"
```

---

### Task 8: Move `default_node_config` into `impl NodeConfig`

**Files:**
- Modify: `dippin-parser/src/ir.rs`, `dippin-parser/src/parser.rs`

**Step 1: Refactor**

```rust
impl NodeConfig {
    pub fn default_for(kind: &NodeKind) -> Self {
        match kind {
            NodeKind::Agent => NodeConfig::Agent(AgentConfig::default()),
            NodeKind::Human => NodeConfig::Human(HumanConfig::default()),
            // ...
        }
    }
}
```

Delete the free function. Update parser callers.

**Step 2: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/ir.rs dippin-parser/src/parser.rs
git commit -m "refactor(ir): default_node_config becomes NodeConfig::default_for"
```

---

### Task 9: Add `Display` for `NodeKind` and other missing trait derives

**Files:**
- Modify: `dippin-parser/src/ir.rs`

**Step 1: Add derives**

To `NodeKind`: add `Hash` (and `Copy` if it's small enough — it is, no payload):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NodeKind { ... }
```

**Step 2: Implement `Display`**

```rust
impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NodeKind::Agent => "agent",
            NodeKind::Human => "human",
            NodeKind::Tool => "tool",
            NodeKind::Parallel => "parallel",
            NodeKind::FanIn => "fan_in",
            NodeKind::Subgraph => "subgraph",
        })
    }
}
```

**Step 3: Add `PartialEq`/`Eq` to all IR structs where derivable**

`Workflow`, `Node`, `Edge`, `Condition`, `StylesheetRule`, `AgentConfig`, etc. — derive `PartialEq, Eq` on each. Note that `NodeConfig` will need it too.

**Step 4: Round-trip symmetry test**

```rust
#[test]
fn test_node_kind_display_fromstr_roundtrip() {
    use std::str::FromStr;
    for k in &[NodeKind::Agent, NodeKind::Human, NodeKind::Tool, NodeKind::Parallel, NodeKind::FanIn, NodeKind::Subgraph] {
        let s = k.to_string();
        assert_eq!(NodeKind::from_str(&s).unwrap(), *k);
    }
}
```

**Step 5: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/ir.rs
git commit -m "feat(ir): Display impl and Hash/PartialEq derives on IR types"
```

---

### Task 10: Use `&Path` for filename parameter

**Files:**
- Modify: `dippin-parser/src/lib.rs`, `dippin-parser/src/parser.rs`, `dippin-parser/src/lexer.rs`, `dippin-parser/src/error.rs`, `dippin-parser/tests/integration_tests.rs`, `dot-viewer-cli/src/main.rs`

**Step 1: Change public API signatures**

```rust
pub fn parse(source: &str, filename: impl AsRef<Path>) -> Result<Workflow> { ... }
pub fn parse_to_dot(source: &str, filename: impl AsRef<Path>) -> Result<String> { ... }
pub fn parse_to_dot_with_options(
    source: &str,
    filename: impl AsRef<Path>,
    opts: &ExportOptions,
) -> Result<String> { ... }
```

Internally convert to `String` once at the entry point: `let filename = filename.as_ref().to_string_lossy().into_owned();`.

**Step 2: Update callers**

`dot-viewer-cli/src/main.rs` can now pass `&cli.file` directly.

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
cargo build -p dot-viewer-cli
git add dippin-parser/src/lib.rs dot-viewer-cli/src/main.rs dippin-parser/tests/integration_tests.rs
git commit -m "refactor(api): accept impl AsRef<Path> for filename parameter"
```

---

### Task 11: Reduce token cloning with `Arc<str>` filenames

**Files:**
- Modify: `dippin-parser/src/lexer.rs`, `dippin-parser/src/ir.rs`

**Step 1: Change `SourceLocation.file: String` to `Arc<str>`**

```rust
use std::sync::Arc;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceLocation {
    pub file: Arc<str>,
    pub line: usize,
    pub column: usize,
}
```

**Step 2: Construct once in `Lexer::new`**

```rust
pub fn new(input: String, filename: String) -> Self {
    let filename: Arc<str> = Arc::from(filename);
    // ... store filename and clone the Arc cheaply for each token
}
```

**Step 3: Update all `SourceLocation { file: filename.clone(), ... }` sites — `Arc::clone` is cheap**

**Step 4: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/lexer.rs dippin-parser/src/ir.rs
git commit -m "perf(lexer): share filename via Arc<str> across tokens"
```

---

## Phase 3: Feature flags (serde / uniffi)

### Task 12: Wire up `serde` feature

**Files:**
- Modify: `dippin-parser/Cargo.toml`, `dippin-parser/src/ir.rs`, `dippin-parser/src/error.rs`, `dippin-parser/src/duration.rs`, `dippin-parser/src/export_dot.rs`

**Step 1: Verify Cargo.toml feature declaration**

```toml
[features]
default = []
serde = ["dep:serde"]

[dependencies]
serde = { version = "1", features = ["derive"], optional = true }
```

**Step 2: Add cfg-gated derives to every public IR type**

```rust
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Workflow { ... }
```

Apply to every public struct/enum in `ir.rs`, `error.rs`, `duration.rs` (note: `Duration` wraps `std::time::Duration` which has `serde` support behind its own feature — handle with `serde_with` or a manual impl), and `export_dot.rs::ExportOptions`.

**Step 3: Add a feature test**

Add to `dippin-parser/tests/serde_feature.rs`:
```rust
// ABOUTME: Verifies the optional `serde` feature compiles and round-trips.
// ABOUTME: Only compiled when `--features serde` is enabled.

#![cfg(feature = "serde")]

#[test]
fn test_workflow_roundtrips_through_json() {
    let src = "workflow F\n  start: A\n  exit: A\nagent A\n  prompt: x\n  model: m\n  provider: p\n";
    let wf = dippin_parser::parse(src, "t.dip").unwrap();
    let json = serde_json::to_string(&wf).unwrap();
    let _back: dippin_parser::Workflow = serde_json::from_str(&json).unwrap();
}
```

Add `serde_json = "1"` as a dev-dep.

**Step 4: Run with feature**

```bash
cargo test -p dippin-parser --features serde
```

**Step 5: Commit**

```bash
git add dippin-parser/Cargo.toml dippin-parser/src/ir.rs dippin-parser/src/error.rs dippin-parser/src/duration.rs dippin-parser/src/export_dot.rs dippin-parser/tests/serde_feature.rs
git commit -m "feat(serde): wire up optional serde feature with derives"
```

---

### Task 13: Decide on `uniffi` feature — wire up or remove

**Files:**
- Modify: `dippin-parser/Cargo.toml`

**Step 1: Check whether anything in the workspace uses `dippin-parser` via UniFFI**

```bash
grep -rn "dippin_parser" dot-core/ DotViewer/ 2>&1 | head
```

**Step 2: If unused, remove the feature**

```toml
# Drop these lines from Cargo.toml:
# uniffi = ["dep:uniffi"]
# uniffi = { version = "...", optional = true }
```

Remove the ABOUTME mention if it claims uniffi support.

**Step 3: If used, add `uniffi::Record`/`uniffi::Enum` derives** to the relevant IR types (you'd know which from the consumer).

**Step 4: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/Cargo.toml
git commit -m "chore(deps): remove unused uniffi feature from dippin-parser"
```

(Adjust message if you wired it up instead.)

---

## Phase 4: Cargo metadata & lints

### Task 14: Fill out `dippin-parser/Cargo.toml`

**Files:**
- Modify: `dippin-parser/Cargo.toml`

**Step 1: Add metadata**

```toml
[package]
name = "dippin-parser"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
description = "Parser and DOT exporter for the Dippin AI workflow DSL."
license = "MIT OR Apache-2.0"
repository = "https://github.com/2389-research/dot-viewer"
documentation = "https://docs.rs/dippin-parser"
readme = "README.md"
keywords = ["dippin", "parser", "graphviz", "dot", "workflow"]
categories = ["parser-implementations", "compilers"]

[lints.clippy]
all = "warn"
pedantic = "warn"
```

**Step 2: Verify `cargo build` still passes with the new lints**

Address any clippy warnings introduced by `pedantic` (or downgrade to `warn` and address selectively).

**Step 3: Commit**

```bash
git add dippin-parser/Cargo.toml
git commit -m "chore(meta): fill out dippin-parser Cargo.toml metadata and lints"
```

---

## Phase 5: Documentation

### Task 15: Add crate-level rustdoc

**Files:**
- Modify: `dippin-parser/src/lib.rs`

**Step 1: Add `//!` block above the existing `// ABOUTME` lines**

```rust
//! # dippin-parser
//!
//! A Rust parser and DOT exporter for the Dippin DSL — a higher-level authoring
//! format for AI agent workflows that lowers to Graphviz DOT for visualization.
//!
//! This crate is a port of the upstream Go implementation at
//! [github.com/2389-research/dippin-lang](https://github.com/2389-research/dippin-lang).
//! See that repository for the canonical language reference.
//!
//! ## Quick start
//!
//! ```
//! use dippin_parser::{parse, parse_to_dot};
//!
//! let source = r#"
//! workflow Greet
//!   start: Ask
//!   exit: Done
//!
//! agent Ask
//!   prompt: "Hi!"
//!   model: claude-sonnet-4-6
//!   provider: anthropic
//!
//! agent Done
//!   prompt: "Bye!"
//!   model: gpt-4.1-nano
//!   provider: openai
//!
//! edges
//!   Ask -> Done
//! "#;
//!
//! let wf = parse(source, "greet.dip").unwrap();
//! assert_eq!(wf.name, "Greet");
//!
//! let dot = parse_to_dot(source, "greet.dip").unwrap();
//! assert!(dot.contains("digraph Greet {"));
//! ```
//!
//! ## Features
//!
//! - `serde` — derives `Serialize`/`Deserialize` for IR types.
//!
//! ## Stability
//!
//! Pre-1.0. All public types are `#[non_exhaustive]`.
```

**Step 2: Test**

```bash
cargo test -p dippin-parser --doc
```

**Step 3: Commit**

```bash
git add dippin-parser/src/lib.rs
git commit -m "docs(parser): add crate-level rustdoc with quick-start example"
```

---

### Task 16: Document every public function with `# Examples` and `# Errors`

**Files:**
- Modify: `dippin-parser/src/lib.rs`

**Step 1: For each of `parse`, `parse_to_dot`, `parse_to_dot_with_options`**

Add a doc block:
```rust
/// Parse a Dippin source string into a [`Workflow`].
///
/// # Arguments
///
/// * `source` — the `.dip` source text.
/// * `filename` — used in diagnostics; pass an empty path for inline content.
///
/// # Errors
///
/// Returns [`Error::Parse`] containing one or more [`Diagnostic`]s if the source
/// has syntax errors, references undefined nodes, or exceeds [`MAX_INPUT_SIZE`].
///
/// # Examples
///
/// ```
/// use dippin_parser::parse;
/// let wf = parse("workflow F\n  start: A\n  exit: A\nagent A\n  prompt: x\n  model: m\n  provider: p\n", "t.dip").unwrap();
/// assert_eq!(wf.name, "F");
/// ```
pub fn parse(...) -> Result<Workflow> { ... }
```

Repeat for `parse_to_dot` and `parse_to_dot_with_options`.

**Step 2: Test doc examples**

```bash
cargo test -p dippin-parser --doc
```

**Step 3: Commit**

```bash
git add dippin-parser/src/lib.rs
git commit -m "docs(api): full rustdoc on parse/parse_to_dot entry points"
```

---

### Task 17: Document every public IR field

**Files:**
- Modify: `dippin-parser/src/ir.rs`

**Step 1: Add `///` to every field**

Walk through `Workflow`, `WorkflowDefaults`, `Node`, `Edge`, `AgentConfig`, `HumanConfig`, `ToolConfig`, `ParallelConfig`, `BranchConfig`, `FanInConfig`, `SubgraphConfig`, `RetryConfig`, `NodeIO`, `Condition`, `StylesheetRule`. For each field, add a one-line doc explaining what it stores and what units/format if relevant.

Examples to disambiguate:
- `Workflow.start` → `/// ID of the entry node (must reference a declared Node).`
- `Workflow.exit` → `/// ID of the terminal node (must reference a declared Node).`
- `WorkflowDefaults.fidelity` → `/// Default information-fidelity setting (e.g., "summary:medium").`
- `Edge.weight` → `/// Layout hint for Graphviz; higher values pull endpoints closer.`
- `Edge.restart` → `/// If true, this edge restarts the workflow from the source node.`
- `RetryConfig.policy` → `/// Retry policy name (e.g., "exponential", "fixed", "none").`
- `AgentConfig.compaction_threshold` → `/// Token-count fraction at which context compaction triggers (0.0–1.0).`

**Step 2: Test**

```bash
cargo doc -p dippin-parser --no-deps 2>&1 | tail -20
```

**Step 3: Commit**

```bash
git add dippin-parser/src/ir.rs
git commit -m "docs(ir): document every public IR field"
```

---

### Task 18: Add grammar reference at top of `parser.rs`

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Add a module-level `//!` doc with informal EBNF**

```rust
//! # Dippin parser
//!
//! Recursive-descent parser for the Dippin DSL. The grammar is roughly:
//!
//! ```text
//! file        := top_level*
//! top_level   := workflow_decl | node_decl | edges_block | stylesheet_block
//! workflow    := "workflow" IDENT NEWLINE INDENT workflow_field* OUTDENT
//! workflow_field := "goal:" STRING
//!                | "start:" IDENT
//!                | "exit:" IDENT
//!                | "version:" STRING
//!                | "defaults" NEWLINE INDENT default_field* OUTDENT
//! node_decl   := node_kind IDENT NEWLINE INDENT node_field* OUTDENT
//! node_kind   := "agent" | "human" | "tool" | "parallel" | "fan_in" | "subgraph"
//! edges_block := "edges" NEWLINE INDENT edge_decl* OUTDENT
//! edge_decl   := IDENT "->" IDENT edge_attr*
//! edge_attr   := "when" CONDITION | "weight:" INT | "label:" STRING | "restart"
//! ```
//!
//! Indentation is significant; tabs and spaces must not be mixed within a file.
//! See `/Users/dylanr/work/2389/dippin-lang/parser/` for the canonical Go implementation.
```

**Step 2: Commit**

```bash
git add dippin-parser/src/parser.rs
git commit -m "docs(parser): add grammar reference at module top"
```

---

### Task 19: Document the indent/outdent algorithm in `lexer.rs`

**Files:**
- Modify: `dippin-parser/src/lexer.rs`

**Step 1: Add a doc block above `Lexer` struct**

```rust
//! # Dippin lexer
//!
//! Indentation-aware tokenizer modeled on Python's INDENT/DEDENT scheme.
//! `indent_stack` holds the stack of currently-active indent columns; line 0
//! is sentinel value `0`. On each line:
//!
//! 1. Compute the leading whitespace count (spaces only — see `check_indent_consistency`).
//! 2. If `indent > top`, push `indent` and emit `INDENT`.
//! 3. While `indent < top`, pop and emit `OUTDENT`.
//! 4. If after popping `indent != top`, emit an `InvalidIndentation` diagnostic.
//!
//! Raw blocks (multi-line `prompt:` / `tool_command:` values) are extracted
//! by `extract_raw_block` which preserves the original text minus the common
//! indent prefix.
```

**Step 2: Comment `node_shape()` in `export_dot.rs`**

```rust
/// Map each NodeKind to a Graphviz shape. The mapping mirrors the Go reference
/// in `dippin-lang/export/dot.go`. Start nodes override to `Mdiamond` and
/// exit nodes to `Msquare` regardless of their underlying kind.
fn node_shape(kind: &NodeKind) -> &'static str { ... }
```

**Step 3: Commit**

```bash
git add dippin-parser/src/lexer.rs dippin-parser/src/export_dot.rs
git commit -m "docs(lexer,export): explain indent algorithm and shape mapping"
```

---

### Task 20: Create `dippin-parser/README.md`

**Files:**
- Create: `dippin-parser/README.md`

**Step 1: Write the README**

```markdown
# dippin-parser

A Rust parser and DOT exporter for the [Dippin DSL](https://github.com/2389-research/dippin-lang),
a higher-level authoring format for AI agent workflows.

## Status

Pre-1.0. Public types are `#[non_exhaustive]` to allow additive evolution.

## Usage

```rust
use dippin_parser::{parse, parse_to_dot, ExportOptions, RankDir};

let src = std::fs::read_to_string("workflow.dip")?;
let wf = parse(&src, "workflow.dip")?;

let dot = parse_to_dot_with_options(
    &src,
    "workflow.dip",
    &ExportOptions {
        include_prompts: true,
        rank_dir: RankDir::LeftRight,
        ..Default::default()
    },
)?;
println!("{dot}");
```

## Features

- `serde` — derives `Serialize`/`Deserialize` on every public IR type.

## Relationship to dippin-lang

This crate tracks the upstream Go implementation at
[`2389-research/dippin-lang`](https://github.com/2389-research/dippin-lang).
Behavioral parity is maintained through ported test fixtures in `testdata/`.

## License

MIT OR Apache-2.0
```

**Step 2: Commit**

```bash
git add dippin-parser/README.md
git commit -m "docs(parser): add crate README"
```

---

### Task 21: Create `examples/parse_and_export.rs`

**Files:**
- Create: `dippin-parser/examples/parse_and_export.rs`

**Step 1: Write the example**

```rust
// ABOUTME: Library usage example: parse a .dip file and emit DOT.
// ABOUTME: Run with `cargo run -p dippin-parser --example parse_and_export -- <path>`.

use dippin_parser::{parse_to_dot_with_options, ExportOptions, RankDir};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("usage: parse_and_export <path.dip>");
            return ExitCode::from(64);
        }
    };

    let src = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading {}: {}", path, e);
            return ExitCode::from(66);
        }
    };

    let opts = ExportOptions {
        include_prompts: true,
        rank_dir: RankDir::TopBottom,
        ..Default::default()
    };

    match parse_to_dot_with_options(&src, &path, &opts) {
        Ok(dot) => {
            println!("{dot}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            for d in e.diagnostics() {
                eprintln!("{}", d.render());
            }
            ExitCode::from(65)
        }
    }
}
```

**Step 2: Verify**

```bash
cargo build -p dippin-parser --example parse_and_export
cargo run -p dippin-parser --example parse_and_export -- dippin-parser/testdata/valid_minimal.dip
```

**Step 3: Commit**

```bash
git add dippin-parser/examples/parse_and_export.rs
git commit -m "docs(example): library usage example for parse + DOT export"
```

---

### Task 22: Create `testdata/README.md` index

**Files:**
- Create: `dippin-parser/testdata/README.md`

**Step 1: Write the index**

```markdown
# dippin-parser test fixtures

These `.dip` files exercise the parser. Each is described below.

| File | Purpose |
|---|---|
| `valid_minimal.dip` | Smallest valid workflow with one human and one agent node. |
| `valid_minimal_v2.dip` | Variant of `valid_minimal.dip` testing alternate syntax. |
| `multi_provider.dip` | Multiple agents using different LLM providers. |
| `ask_and_execute.dip` | Complex workflow with parallel, fan_in, conditionals, restarts. |
| `ask_and_execute.dot` | Golden DOT output for `ask_and_execute.dip`. |
| `unicode.dip` | UTF-8 regression coverage (multi-byte chars in identifiers, prompts, labels). |

Files starting with `valid_` must parse without errors. The corresponding
integration tests live in `dippin-parser/tests/integration_tests.rs`.

Failure-case fixtures live alongside the test code in
`dippin-parser/tests/error_cases.rs` (see companion plan).
```

**Step 2: Commit**

```bash
git add dippin-parser/testdata/README.md
git commit -m "docs(testdata): add fixture index"
```

---

### Task 23: Write `docs/plans/dippin-support/` design doc on this branch

**Files:**
- Create: `docs/plans/dippin-support/README.md`

**Step 1: Note: this directory exists in the main branch but not the variant-rust-port worktree. Copy it over.**

```bash
ls /Users/dylanr/work/2389/dot-viewer/docs/plans/dippin-support/ 2>&1
```

If it exists in main, copy the relevant context files into this branch:
```bash
mkdir -p docs/plans/dippin-support
cp /Users/dylanr/work/2389/dot-viewer/docs/plans/dippin-support/*.md docs/plans/dippin-support/ 2>/dev/null || true
```

**Step 2: If it doesn't exist anywhere, create a minimal index**

```markdown
# Dippin support

Tracks the design and implementation of `.dip` (Dippin DSL) support
across the dot-viewer project surfaces (CLI, web, macOS app).

## Plans

- `2026-04-07-dippin-correctness.md` — typed errors, parser hardening, lexer fixes, Go parity
- `2026-04-07-dippin-api-polish.md` — API hardening, semver, docs (this plan)
- `2026-04-07-dippin-ux-and-tests.md` — CLI UX, test coverage, fixtures

## Reference

- Upstream Go implementation: `/Users/dylanr/work/2389/dippin-lang/`
- Rust crate: `dippin-parser/`
- CLI integration: `dot-viewer-cli/src/main.rs`
```

**Step 3: Commit**

```bash
git add docs/plans/dippin-support/
git commit -m "docs(plans): dippin-support index for variant-rust-port branch"
```

---

### Task 24: Update top-level README with `.dip` section

**Files:**
- Modify: `README.md`

**Step 1: Add a section**

After the existing CLI section, add:

```markdown
## Dippin (.dip) support

dot-viewer now reads [Dippin DSL](https://github.com/2389-research/dippin-lang)
files in addition to Graphviz DOT. `.dip` files are converted to DOT internally
before rendering — no extra flags required.

### CLI

```bash
dot-viewer workflow.dip
dot-viewer workflow.dip --engine dot
dot-viewer - --format dip < workflow.dip   # via stdin
```

The CLI auto-detects the file format from extension. Override with `--format dot|dip|auto`.

### Architecture

The `dippin-parser` crate handles lexing, parsing, and DOT export. See
`dippin-parser/README.md` for library usage.
```

Add `dippin-parser/` to the architecture diagram if one exists in the README.

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs(readme): document .dip / Dippin support"
```

---

## Final verification

### Task 25: Full test sweep + clippy + doc build

```bash
cargo test -p dippin-parser
cargo test -p dippin-parser --features serde
cargo test -p dippin-parser --doc
cargo clippy -p dippin-parser -- -D warnings
cargo doc -p dippin-parser --no-deps
```

Fix anything that fails. Commit fixes.

---

## Notes for the executing engineer

- Run AFTER `2026-04-07-dippin-correctness.md`. Some tasks here depend on `Error`, `Diagnostic`, `Duration`, and `validate` being in place.
- `#[non_exhaustive]` (Task 3) is a one-line-per-type change but may surface issues at construction sites — fix them with `..Default::default()`.
- The `serde` task (Task 12) is the largest single change. The simplest approach: derive on every type unconditionally, gated by `cfg_attr`. `Duration` newtype needs a manual `Serialize`/`Deserialize` because `std::time::Duration` lacks default serde.
- Documentation tasks are straightforward but tedious — consider doing them in one sitting to avoid context switches.
- The `superpowers:test-driven-development` skill applies to behavior changes (Tasks 4, 5, 6, 9, 12). Pure refactors and documentation tasks don't require new tests, but existing tests must still pass.
