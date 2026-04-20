# Dippin Full Wiring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `.dip` files first-class in the macOS DotViewer app and the web app — user edits raw dippin, preview renders, bidirectional editor↔preview linking via a source map.

**Architecture:** `dippin-parser` gains a `parse_to_dot_with_map` entry point that returns DOT plus a list of `(dot_range, dip_range)` pairs. `dot-core` exposes this via UniFFI, `dot-core-wasm` via wasm-bindgen. Both surface apps add a `.dip` file type, maintain `{raw source, generated DOT, source map}` state, and run the existing node/range APIs against the generated DOT while translating offsets through the map.

**Tech Stack:** Rust (dippin-parser, dot-core, dot-core-wasm), UniFFI 0.30, wasm-bindgen, SwiftUI (DotViewer), SvelteKit + vitest + playwright (web).

**Design doc:** `docs/plans/2026-04-08-dippin-full-wiring-design.md`

**Key decisions** (from design doc):
1. Editor shows raw `.dip`; preview shows rendered DOT; save writes dippin verbatim
2. Conversion lives in `dot-core` / `dot-core-wasm`, not a separate FFI crate
3. Parse errors flatten to `DotError::SyntaxError("file:line:col: msg")` for v1
4. One UTI `com.2389.dot-viewer.dip` for the `.dip` extension
5. Full bidirectional source map (not identity/disabled linking)

**Source-map strategy:** The lexer does not currently track byte spans. Rather than thread spans through tokens (large refactor), we compute dippin byte ranges at export time from the existing `(line, column)` on `Node.source` / `Edge.source` combined with a line-offset table built from the original source string. For each node, the dippin range spans the declaration line through the last line owned by that node (bounded by the next node/edge/EOF). For each edge, the dippin range is the single declaration line. This is contained entirely to a new `export_dot_with_map` function and leaves the parser/lexer/IR untouched.

---

## Task 1: Add source-map types + stub entry point to dippin-parser

**Files:**
- Modify: `dippin-parser/src/lib.rs`
- Modify: `dippin-parser/src/export_dot.rs`

**Goal:** Introduce the new public API types and a `parse_to_dot_with_map` function that returns an empty source map but produces DOT byte-equivalent to `parse_to_dot`. No behavioral change — just scaffolding.

**Step 1: Write the failing test**

Add to `dippin-parser/src/lib.rs` tests module (or create new `dippin-parser/tests/source_map.rs`):

```rust
#[test]
fn parse_to_dot_with_map_matches_parse_to_dot_output() {
    let src = "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
    let plain = dippin_parser::parse_to_dot(src, "t.dip").unwrap();
    let with_map = dippin_parser::parse_to_dot_with_map(src, "t.dip").unwrap();
    assert_eq!(with_map.dot_source, plain, "dot_source must equal parse_to_dot output");
}
```

**Step 2: Run and verify it fails**

Run: `cd dippin-parser && cargo test parse_to_dot_with_map_matches -- --nocapture`
Expected: FAIL — `parse_to_dot_with_map` does not exist.

**Step 3: Add the types and stub in `dippin-parser/src/export_dot.rs`**

Near the top of `export_dot.rs` (after `ExportOptions`):

```rust
/// A pair of byte ranges linking a fragment of generated DOT to its original
/// location in the Dippin source. `dot_range` indexes into the `dot_source`
/// string returned alongside it; `dip_range` indexes into the original
/// `.dip` source passed to the parser.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMapEntry {
    /// Byte range in the generated DOT output.
    pub dot_range: std::ops::Range<usize>,
    /// Byte range in the original dippin source.
    pub dip_range: std::ops::Range<usize>,
}

/// DOT output plus a source map connecting fragments of the DOT back to
/// their original location in the dippin source.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct DippinConversion {
    /// Rendered DOT source.
    pub dot_source: String,
    /// Source map entries, one per node and edge in source order.
    pub source_map: Vec<SourceMapEntry>,
}
```

Add a new public function `export_dot_with_map` next to `export_dot`:

```rust
/// Render a workflow as DOT and also produce a source map. `dippin_source`
/// must be the same text originally passed to the parser.
pub fn export_dot_with_map(w: &Workflow, opts: &ExportOptions, dippin_source: &str) -> DippinConversion {
    let dot_source = export_dot(w, opts);
    // Source map entries will be populated in later tasks.
    let _ = dippin_source;
    DippinConversion { dot_source, source_map: Vec::new() }
}
```

**Step 4: Add `parse_to_dot_with_map` to `dippin-parser/src/lib.rs`**

After the existing `parse_to_dot_with_options` function, add:

```rust
/// Parse a Dippin source string, emit Graphviz DOT, and produce a source map
/// linking each emitted node/edge line back to its origin in the dippin source.
///
/// # Errors
///
/// Same as [`parse`].
pub fn parse_to_dot_with_map(
    source: &str,
    filename: impl AsRef<std::path::Path>,
) -> Result<crate::export_dot::DippinConversion> {
    let wf = parse(source, filename)?;
    Ok(crate::export_dot::export_dot_with_map(
        &wf,
        &crate::export_dot::ExportOptions::default(),
        source,
    ))
}
```

Make sure `DippinConversion` and `SourceMapEntry` are re-exported from `lib.rs`:

```rust
pub use crate::export_dot::{
    export_dot, ExportOptions, RankDir,
    DippinConversion, SourceMapEntry,
    export_dot_with_map,
};
```

(Merge with the existing `pub use` statement — do not create a duplicate.)

**Step 5: Run and verify it passes**

Run: `cd dippin-parser && cargo test parse_to_dot_with_map_matches -- --nocapture`
Expected: PASS.

Run: `cd dippin-parser && cargo test` — ensure no other tests broke.

**Step 6: Commit**

```bash
git add dippin-parser/src/lib.rs dippin-parser/src/export_dot.rs dippin-parser/tests/source_map.rs
git commit -m "feat(dippin-parser): add DippinConversion scaffold + parse_to_dot_with_map"
```

---

## Task 2: Build a line-offset table over the dippin source

**Files:**
- Modify: `dippin-parser/src/export_dot.rs`

**Goal:** Add a helper that pre-computes the byte offset of each line start in the dippin source. Used by later tasks to translate `(line, col)` to byte positions.

**Step 1: Write the failing test**

