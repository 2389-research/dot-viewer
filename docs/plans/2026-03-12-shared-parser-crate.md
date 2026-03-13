# Shared Parser Crate Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract the duplicated DOT parser into a shared `dot-parser` crate with feature flags for UniFFI and serde derives.

**Architecture:** Create `dot-parser/` containing all types, parsing logic, and query helpers. Use `cfg_attr` for conditional derives. `dot-core` and `dot-core-wasm` become thin wrappers that re-export types and add platform-specific export attributes.

**Tech Stack:** Rust, cargo features, UniFFI 0.30, serde, wasm-bindgen

---

## Task 1: Create dot-parser crate with types and parsing logic

Move the shared parser code into a standalone crate with feature-flagged derives.

**Files:**
- Create: `dot-parser/Cargo.toml`
- Create: `dot-parser/src/lib.rs`

**Step 1: Create `dot-parser/Cargo.toml`**

```toml
# ABOUTME: Cargo manifest for the shared DOT parser library.
# ABOUTME: Feature flags control UniFFI or serde derive macros on types.

[package]
name = "dot-parser"
version = "0.1.0"
edition = "2021"

[features]
default = []
uniffi = ["dep:uniffi"]
serde = ["dep:serde"]

[dependencies]
uniffi = { version = "0.30", optional = true }
serde = { version = "1", features = ["derive"], optional = true }
```

**Step 2: Create `dot-parser/src/lib.rs`**

This file contains all types, helpers, parsing logic, query functions, and tests — moved from `dot-core/src/parser.rs`. Key changes from the original:

1. Types get `cfg_attr` for conditional derives:
```rust
#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SourceRange { ... }

#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum DotStatement { ... }

#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DotGraph { ... }
```

2. `parse_dot` takes `&str` (the natural Rust API):
```rust
pub fn parse_dot(source: &str) -> DotGraph { ... }
```

3. Query functions use references:
```rust
pub fn statement_at(graph: &DotGraph, offset: u32) -> Option<&DotStatement> { ... }
pub fn node_id_at(statement: &DotStatement, offset: u32) -> Option<String> { ... }
pub fn definition_for_node<'a>(graph: &'a DotGraph, node_id: &str) -> Option<&'a DotStatement> { ... }
```

4. All 27 unit tests are included, calling `parse_dot("...")` with `&str`.

5. All scanning helpers (`is_ident_char`, `skip_whitespace_and_semicolons`, `skip_whitespace_only`, `skip_to_end_of_line`, `skip_block_comment`, `extract_identifier`, `find_statement_end`) are private functions in this crate.

6. `DotStatement::source_range(&self) -> &SourceRange` is a public method.

**Step 3: Run tests**

Run: `cd dot-parser && cargo test`
Expected: All 27 tests pass

**Step 4: Verify with each feature flag**

Run: `cd dot-parser && cargo test --features uniffi`
Expected: PASS (types get UniFFI derives)

Run: `cd dot-parser && cargo test --features serde`
Expected: PASS (types get serde derives)

**Step 5: Commit**

```bash
git add dot-parser/
git commit -m "feat: create dot-parser shared crate with feature-flagged derives"
```

---

## Task 2: Update dot-core to depend on dot-parser

Replace `dot-core/src/parser.rs` with thin UniFFI export wrappers around `dot-parser`.

**Files:**
- Modify: `dot-core/Cargo.toml`
- Rewrite: `dot-core/src/parser.rs`

**Step 1: Add dot-parser dependency to `dot-core/Cargo.toml`**

Add to `[dependencies]`:
```toml
dot-parser = { path = "../dot-parser", features = ["uniffi"] }
```

**Step 2: Replace `dot-core/src/parser.rs`**

The file becomes thin wrappers that re-export types and provide `#[uniffi::export]` functions. The UniFFI export boundary requires owned `String` parameters and cloned return values:

