# Shared Parser Crate Design

**Goal:** Eliminate parser code duplication between `dot-core` and `dot-core-wasm` by extracting a shared `dot-parser` crate.

## Architecture

Create `dot-parser/` — a pure Rust library containing all DOT parsing types and logic. Feature flags control which derive macros are applied:

- `uniffi` feature: adds `uniffi::Record` / `uniffi::Enum` derives
- `serde` feature: adds `serde::Serialize` derive and `#[serde(tag = "type")]`

### Consumers

- `dot-core` depends on `dot-parser` with `features = ["uniffi"]`, re-exports types, and provides thin `#[uniffi::export]` wrappers
- `dot-core-wasm` depends on `dot-parser` with `features = ["serde"]`, and provides thin `#[wasm_bindgen]` wrappers

### Testing

All parser unit tests live in `dot-parser/`. Consumer crates only test their export layer.

## What Changes

| Component | Action |
|-----------|--------|
| `dot-parser/` (new) | Types, parsing logic, helpers, unit tests |
| `dot-core/src/parser.rs` | Replace with re-exports + UniFFI wrapper functions |
| `dot-core-wasm/src/lib.rs` | Replace parser code with imports + wasm-bindgen wrappers |
| `web/`, macOS app | No changes |