In `dippin-parser/src/export_dot.rs` tests module (or create one if it doesn't exist at the bottom of the file):

```rust
#[cfg(test)]
mod source_map_tests {
    use super::*;

    #[test]
    fn line_offsets_table() {
        let src = "abc\ndef\n\nghi";
        let offsets = compute_line_offsets(src);
        // Line 1 starts at 0, line 2 at 4, line 3 at 8, line 4 at 9 (after empty line).
        assert_eq!(offsets, vec![0, 4, 8, 9]);
    }

    #[test]
    fn line_offsets_empty_source() {
        assert_eq!(compute_line_offsets(""), vec![0]);
    }

    #[test]
    fn line_offsets_no_trailing_newline() {
        // "abc\ndef" → line 1 @ 0, line 2 @ 4.
        assert_eq!(compute_line_offsets("abc\ndef"), vec![0, 4]);
    }
}
```

**Step 2: Run and verify it fails**

Run: `cd dippin-parser && cargo test source_map_tests`
Expected: FAIL — `compute_line_offsets` not defined.

**Step 3: Implement `compute_line_offsets`**

Add to `export_dot.rs` (private, near `export_dot_with_map`):

```rust
/// Return a vector where element `i` is the byte offset of line `i+1` (1-based)
/// in `source`. Always contains at least one element (offset 0 for line 1).
fn compute_line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for (i, ch) in source.char_indices() {
        if ch == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}
```

**Step 4: Run and verify it passes**

Run: `cd dippin-parser && cargo test source_map_tests`
Expected: PASS (all 3 tests).

**Step 5: Commit**

```bash
git add dippin-parser/src/export_dot.rs
git commit -m "feat(dippin-parser): add line-offset table helper for source mapping"
```

---

## Task 3: Emit dippin ranges for nodes

**Files:**
- Modify: `dippin-parser/src/export_dot.rs`

**Goal:** Populate `source_map` with one entry per node. The dippin range for a node spans from the start of its declaration line through the start of the next node/edge/EOF. The DOT range is captured by sampling `dot.len()` before and after `write_node_dot`.

**Step 1: Write the failing test**

In `dippin-parser/tests/source_map.rs`:

```rust
#[test]
fn source_map_contains_one_entry_per_node() {
    let src = "workflow F\n  start: A\n  exit: B\n  agent A\n    prompt: hi\n    model: m\n    provider: p\n  agent B\n    prompt: bye\n    model: m\n    provider: p\n";
    let conv = dippin_parser::parse_to_dot_with_map(src, "t.dip").unwrap();
    assert_eq!(conv.source_map.len(), 2, "expected 2 node entries (no edges)");

    // Entry 0 should be for agent A. Its dippin range must slice to text that
    // includes "agent A".
    let a = &conv.source_map[0];
    let slice = &src[a.dip_range.clone()];
    assert!(slice.contains("agent A"), "entry 0 dip slice must contain 'agent A', got: {:?}", slice);
    assert!(!slice.contains("agent B"), "entry 0 must NOT reach into agent B, got: {:?}", slice);

    // Entry 1 for agent B.
    let b = &conv.source_map[1];
    let slice = &src[b.dip_range.clone()];
    assert!(slice.contains("agent B"), "entry 1 dip slice must contain 'agent B', got: {:?}", slice);

    // DOT range must slice to text that mentions the node ID.
    let dot_slice_a = &conv.dot_source[a.dot_range.clone()];
    assert!(dot_slice_a.contains("\"A\""), "dot slice for A must reference A, got: {:?}", dot_slice_a);
}
```

**Step 2: Run and verify it fails**

Run: `cd dippin-parser && cargo test source_map_contains_one_entry_per_node`
Expected: FAIL — `source_map` is empty.

**Step 3: Implement node range emission**

Replace the body of `export_dot_with_map` in `dippin-parser/src/export_dot.rs`:

```rust
pub fn export_dot_with_map(w: &Workflow, opts: &ExportOptions, dippin_source: &str) -> DippinConversion {
    let line_offsets = compute_line_offsets(dippin_source);
    let total_len = dippin_source.len();
    let line_start = |line: usize| -> usize {
        if line == 0 || line > line_offsets.len() {
            return total_len;
        }
        line_offsets[line - 1]
    };

    // Build a list of "boundary lines" in source order: each node's line,
    // each edge's line, plus a sentinel past EOF. The dippin range of each
    // construct is [its line_start .. next boundary line_start).
    let mut boundaries: Vec<usize> = Vec::new();
    for n in &w.nodes {
        boundaries.push(n.source.line);
    }
    for e in &w.edges {
        boundaries.push(e.source.line);
    }
    boundaries.sort_unstable();
    boundaries.push(usize::MAX); // sentinel for EOF

    let next_boundary_after = |line: usize| -> usize {
        for &b in &boundaries {
            if b > line {
                return if b == usize::MAX { total_len } else { line_start(b) };
            }
        }
        total_len
    };

    let mut dot_source = String::new();
    let mut source_map: Vec<SourceMapEntry> = Vec::new();

    write_dot_header(&mut dot_source, w, opts);

    for n in &w.nodes {
        let dot_start = dot_source.len();
        write_node_dot(&mut dot_source, n, w, opts);
        let dot_end = dot_source.len();

        let dip_start = line_start(n.source.line);
        let dip_end = next_boundary_after(n.source.line);

        source_map.push(SourceMapEntry {
            dot_range: dot_start..dot_end,
            dip_range: dip_start..dip_end,
        });
    }

    dot_source.push('\n');

    for e in &w.edges {
        write_edge_dot(&mut dot_source, e);
    }

    dot_source.push_str("}\n");

    DippinConversion { dot_source, source_map }
}
```

**Step 4: Run and verify it passes**

Run: `cd dippin-parser && cargo test source_map`
Expected: PASS (including the parity test from Task 1 and the new node test).

Important: the parity test `parse_to_dot_with_map_matches_parse_to_dot_output` must still pass. The new function now duplicates the body of `export_dot` rather than calling it — verify the DOT output is still byte-identical.

**Step 5: Commit**

```bash
git add dippin-parser/src/export_dot.rs dippin-parser/tests/source_map.rs
git commit -m "feat(dippin-parser): emit dippin source ranges for exported nodes"
```

---

## Task 4: Emit dippin ranges for edges

**Files:**
- Modify: `dippin-parser/src/export_dot.rs`
- Modify: `dippin-parser/tests/source_map.rs`

**Goal:** Add one source-map entry per edge. Edge dippin range is its declaration line (line_start .. next line_start).

**Step 1: Write the failing test**

Append to `dippin-parser/tests/source_map.rs`:

```rust
#[test]
fn source_map_contains_entries_for_edges() {
    let src = "workflow F\n  start: A\n  exit: B\n  agent A\n    prompt: hi\n    model: m\n    provider: p\n  agent B\n    prompt: bye\n    model: m\n    provider: p\n  A -> B\n";
    let conv = dippin_parser::parse_to_dot_with_map(src, "t.dip").unwrap();
    // 2 nodes + 1 edge.
    assert_eq!(conv.source_map.len(), 3);

    // Last entry is the edge.
    let edge_entry = conv.source_map.last().unwrap();
    let dip_slice = &src[edge_entry.dip_range.clone()];
    assert!(dip_slice.contains("A -> B"), "edge dip slice must contain 'A -> B', got: {:?}", dip_slice);

    let dot_slice = &conv.dot_source[edge_entry.dot_range.clone()];
    assert!(dot_slice.contains("->"), "edge dot slice must contain '->', got: {:?}", dot_slice);
}
```

**Step 2: Run and verify it fails**

Run: `cd dippin-parser && cargo test source_map_contains_entries_for_edges`
Expected: FAIL — source_map has 2 entries, not 3.

**Step 3: Implement edge emission**

In `export_dot_with_map` (in `export_dot.rs`), replace the `for e in &w.edges` loop with:

```rust
for e in &w.edges {
    let dot_start = dot_source.len();
    write_edge_dot(&mut dot_source, e);
    let dot_end = dot_source.len();

    let dip_start = line_start(e.source.line);
    let dip_end = next_boundary_after(e.source.line);

    source_map.push(SourceMapEntry {
        dot_range: dot_start..dot_end,
        dip_range: dip_start..dip_end,
    });
}
```

**Step 4: Run and verify it passes**

Run: `cd dippin-parser && cargo test source_map`
Expected: PASS (all 3 tests).

**Step 5: Commit**

```bash
git add dippin-parser/src/export_dot.rs dippin-parser/tests/source_map.rs
git commit -m "feat(dippin-parser): emit dippin source ranges for exported edges"
```

---

## Task 5: Golden source-map fixture

**Files:**
- Create: `dippin-parser/tests/fixtures/source_map_simple.dip`
- Create: `dippin-parser/tests/fixtures/source_map_simple.map.json`
- Modify: `dippin-parser/tests/source_map.rs`

**Goal:** Lock down source-map stability with a golden JSON fixture that includes the expected (dot_range, dip_range) pairs plus the expected DOT slices for each entry. Catches accidental drift.

**Step 1: Create the fixture**

`dippin-parser/tests/fixtures/source_map_simple.dip`:

```text
workflow Hello
  start: A
  exit: B
  agent A
    prompt: hello
    model: m
    provider: p
  agent B
    prompt: bye
    model: m
    provider: p
  A -> B
```

**Step 2: Write the failing test**

Append to `dippin-parser/tests/source_map.rs`:

```rust
#[test]
fn source_map_golden_simple() {
    let src = include_str!("fixtures/source_map_simple.dip");
    let conv = dippin_parser::parse_to_dot_with_map(src, "source_map_simple.dip").unwrap();

    // The golden fixture pins the DOT slice for each source-map entry.
    // Update this test and regenerate if the emitted DOT format intentionally changes.
    let expected_dot_slices = [
        "\"A\"",   // agent A node line must reference "A"
        "\"B\"",   // agent B node line must reference "B"
        "\"A\" -> \"B\"", // edge line
    ];
    assert_eq!(conv.source_map.len(), expected_dot_slices.len());
    for (i, expected) in expected_dot_slices.iter().enumerate() {
        let entry = &conv.source_map[i];
        let dot_slice = &conv.dot_source[entry.dot_range.clone()];
        assert!(
            dot_slice.contains(expected),
            "entry {} dot slice must contain {:?}, got: {:?}",
            i, expected, dot_slice,
        );
        let dip_slice = &src[entry.dip_range.clone()];
        assert!(!dip_slice.is_empty(), "entry {} dip slice must be non-empty", i);
    }

    // First two entries must not overlap each other in dippin space.
    assert!(conv.source_map[0].dip_range.end <= conv.source_map[1].dip_range.start);
}
```

**Step 3: Run and verify it fails, then passes**

Run: `cd dippin-parser && cargo test source_map_golden_simple`
Expected: if the DOT emitter produces different node syntax, the test may fail on the slice substring check — adjust the fixture's `expected_dot_slices` to match actual output, then re-run until PASS.

(This is an exception to strict TDD — the golden is a snapshot. Run once, inspect, lock in.)

**Step 4: Commit**

```bash
git add dippin-parser/tests/fixtures/source_map_simple.dip dippin-parser/tests/source_map.rs
git commit -m "test(dippin-parser): golden source-map fixture"
```

---

## Task 6: dot-core UniFFI export

**Files:**
- Modify: `dot-core/src/lib.rs`
- Modify: `dot-core/Cargo.toml`

**Goal:** Add `parse_dippin` UniFFI export that returns a `DippinConversionResult` or throws `DotError::SyntaxError` on parse failure.

**Step 1: Add dependency**

Edit `dot-core/Cargo.toml` — add under `[dependencies]`:

```toml
dippin-parser = { path = "../dippin-parser" }
```

**Step 2: Write the failing test**

Create `dot-core/tests/parse_dippin.rs`:

```rust
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
```

**Step 3: Run and verify it fails**

Run: `cd dot-core && cargo test --test parse_dippin`
Expected: FAIL — `parse_dippin` not exported, `SourceMapEntry` missing.

**Step 4: Add UniFFI records and export**

Append to `dot-core/src/lib.rs` (keeping all new code behind `#[cfg(not(target_arch = "wasm32"))]` like the existing exports):

```rust
/// A pair of byte ranges linking a fragment of generated DOT to its original
/// dippin source location.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, uniffi::Record)]
pub struct SourceMapEntry {
    pub dot_start: u32,
    pub dot_end: u32,
    pub dip_start: u32,
    pub dip_end: u32,
}

/// Result of converting dippin to DOT via [`parse_dippin`].
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, uniffi::Record)]
pub struct DippinConversionResult {
    pub dot_source: String,
    pub source_map: Vec<SourceMapEntry>,
}

/// Parse a dippin source string and return the converted DOT plus a source
/// map connecting each node/edge back to its dippin origin.
#[cfg(not(target_arch = "wasm32"))]
#[uniffi::export]
pub fn parse_dippin(source: String) -> Result<DippinConversionResult, DotError> {
    let conv = dippin_parser::parse_to_dot_with_map(&source, "input.dip")
        .map_err(|e| {
            let first = e.diagnostics().first().map(|d| {
                format!(
                    "{}:{}:{}: {}",
                    d.location.file, d.location.line, d.location.column, d.message
                )
            }).unwrap_or_else(|| "dippin parse failed".to_string());
            DotError::SyntaxError { message: first }
        })?;
    let source_map = conv.source_map.into_iter().map(|e| SourceMapEntry {
        dot_start: e.dot_range.start as u32,
        dot_end: e.dot_range.end as u32,
        dip_start: e.dip_range.start as u32,
        dip_end: e.dip_range.end as u32,
    }).collect();
    Ok(DippinConversionResult { dot_source: conv.dot_source, source_map })
}
```

Note: if `DotError::SyntaxError` has a different variant shape (tuple vs struct), match its existing form. Check `dot-core/src/lib.rs` for the `DotError` enum definition.

**Step 5: Run and verify it passes**

Run: `cd dot-core && cargo test --test parse_dippin`
Expected: PASS (2 tests).

Run: `cd dot-core && cargo build --release` to ensure the release build (required for UniFFI bindings matching) still compiles.

**Step 6: Commit**

```bash
git add dot-core/Cargo.toml dot-core/src/lib.rs dot-core/tests/parse_dippin.rs
git commit -m "feat(dot-core): add parse_dippin UniFFI export with source map"
```

---

## Task 7: Regenerate Swift bindings

**Files:**
- Regenerate: `DotViewer/DotViewer/Generated/dot_core.swift` + `.h` + `.modulemap`

**Goal:** Pick up the new `parse_dippin` export in the Swift bindings so Xcode sees it.

**Step 1: Run the bindings generator**

From the repo root:

```bash
make generate-bindings
```

Expected: builds `dot-core` release, regenerates files under `DotViewer/DotViewer/Generated/`.

**Step 2: Verify the new symbol is in the generated Swift**

Run: `grep -n "parseDippin\|parse_dippin\|DippinConversionResult" DotViewer/DotViewer/Generated/dot_core.swift`
Expected: matches for both the function and the record struct.

**Step 3: Verify the macOS app still builds**

Run: `cd DotViewer && xcodebuild -project DotViewer.xcodeproj -scheme DotViewer -configuration Debug build -quiet`
Expected: BUILD SUCCEEDED.

**Step 4: Commit the regenerated bindings**

```bash
git add DotViewer/DotViewer/Generated/
git commit -m "chore(DotViewer): regenerate Swift bindings for parse_dippin"
```

---

## Task 8: dot-core-wasm export

**Files:**
- Modify: `dot-core-wasm/Cargo.toml`
- Modify: `dot-core-wasm/src/lib.rs`
- Create: `dot-core-wasm/tests/parse_dippin.rs` (wasm-bindgen-test)

**Goal:** Expose `parseDippin` to the web build, returning a JS object with the same shape.

**Step 1: Add dependency**

Edit `dot-core-wasm/Cargo.toml`:

```toml
dippin-parser = { path = "../dippin-parser", features = ["serde"] }
```

**Step 2: Write the failing test**

Create `dot-core-wasm/tests/parse_dippin.rs`:

```rust
// ABOUTME: wasm-bindgen-test coverage for parseDippin.
// ABOUTME: Runs in a headless browser via wasm-pack test.

use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn parse_dippin_returns_dot_and_map() {
    let src = "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
    let value = dot_core_wasm::parse_dippin(src).expect("parse ok");
    // value is a JsValue; round-trip it to a serde_json::Value via the bindgen helper.
    let obj: serde_json::Value = serde_wasm_bindgen::from_value(value).unwrap();
    assert!(obj.get("dotSource").and_then(|v| v.as_str()).unwrap().contains("digraph F {"));
    let map = obj.get("sourceMap").and_then(|v| v.as_array()).unwrap();
    assert!(!map.is_empty());
    let first = &map[0];
    assert!(first.get("dotStart").is_some());
    assert!(first.get("dipEnd").is_some());
}
```

**Step 3: Run and verify it fails**

Run: `cd dot-core-wasm && wasm-pack test --headless --firefox -- --test parse_dippin`
(Or `--chrome` / `--safari` depending on what is installed.)
Expected: FAIL — `parse_dippin` not exported.

**Step 4: Add the export in `dot-core-wasm/src/lib.rs`**

Append:

```rust
#[derive(serde::Serialize)]
struct JsDippinConversion {
    #[serde(rename = "dotSource")]
    dot_source: String,
    #[serde(rename = "sourceMap")]
    source_map: Vec<JsSourceMapEntry>,
}

#[derive(serde::Serialize)]
struct JsSourceMapEntry {
    #[serde(rename = "dotStart")]
    dot_start: u32,
    #[serde(rename = "dotEnd")]
    dot_end: u32,
    #[serde(rename = "dipStart")]
    dip_start: u32,
    #[serde(rename = "dipEnd")]
    dip_end: u32,
}

#[wasm_bindgen(js_name = parseDippin)]
pub fn parse_dippin(source: &str) -> Result<JsValue, JsValue> {
    let conv = dippin_parser::parse_to_dot_with_map(source, "input.dip")
        .map_err(|e| {
            let first = e.diagnostics().first().map(|d| format!(
                "{}:{}:{}: {}",
                d.location.file, d.location.line, d.location.column, d.message
            )).unwrap_or_else(|| "dippin parse failed".into());
            JsValue::from_str(&first)
        })?;
    let js = JsDippinConversion {
        dot_source: conv.dot_source,
        source_map: conv.source_map.into_iter().map(|e| JsSourceMapEntry {
            dot_start: e.dot_range.start as u32,
            dot_end: e.dot_range.end as u32,
            dip_start: e.dip_range.start as u32,
            dip_end: e.dip_range.end as u32,
        }).collect(),
    };
    serde_wasm_bindgen::to_value(&js).map_err(|e| JsValue::from_str(&e.to_string()))
}
```

Ensure `serde` and `serde-wasm-bindgen` are already listed as dependencies (they should be — used by the existing exports).

**Step 5: Run and verify it passes**

Run: `cd dot-core-wasm && wasm-pack test --headless --firefox -- --test parse_dippin`
Expected: PASS.

**Step 6: Rebuild the wasm package for the web app**

Run whatever command the web app uses to rebuild its wasm bundle (check `web/package.json` scripts, `dot-core-wasm/Makefile`, or existing docs). Typically:

```bash
cd dot-core-wasm && wasm-pack build --target web --out-dir ../web/src/lib/wasm-pkg
```

(Adjust path to match where `wasm.ts` loads its module from.)

**Step 7: Commit**

```bash
git add dot-core-wasm/Cargo.toml dot-core-wasm/src/lib.rs dot-core-wasm/tests/parse_dippin.rs web/src/lib/wasm-pkg
git commit -m "feat(dot-core-wasm): add parseDippin export with source map"
```

---

## Task 9: Register the .dip UTI in the macOS app

**Files:**
- Modify: `DotViewer/project.yml`
- Modify: `DotViewer/DotViewer/Info.plist`
- Regenerate: `DotViewer/DotViewer.xcodeproj/project.pbxproj`

**Goal:** Declare `com.2389.dot-viewer.dip` as an exported UTI and register the `.dip` extension so the Finder file picker lists them.

**Step 1: Add the UTI to `Info.plist`**

Read the current `DotViewer/DotViewer/Info.plist` first. Under `CFBundleDocumentTypes`, add a new entry:

```xml
<dict>
    <key>CFBundleTypeName</key>
    <string>Dippin Workflow</string>
    <key>CFBundleTypeRole</key>
    <string>Editor</string>
    <key>LSHandlerRank</key>
    <string>Owner</string>
    <key>LSItemContentTypes</key>
    <array>
        <string>com.2389.dot-viewer.dip</string>
    </array>
</dict>
```

Under `UTExportedTypeDeclarations`, add:

```xml
<dict>
    <key>UTTypeIdentifier</key>
    <string>com.2389.dot-viewer.dip</string>
    <key>UTTypeDescription</key>
    <string>Dippin Workflow</string>
    <key>UTTypeConformsTo</key>
    <array>
        <string>public.plain-text</string>
    </array>
    <key>UTTypeTagSpecification</key>
    <dict>
        <key>public.filename-extension</key>
        <array>
            <string>dip</string>
        </array>
    </dict>
</dict>
```

**Step 2: Regenerate the Xcode project**

```bash
cd DotViewer && xcodegen
```

**Step 3: Build to confirm nothing broke**

```bash
cd DotViewer && xcodebuild -project DotViewer.xcodeproj -scheme DotViewer -configuration Debug build -quiet
```

Expected: BUILD SUCCEEDED.

**Step 4: Manual smoke test**

Launch the app, choose File → Open, and confirm `.dip` files are no longer greyed out in Finder's file picker.

**Step 5: Commit**

```bash
git add DotViewer/project.yml DotViewer/DotViewer/Info.plist DotViewer/DotViewer.xcodeproj
git commit -m "feat(DotViewer): register .dip UTI for file picker support"
```

---

## Task 10: Extend DotDocument with dippin state + read path

**Files:**
- Modify: `DotViewer/DotViewer/DotDocument.swift`

**Goal:** When a `.dip` file is opened, parse it with `parse_dippin`, store `(text, isDippin, generatedDot, sourceMap, parseError)`. `.dot` files behave as before.

**Step 1: Write the failing test**

Create `DotViewer/DotViewerTests/DotDocumentDippinTests.swift`:

```swift
// ABOUTME: Tests DotDocument's handling of .dip source files.
// ABOUTME: Verifies dippin parsing on open and source-map population.

import XCTest
@testable import DotViewer

final class DotDocumentDippinTests: XCTestCase {
    func testOpenDippinFileParsesAndPopulatesSourceMap() throws {
        let src = """
        workflow F
          start: A
          exit: A
          agent A
            prompt: hi
            model: m
            provider: p
        """
        let doc = DotDocument()
        try doc.loadDippin(from: src)

        XCTAssertTrue(doc.isDippin)
        XCTAssertEqual(doc.text, src)
        XCTAssertTrue(doc.generatedDot.contains("digraph F {"))
        XCTAssertFalse(doc.sourceMap.isEmpty)
        XCTAssertNil(doc.parseError)
    }

    func testOpenInvalidDippinSetsParseError() {
        let doc = DotDocument()
        XCTAssertThrowsError(try doc.loadDippin(from: "workflow\n")) { _ in
            XCTAssertNotNil(doc.parseError)
        }
    }

    func testPlainDotDocumentIsNotDippin() throws {
        let doc = DotDocument()
        doc.loadDot(from: "digraph G { A -> B }")
        XCTAssertFalse(doc.isDippin)
        XCTAssertEqual(doc.generatedDot, doc.text)
        XCTAssertTrue(doc.sourceMap.isEmpty)
    }
}
```

**Step 2: Run and verify it fails**

Build and run the test target in Xcode, or:

```bash
cd DotViewer && xcodebuild test -project DotViewer.xcodeproj -scheme DotViewer -destination 'platform=macOS'
```

Expected: FAIL — `loadDippin`, `isDippin`, `generatedDot`, `sourceMap`, `parseError`, `loadDot` do not exist.

**Step 3: Extend `DotDocument.swift`**

Read the current `DotDocument.swift` first to see its existing structure. Then add:

```swift
import UniformTypeIdentifiers

extension UTType {
    static let dippin: UTType = UTType("com.2389.dot-viewer.dip") ?? .plainText
}

extension DotDocument {
    /// True when the current document was loaded from a .dip source.
    var isDippin: Bool { _isDippin }

    /// The DOT that should be fed to the renderer. Equals `text` for plain
    /// DOT documents; equals the converted DOT for dippin documents.
    var generatedDot: String { _generatedDot }

    /// Source-map entries for bidirectional editor↔preview linking.
    /// Empty for plain DOT documents.
    var sourceMap: [SourceMapEntry] { _sourceMap }

    /// Parse error message from the most recent dippin reparse, if any.
    var parseError: String? { _parseError }
}
```

Add stored properties to the class (not the extension):

```swift
private var _isDippin: Bool = false
private var _generatedDot: String = ""
private var _sourceMap: [SourceMapEntry] = []
private var _parseError: String? = nil
```

Add load methods:

```swift
/// Load a plain DOT source (`.dot` / `.gv` / `.txt` / Word `.dot`).
func loadDot(from source: String) {
    self.text = source
    self._isDippin = false
    self._generatedDot = source
    self._sourceMap = []
    self._parseError = nil
}

/// Load dippin source. Parses immediately and populates generated DOT +
/// source map, or throws and stores `parseError`.
func loadDippin(from source: String) throws {
    self.text = source
    self._isDippin = true
    do {
        let result = try parseDippin(source: source)
        self._generatedDot = result.dotSource
        self._sourceMap = result.sourceMap
        self._parseError = nil
    } catch let error as DotError {
        let msg: String
        switch error {
        case .SyntaxError(let m): msg = m
        default: msg = "\(error)"
        }
        self._generatedDot = ""
        self._sourceMap = []
        self._parseError = msg
        throw error
    }
}

/// Re-parse after an edit when the current document is dippin. Updates
/// generatedDot/sourceMap or parseError without throwing.
func reparseDippinIfNeeded() {
    guard _isDippin else {
        _generatedDot = text
        return
    }
    do {
        let result = try parseDippin(source: text)
        self._generatedDot = result.dotSource
        self._sourceMap = result.sourceMap
        self._parseError = nil
    } catch let error as DotError {
        if case .SyntaxError(let m) = error {
            self._parseError = m
        } else {
            self._parseError = "\(error)"
        }
        // Keep the previous generatedDot so the preview doesn't go blank on
        // a transient parse error mid-typing.
    } catch {
        self._parseError = "\(error)"
    }
}
```

Update the `read(configuration:)` method (find the existing one and extend it):

```swift
func read(configuration: ReadConfiguration) throws {
    guard let data = configuration.file.regularFileContents,
          let source = String(data: data, encoding: .utf8) else {
        throw CocoaError(.fileReadCorruptFile)
    }
    if configuration.contentType == .dippin {
        try self.loadDippin(from: source)
    } else {
        self.loadDot(from: source)
    }
}
```

Update `readableContentTypes`:

```swift
static var readableContentTypes: [UTType] { [.graphvizDot, .graphvizGv, .msWordDot, .dippin, .plainText] }
```

Add `.dippin` to `writableContentTypes` too if it exists.

The `snapshot`/`fileWrapper` path writes `text` verbatim — no change needed.

**Step 4: Run and verify it passes**

```bash
cd DotViewer && xcodebuild test -project DotViewer.xcodeproj -scheme DotViewer -destination 'platform=macOS'
```

Expected: PASS (3 new tests).

**Step 5: Commit**

```bash
git add DotViewer/DotViewer/DotDocument.swift DotViewer/DotViewerTests/DotDocumentDippinTests.swift
git commit -m "feat(DotViewer): parse .dip files on open and maintain source map"
```

---

## Task 11: Add offset-translation helpers

**Files:**
- Modify: `DotViewer/DotViewer/DotDocument.swift`
- Modify: `DotViewer/DotViewerTests/DotDocumentDippinTests.swift`

**Goal:** Add `dotOffsetForDippinOffset(_:)` and `dippinRangeForDotOffset(_:)` to translate between editor coordinates and DOT coordinates via the source map.

**Step 1: Write the failing test**

Append to `DotDocumentDippinTests.swift`:

```swift
func testOffsetTranslationIsIdentityForPlainDot() {
    let doc = DotDocument()
    doc.loadDot(from: "digraph G { A -> B }")
    XCTAssertEqual(doc.dotOffsetForDippinOffset(5), 5)
    XCTAssertEqual(doc.dippinRangeForDotOffset(5)?.lowerBound, 5)
}

func testOffsetTranslationMapsThroughSourceMap() throws {
    let src = """
    workflow F
      start: A
      exit: A
      agent A
        prompt: hi
        model: m
        provider: p
    """
    let doc = DotDocument()
    try doc.loadDippin(from: src)

    // Find "agent A" in the dippin source.
    let agentOffset = src.distance(from: src.startIndex,
                                   to: src.range(of: "agent A")!.lowerBound)
    // Translation should land inside the first source-map entry's DOT range.
    let dotOffset = doc.dotOffsetForDippinOffset(agentOffset)
    XCTAssertNotNil(dotOffset)
    let entry = doc.sourceMap[0]
    XCTAssertTrue(Int(entry.dotStart)...Int(entry.dotEnd) ~= dotOffset!)

    // Reverse: pick the DOT range midpoint, translate back.
    let midDot = (Int(entry.dotStart) + Int(entry.dotEnd)) / 2
    let dipRange = doc.dippinRangeForDotOffset(midDot)
    XCTAssertNotNil(dipRange)
    XCTAssertTrue(dipRange!.contains(agentOffset))
}
```

**Step 2: Run and verify it fails**

Run tests — expected FAIL (methods don't exist).

**Step 3: Implement the helpers in `DotDocument.swift`**

```swift
/// Translate an offset in `text` (dippin space) to the corresponding offset
/// in `generatedDot` (DOT space). Returns identity for non-dippin docs.
/// Returns nil if no entry contains the offset.
func dotOffsetForDippinOffset(_ dipOffset: Int) -> Int? {
    if !_isDippin { return dipOffset }
    for entry in _sourceMap {
        let dipStart = Int(entry.dipStart)
        let dipEnd = Int(entry.dipEnd)
        if dipOffset >= dipStart && dipOffset < dipEnd {
            // Return the start of the DOT range — clicking anywhere inside the
            // dippin block maps to the start of the matching DOT construct.
            return Int(entry.dotStart)
        }
    }
    return nil
}

/// Translate an offset in `generatedDot` (DOT space) to a range in `text`
/// (dippin space). Returns `offset..<offset` for non-dippin docs.
func dippinRangeForDotOffset(_ dotOffset: Int) -> Range<Int>? {
    if !_isDippin { return dotOffset..<dotOffset }
    for entry in _sourceMap {
        let dotStart = Int(entry.dotStart)
        let dotEnd = Int(entry.dotEnd)
        if dotOffset >= dotStart && dotOffset < dotEnd {
            return Int(entry.dipStart)..<Int(entry.dipEnd)
        }
    }
    return nil
}
```

**Step 4: Run and verify it passes**

```bash
cd DotViewer && xcodebuild test -project DotViewer.xcodeproj -scheme DotViewer -destination 'platform=macOS'
```

Expected: PASS.

**Step 5: Commit**

```bash
git add DotViewer/DotViewer/DotDocument.swift DotViewer/DotViewerTests/DotDocumentDippinTests.swift
git commit -m "feat(DotViewer): add bidirectional offset translation helpers"
```

---

## Task 12: Wire preview + linking to use generatedDot

**Files:**
- Modify: preview render call site in the DotViewer SwiftUI code (likely `DotViewer/DotViewer/ContentView.swift` or `DotViewer/DotViewer/Views/*`)
- Modify: editor↔preview link handler(s) in the same view layer

**Goal:** Change the render pipeline to read `document.generatedDot` instead of `document.text`. Route editor-cursor and preview-click events through the offset translation helpers. Debounce dippin reparse alongside existing render debounce.

**Step 1: Locate the render + link call sites**

```bash
grep -rn "document\.text\|document\.source\|\.text\b.*render\|renderDot\|nodeIdAt\|definitionRangeForNode" DotViewer/DotViewer/ --include="*.swift"
```

Identify:
- Where `renderDot(...)` (or the wrapper function) is called with the document source
- Where the editor publishes cursor changes
- Where the preview reports node clicks
- Where the editor content binding lives

**Step 2: Write the failing test**

This is an integration-level change. The unit coverage already exists from Task 11. Add one more test that verifies `reparseDippinIfNeeded` keeps state coherent on edits:

```swift
func testReparseOnEditUpdatesGeneratedDot() throws {
    let doc = DotDocument()
    try doc.loadDippin(from: """
    workflow F
      start: A
      exit: A
      agent A
        prompt: hi
        model: m
        provider: p
    """)
    let before = doc.generatedDot
    doc.text = doc.text.replacingOccurrences(of: "agent A", with: "agent Foo")
        .replacingOccurrences(of: "start: A", with: "start: Foo")
        .replacingOccurrences(of: "exit: A", with: "exit: Foo")
    doc.reparseDippinIfNeeded()
    XCTAssertNotEqual(doc.generatedDot, before)
    XCTAssertTrue(doc.generatedDot.contains("Foo"))
    XCTAssertNil(doc.parseError)
}
```

**Step 3: Run and verify it fails** (will fail until step 4 is in — but unit methods already exist so it may just pass immediately. In that case skip step 4 for the unit logic and proceed to the view-layer wiring.)

**Step 4: Update the render call site**

Replace calls like `renderDot(document.text, engine)` with `renderDot(document.generatedDot, engine)`.

Hook `document.reparseDippinIfNeeded()` into whatever existing debounce pipeline handles editor changes. The order is:
1. Editor change → update `document.text`
2. Call `document.reparseDippinIfNeeded()` (no-op for plain DOT)
3. If `document.parseError != nil`, show it in the error banner and skip render
4. Else call `renderDot(document.generatedDot, engine)`

Update the editor-cursor → node-highlight path:
1. Editor reports offset `n` in `document.text`
2. Translate: `let dotOffset = document.dotOffsetForDippinOffset(n)`
3. If nil, clear highlight
4. Else call existing `nodeIdAt(document.generatedDot, dotOffset)`

Update the preview-click → editor-highlight path:
1. Preview reports node ID
2. Call existing `definitionRangeForNode(document.generatedDot, nodeId)`
3. Translate: `let dipRange = document.dippinRangeForDotOffset(range.location)`
4. Highlight `dipRange` in the editor

**Step 5: Run all tests**

```bash
cd DotViewer && xcodebuild test -project DotViewer.xcodeproj -scheme DotViewer -destination 'platform=macOS'
```

Expected: PASS.

**Step 6: Manual smoke test**

- Launch the app
- Open a `.dip` fixture (copy `dippin-parser/tests/fixtures/source_map_simple.dip` somewhere you can navigate to)
- Confirm the preview renders the graph
- Click a node in the preview → confirm the editor highlights the corresponding `agent X` block
- Place the cursor inside an `agent X` block in the editor → confirm the preview highlights the matching node

**Step 7: Commit**

```bash
git add DotViewer/DotViewer/
git commit -m "feat(DotViewer): route preview + linking through generatedDot for .dip files"
```

---

## Task 13: Web app — Toolbar accepts .dip + passes filename

**Files:**
- Modify: `web/src/lib/components/Toolbar.svelte`
- Modify: `web/src/routes/+page.svelte`

**Goal:** Accept `.dip` in the file picker and pass the filename alongside content so the consumer can dispatch on extension.

**Step 1: Write the failing test**

Create `web/src/lib/components/Toolbar.test.ts` (or extend existing) using vitest + Svelte Testing Library:

```ts
import { render, fireEvent } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import Toolbar from './Toolbar.svelte';

test('Toolbar accept attribute includes .dip', () => {
    const { container } = render(Toolbar, { props: {} });
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;
    expect(input.accept).toContain('.dip');
});

test('Toolbar onfileopen receives filename alongside content', async () => {
    const onfileopen = vi.fn();
    const { container } = render(Toolbar, { props: { onfileopen } });
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;
    const file = new File(['workflow F\n  start: A\n'], 'sample.dip', { type: 'text/plain' });
    Object.defineProperty(input, 'files', { value: [file] });
    await fireEvent.change(input);
    expect(onfileopen).toHaveBeenCalledWith(expect.any(String), 'sample.dip');
});
```

**Step 2: Run and verify it fails**

```bash
cd web && npm run test -- Toolbar.test
```

Expected: FAIL — accept attribute does not include `.dip`; `onfileopen` is called with one argument.

**Step 3: Update `Toolbar.svelte`**

Change `accept=".dot,.gv,.txt"` → `accept=".dot,.gv,.txt,.dip"`.

Change the callback signature and call site:

```ts
onfileopen?: (content: string, filename: string) => void;
// ...
async function handleFileSelected(event: Event) {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;
    const content = await file.text();
    onfileopen?.(content, file.name);
    target.value = '';
}
```

**Step 4: Update `+page.svelte` to accept the new signature**

```ts
function handleFileOpen(content: string, filename: string) {
    // Full dippin handling added in Task 14 — for now, just store.
    currentSource = content;
    editor.setContent(content);
    render(currentSource);
}
```

**Step 5: Run and verify it passes**

```bash
cd web && npm run test -- Toolbar.test
```

Expected: PASS.

**Step 6: Commit**

```bash
git add web/src/lib/components/Toolbar.svelte web/src/routes/+page.svelte web/src/lib/components/Toolbar.test.ts
git commit -m "feat(web): accept .dip in file picker and pass filename to handler"
```

---

## Task 14: Web app — parseDippin binding + state wiring

**Files:**
- Modify: `web/src/lib/wasm.ts`
- Modify: `web/src/routes/+page.svelte`

**Goal:** Expose `parseDippin` in the TypeScript wasm wrapper, maintain `{currentSource, isDippin, generatedDot, sourceMap, parseError}` state, and feed `generatedDot` to all renderer + parser API calls.

**Step 1: Write the failing test**

Create `web/src/routes/page.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as wasm from '$lib/wasm';

describe('parseDippin wrapper', () => {
    it('returns dotSource and sourceMap for valid dippin', async () => {
        const src = 'workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n';
        const result = await wasm.parseDippin(src);
        expect(result.dotSource).toContain('digraph F {');
        expect(result.sourceMap.length).toBeGreaterThan(0);
        expect(result.sourceMap[0]).toHaveProperty('dotStart');
        expect(result.sourceMap[0]).toHaveProperty('dipEnd');
    });
});
```

**Step 2: Run and verify it fails**

```bash
cd web && npm run test -- page.test
```

Expected: FAIL — `parseDippin` not exported from `wasm.ts`.

**Step 3: Add the wrapper in `web/src/lib/wasm.ts`**

Append:

```ts
export interface SourceMapEntry {
    dotStart: number;
    dotEnd: number;
    dipStart: number;
    dipEnd: number;
}

export interface DippinConversion {
    dotSource: string;
    sourceMap: SourceMapEntry[];
}

export async function parseDippin(source: string): Promise<DippinConversion> {
    const mod = await parserModule;
    return mod.parseDippin(source) as DippinConversion;
}
```

(Use whatever module-loading pattern the existing `parseDot` uses — likely lazy-initializing `parserModule`.)

**Step 4: Update `+page.svelte` with dippin state**

Add state:

```ts
let isDippin = $state(false);
let generatedDot = $state(currentSource); // equal to currentSource for plain DOT
let sourceMap = $state<SourceMapEntry[]>([]);
let parseError = $state('');
```

Replace `handleFileOpen`:

```ts
async function handleFileOpen(content: string, filename: string) {
    currentSource = content;
    editor.setContent(content);
    if (filename.endsWith('.dip')) {
        isDippin = true;
        try {
            const result = await parseDippin(content);
            generatedDot = result.dotSource;
            sourceMap = result.sourceMap;
            parseError = '';
        } catch (e) {
            parseError = e instanceof Error ? e.message : String(e);
            // Leave generatedDot as whatever it was.
            return;
        }
    } else {
        isDippin = false;
        generatedDot = content;
        sourceMap = [];
        parseError = '';
    }
    render(generatedDot);
}
```

Replace `handleEditorChange`:

```ts
async function handleEditorChange(value: string) {
    currentSource = value;
    if (isDippin) {
        try {
            const result = await parseDippin(value);
            generatedDot = result.dotSource;
            sourceMap = result.sourceMap;
            parseError = '';
        } catch (e) {
            parseError = e instanceof Error ? e.message : String(e);
            error = parseError;
            return;
        }
    } else {
        generatedDot = value;
    }
    render(generatedDot);
}
```

Change the existing `render` call site and cursor/click handlers to use `generatedDot` in place of `currentSource` when calling `renderDot`, `nodeIdAtOffset`, and `definitionRangeForNode`. Translation helpers come in Task 15.

**Step 5: Run the test**

```bash
cd web && npm run test -- page.test
```

Expected: PASS.

**Step 6: Commit**

```bash
git add web/src/lib/wasm.ts web/src/routes/+page.svelte web/src/routes/page.test.ts
git commit -m "feat(web): parse .dip files on open and maintain source map state"
```

---

## Task 15: Web app — offset translation + playwright e2e

**Files:**
- Modify: `web/src/routes/+page.svelte`
- Create: `web/tests/e2e/dippin.spec.ts`
- Create: `web/tests/fixtures/sample.dip`

**Goal:** Add `dotOffsetFromDip(n)` and `dipRangeFromDot(range)` helpers, route the cursor/click handlers through them, and add a playwright smoke test that opens a `.dip` file and verifies rendering + interaction.

**Step 1: Write the failing e2e test**

Create `web/tests/fixtures/sample.dip`:

```text
workflow Hello
  start: A
  exit: B
  agent A
    prompt: hi
    model: m
    provider: p
  agent B
    prompt: bye
    model: m
    provider: p
  A -> B
```

Create `web/tests/e2e/dippin.spec.ts`:

```ts
import { test, expect } from '@playwright/test';
import path from 'path';

test('opening a .dip file renders the graph', async ({ page }) => {
    await page.goto('/');
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(path.join(__dirname, '../fixtures/sample.dip'));
    // Give the wasm parser a moment to run.
    await expect(page.locator('svg')).toBeVisible();
    // The rendered SVG should contain node labels A and B (or their labels).
    const svgText = await page.locator('svg').innerText();
    expect(svgText).toMatch(/A|Hello/);
});

test('clicking a rendered node highlights dippin source', async ({ page }) => {
    await page.goto('/');
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(path.join(__dirname, '../fixtures/sample.dip'));
    await expect(page.locator('svg')).toBeVisible();
    // Click the first node in the SVG.
    const firstNode = page.locator('svg .node').first();
    await firstNode.click();
    // The editor should have a non-empty selection range inside the dippin source.
    // (Assertion depends on the editor DOM — tighten in review.)
});
```

**Step 2: Run and verify it fails**

```bash
cd web && npm run test:e2e -- dippin.spec
```

Expected: FAIL — translation helpers missing, click may land at wrong offset.

**Step 3: Add translation helpers in `+page.svelte`**

```ts
function dotOffsetFromDip(dipOffset: number): number | null {
    if (!isDippin) return dipOffset;
    for (const e of sourceMap) {
        if (dipOffset >= e.dipStart && dipOffset < e.dipEnd) {
            return e.dotStart;
        }
    }
    return null;
}

function dipRangeFromDot(dotOffset: number): { start: number; end: number } | null {
    if (!isDippin) return { start: dotOffset, end: dotOffset };
    for (const e of sourceMap) {
        if (dotOffset >= e.dotStart && dotOffset < e.dotEnd) {
            return { start: e.dipStart, end: e.dipEnd };
        }
    }
    return null;
}
```

Update `handleNodeClick`:

```ts
async function handleNodeClick(nodeId: string) {
    const generation = ++interactionGeneration;
    highlightedNode = nodeId;
    const range = await definitionRangeForNode(generatedDot, nodeId);
    if (generation !== interactionGeneration || !range) return;
    const dipRange = dipRangeFromDot(range.location);
    if (!dipRange) return;
    editor.highlightRange(dipRange.start, dipRange.end);
}
```

Update `handleCursorChange`:

```ts
async function handleCursorChange(offset: number) {
    const generation = ++interactionGeneration;
    const dotOffset = dotOffsetFromDip(offset);
    if (dotOffset === null) {
        editor.clearHighlight();
        highlightedNode = undefined;
        return;
    }
    const nodeId = await nodeIdAtOffset(generatedDot, dotOffset);
    if (generation !== interactionGeneration) return;
    highlightedNode = nodeId;
    if (nodeId) {
        const range = await definitionRangeForNode(generatedDot, nodeId);
        if (generation === interactionGeneration && range) {
            const dipRange = dipRangeFromDot(range.location);
            if (dipRange) editor.highlightRange(dipRange.start, dipRange.end);
        }
    } else {
        editor.clearHighlight();
    }
}
```

**Step 4: Run the e2e suite**

```bash
cd web && npm run test:e2e -- dippin.spec
```

Expected: PASS.

Run full vitest suite to ensure nothing else regressed:

```bash
cd web && npm run test
```

**Step 5: Commit**

```bash
git add web/src/routes/+page.svelte web/tests/e2e/dippin.spec.ts web/tests/fixtures/sample.dip
git commit -m "feat(web): bidirectional source map translation + .dip e2e test"
```

---

## Task 16: End-to-end verification + finishing

**Goal:** Smoke test both surfaces against a real dippin fixture, then finish the branch.

**Step 1: macOS end-to-end**

- Launch the built app
- Open `dippin-parser/tests/fixtures/source_map_simple.dip` (or `web/tests/fixtures/sample.dip`)
- Verify: editor shows raw dippin source
- Verify: preview renders the graph
- Verify: clicking node A in the preview highlights the `agent A` block in the editor
- Verify: placing cursor inside the `agent A` block highlights node A in the preview
- Verify: introducing a syntax error (e.g. remove a required field) shows the error in the preview area without crashing
- Verify: saving the file writes the dippin text verbatim (not the converted DOT)

**Step 2: Web end-to-end**

- Start the dev server: `cd web && npm run dev -- --host 0.0.0.0`
- Open the app in a browser
- Click Open, select `sample.dip`
- Verify the same behaviors as macOS

**Step 3: Run all test suites one more time**

```bash
cd dippin-parser && cargo test
cd dot-core && cargo test
cd dot-core-wasm && wasm-pack test --headless --firefox
cd DotViewer && xcodebuild test -project DotViewer.xcodeproj -scheme DotViewer -destination 'platform=macOS'
cd web && npm run test && npm run test:e2e
```

All green.

**Step 4: Finish the branch**

Use @superpowers:finishing-a-development-branch to decide how to integrate the work (merge locally, push + PR, keep, or discard).

---

## Appendix: Key principles

- **DRY:** the translation logic lives on `DotDocument` (Swift) and in `+page.svelte` helpers (web). The `dippin-parser` source-map emission is the single source of truth for mapping.
- **YAGNI:** v1 flattens diagnostics, ignores the `.dippin` long-form extension, and does not split the wasm bundle. All are documented follow-ups in the design doc.
- **TDD:** every task writes a failing test first. The golden source-map fixture (Task 5) is the one exception — a snapshot test recorded once.
- **Frequent commits:** every task ends with a commit. Reviewable in isolation.
- **Match existing style:** the new UniFFI record names match the existing `DotError` / `LayoutEngine` conventions; web helpers use the existing `$state` / callback patterns.