```rust
// ABOUTME: UniFFI export wrappers for the shared DOT parser.
// ABOUTME: Re-exports parser types and provides owned-value wrappers for FFI boundary.

pub use dot_parser::{DotGraph, DotStatement, SourceRange};

#[uniffi::export]
pub fn parse_dot(source: String) -> DotGraph {
    dot_parser::parse_dot(&source)
}

#[uniffi::export]
pub fn statement_at(graph: &DotGraph, offset: u32) -> Option<DotStatement> {
    dot_parser::statement_at(graph, offset).cloned()
}

#[uniffi::export]
pub fn node_id_at(statement: &DotStatement, offset: u32) -> Option<String> {
    dot_parser::node_id_at(statement, offset)
}

#[uniffi::export]
pub fn definition_for_node(graph: &DotGraph, node_id: String) -> Option<DotStatement> {
    dot_parser::definition_for_node(graph, &node_id).cloned()
}
```

**Step 3: Run dot-core tests**

Run: `cd dot-core && cargo test`
Expected: All tests pass (lib.rs tests for render/validate, parser tests now in dot-parser)

**Step 4: Verify UniFFI bindings still generate**

Run: `make generate-bindings`
Expected: Swift bindings generated without errors

**Step 5: Commit**

```bash
git add dot-core/Cargo.toml dot-core/src/parser.rs
git commit -m "refactor(dot-core): use dot-parser shared crate for parsing"
```

---

## Task 3: Update dot-core-wasm to depend on dot-parser

Replace the duplicated parser code in `dot-core-wasm/src/lib.rs` with imports from `dot-parser`.

**Files:**
- Modify: `dot-core-wasm/Cargo.toml`
- Rewrite: `dot-core-wasm/src/lib.rs`

**Step 1: Add dot-parser dependency to `dot-core-wasm/Cargo.toml`**

Add to `[dependencies]`:
```toml
dot-parser = { path = "../dot-parser", features = ["serde"] }
```

Remove the direct `serde` dependency since `dot-parser` handles it:
```toml
[dependencies]
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"
dot-parser = { path = "../dot-parser", features = ["serde"] }
```

**Step 2: Replace `dot-core-wasm/src/lib.rs`**

The file becomes thin wasm-bindgen wrappers:

```rust
// ABOUTME: WASM entry point for the DOT parser, used by the web editor.
// ABOUTME: Thin wasm-bindgen wrappers around the shared dot-parser crate.

use wasm_bindgen::prelude::*;

// Re-export types so they're available if needed
pub use dot_parser::{DotGraph, DotStatement, SourceRange};

#[wasm_bindgen(js_name = "parseDot")]
pub fn parse_dot_wasm(source: &str) -> Result<JsValue, JsValue> {
    let graph = dot_parser::parse_dot(source);
    serde_wasm_bindgen::to_value(&graph)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "nodeIdAtOffset")]
pub fn node_id_at_offset_wasm(source: &str, offset: u32) -> Option<String> {
    let graph = dot_parser::parse_dot(source);
    let stmt = dot_parser::statement_at(&graph, offset)?;
    dot_parser::node_id_at(stmt, offset)
}

#[wasm_bindgen(js_name = "definitionOffsetForNode")]
pub fn definition_offset_for_node_wasm(source: &str, node_id: &str) -> Option<u32> {
    let graph = dot_parser::parse_dot(source);
    let stmt = dot_parser::definition_for_node(&graph, node_id)?;
    Some(stmt.source_range().location)
}
```

**Step 3: Run tests and build WASM**

Run: `cd dot-core-wasm && cargo test`
Expected: No tests in this crate (all moved to dot-parser)

Run: `cd dot-core-wasm && wasm-pack build --target web --release`
Expected: WASM package builds successfully

**Step 4: Commit**

```bash
git add dot-core-wasm/Cargo.toml dot-core-wasm/src/lib.rs
git commit -m "refactor(dot-core-wasm): use dot-parser shared crate for parsing"
```

---

## Task 4: Verify full build pipeline

Verify everything still works end-to-end: Rust tests, UniFFI bindings, WASM build, web type-check.

**Step 1: Run all Rust tests**

Run: `cargo test -p dot-parser && cargo test -p dot-core && cargo test -p dot-core-wasm`
Expected: All pass

**Step 2: Generate UniFFI bindings**

Run: `make generate-bindings`
Expected: Success

**Step 3: Build WASM and web app**

Run: `cd dot-core-wasm && wasm-pack build --target web --release`
Run: `cd web && npm run build`
Expected: Both succeed

**Step 4: Commit any lockfile changes**

```bash
git add dot-parser/Cargo.lock dot-core-wasm/Cargo.lock
git commit -m "chore: update lockfiles after parser extraction"
```
