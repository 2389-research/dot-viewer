# Dippin Full Wiring Design

**Date:** 2026-04-08
**Status:** Approved, awaiting implementation plan
**Scope:** Wire `dippin-parser` through `dot-core` (UniFFI) and `dot-core-wasm` so `.dip` files are first-class in both the macOS DotViewer app and the web app, with full bidirectional editor↔preview linking.

---

## Goal

Make `.dip` files openable and editable in the macOS DotViewer app and the web app. The user edits raw dippin source, the preview renders the converted DOT, and clicking a node in the preview highlights the corresponding dippin source range (and vice versa).

## Context

The `dippin-parser` crate already exists and is thoroughly tested. It parses dippin workflow source and exports to DOT via `export_dot.rs`. Today it is consumed only by `dot-viewer-cli`. Neither `dot-core` (the UniFFI crate used by the macOS app) nor `dot-core-wasm` (the wasm crate used by the web app) know about dippin. Opening a `.dip` file in either surface does not work.

## Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Editor shows raw `.dip` source; preview shows the rendered graph | Preserves round-trip fidelity and matches user mental model ("I'm editing a dippin file") |
| 2 | Conversion lives inside `dot-core` / `dot-core-wasm` (not a separate FFI crate) | One function per surface, no new build target; keeps the FFI surface minimal |
| 3 | Diagnostics flattened to `DotError::SyntaxError("file:line:col: msg")` for v1 | Ships fast, matches existing error pattern; structured diagnostics are a follow-up |
| 4 | One new UTI `com.2389.dot-viewer.dip` for extension `.dip` only | Minimum scope; `.dippin` long-form can be added later with a plist edit |
| 5 | Full bidirectional source map (DOT range ↔ dippin range) | Linking is core UX; half-broken linking is worse than none |

---

## Architecture

### Crate topology

```
dippin-parser (standalone, no FFI)
    │
    ├─ dot-core (UniFFI → Swift)
    │       │
    │       └─ DotViewer (macOS app)
    │
    ├─ dot-core-wasm (wasm-bindgen → JS)
    │       │
    │       └─ web/ (SvelteKit app)
    │
    └─ dot-viewer-cli (direct consumer, unchanged)
```

### New API: `dippin-parser`

```rust
pub struct DippinConversion {
    pub dot_source: String,
    /// (dot_byte_range, dip_byte_range) pairs, one per emitted node/edge line.
    pub source_map: Vec<(Range<usize>, Range<usize>)>,
}

pub fn parse_to_dot_with_map(source: &str, path: &str) -> Result<DippinConversion, DippinError>;
```

The existing `parse_to_dot` becomes a thin wrapper that discards the map. Implementation threads byte-span tracking from the lexer through the IR, so `ir::Node` and `ir::Edge` carry a full source range (not just `Location { line, col }`). `export_dot` captures `dot_source.len()` before and after writing each node/edge line to derive the DOT range and pairs it with the dippin span.

### New API: `dot-core` (UniFFI)

```rust
#[derive(uniffi::Record)]
pub struct DippinConversionResult {
    pub dot_source: String,
    pub source_map: Vec<SourceMapEntry>,
}

#[derive(uniffi::Record)]
pub struct SourceMapEntry {
    pub dot_start: u32, pub dot_end: u32,
    pub dip_start: u32, pub dip_end: u32,
}

#[uniffi::export]
pub fn parse_dippin(source: String) -> Result<DippinConversionResult, DotError>;
```

On parse failure, flatten dippin diagnostics to `DotError::SyntaxError(message)` where `message` is the first diagnostic formatted as `"file:line:col: text"`.

### New API: `dot-core-wasm` (wasm-bindgen)

```rust
#[wasm_bindgen(js_name = parseDippin)]
pub fn parse_dippin(source: &str) -> Result<JsValue, JsValue>;
// Returns { dotSource: string, sourceMap: [{dotStart, dotEnd, dipStart, dipEnd}, ...] }
```

### CLI

`dot-viewer-cli` stays on direct `dippin-parser` usage. No changes.

---

## macOS app wiring

### UTI registration (`DotViewer/project.yml`)

- Add `UTExportedTypeDeclarations` entry for `com.2389.dot-viewer.dip` conforming to `public.plain-text`, description "Dippin Workflow", extension `dip`.
- Add `CFBundleDocumentTypes` entry with `LSItemContentTypes = ["com.2389.dot-viewer.dip"]`, `CFBundleTypeExtensions = ["dip"]`, role `Editor`.
- Regenerate `DotViewer.xcodeproj` via `xcodegen`.

### `DotDocument.swift`

```swift
extension UTType {
    static let dippin: UTType = UTType(exportedAs: "com.2389.dot-viewer.dip")
}

final class DotDocument: ReferenceFileDocument {
    static var readableContentTypes: [UTType] {
        [.graphvizDot, .graphvizGv, .msWordDot, .dippin, .plainText]
    }

    var text: String              // source the user edits (dippin or DOT)
    var isDippin: Bool            // true when opened from .dip
    var generatedDot: String      // rendered DOT (== text when !isDippin)
    var sourceMap: [SourceMapEntry]  // empty when !isDippin
    var parseError: String?       // dippin parse failure message

    // On read(configuration:): if contentType == .dippin, call parse_dippin()
    //   and populate isDippin, generatedDot, sourceMap.
    // On text change: if isDippin, re-run parse_dippin() (debounced alongside render).
    // On snapshot/fileWrapper: write `text` verbatim; no conversion on save.
}
```

### Preview pipeline

Existing render call site uses `document.text` as the DOT source. Change to `document.generatedDot`. One-line change.

### Editor↔preview linking

Two helpers on `DotDocument`:

```swift
func dotOffsetForDippinRange(_ dip: Range<Int>) -> Range<Int>?
func dippinRangeForDotOffset(_ dot: Int) -> Range<Int>?
```

Both are identity when `!isDippin`. When `isDippin`, they walk `sourceMap` to translate.

- **Cursor → node highlight:** cursor offset is in `text` (dippin). Translate to `generatedDot` offset via `dotOffsetForDippinRange`. Call existing `definitionForNode` / `nodeIdAt` on `generatedDot`.
- **Node click → editor cursor:** preview sends node ID. `definitionRangeForNode(generatedDot, nodeId)` gives a DOT range. Translate via `dippinRangeForDotOffset`. Highlight in editor.

### Parse errors

If `parse_dippin` throws, show the error in the existing preview error area. Editor retains the user's text. No crash, no data loss.

---

## Web app wiring

### `Toolbar.svelte`

- Update `accept=".dot,.gv,.txt"` → `accept=".dot,.gv,.txt,.dip"`
- Change `onfileopen` callback signature to pass filename alongside content:

```ts
onfileopen?: (content: string, filename: string) => void;
```

### `wasm.ts`

```ts
export interface SourceMapEntry {
    dotStart: number; dotEnd: number;
    dipStart: number; dipEnd: number;
}
export interface DippinConversion {
    dotSource: string;
    sourceMap: SourceMapEntry[];
}
export async function parseDippin(source: string): Promise<DippinConversion>;
```

### `+page.svelte`

Mirror the Swift state model:

```ts
let currentSource = $state('...');     // editor content (dippin or DOT)
let isDippin = $state(false);
let generatedDot = $state('');          // feeds renderer + parser APIs
let sourceMap = $state<SourceMapEntry[]>([]);
let parseError = $state('');
```

- `handleFileOpen(content, filename)`: if `filename.endsWith('.dip')`, set `isDippin = true`, call `parseDippin(content)`, store results. Otherwise set `isDippin = false`, `generatedDot = content`. Always update `currentSource` and call `render(generatedDot)`.
- `handleEditorChange(value)`: if `isDippin`, re-parse (debounced), update `generatedDot` / `sourceMap` / `parseError`. If error, show it and skip render. If success, `render(generatedDot)`. Otherwise `generatedDot = value; render(value)`.
- `handleNodeClick` / `handleCursorChange`: use two translation helpers `dotOffsetFromDip(n)` and `dipRangeFromDot(range)`, both identity when `!isDippin`. Route through `generatedDot` for all parser API calls.

### Build

`dot-core-wasm` already produces a wasm package for the web. Adding `dippin-parser` as a dep grows the wasm binary by ~20-40 KB compressed. Acceptable for v1. If it becomes a problem, dippin can be split into a lazy-loaded chunk later.

---

## Testing strategy

### `dippin-parser`

- Unit tests for `parse_to_dot_with_map`:
  - `dot_source` equals existing `parse_to_dot` output (regression guard)
  - Each source-map entry's DOT range slices to a valid node/edge line
  - Each dippin range points to the corresponding `agent Foo` / `human Bar` / edge source
- Golden fixtures get `*.map.json` siblings listing expected `(dot_range, dip_range)` pairs for stability.

### `dot-core`

- Rust integration test calling `parse_dippin` on a canonical workflow; asserts non-empty `dot_source`, non-empty `source_map`, and that a syntax-error input returns `DotError::SyntaxError` with a message containing `line:col:`.

### `dot-core-wasm`

- `wasm-bindgen-test` case calling `parseDippin` on a small workflow; asserts returned object has `dotSource` and `sourceMap` with correct shapes.

### macOS app

- `DotDocumentTests`: open a `.dip` fixture, assert `isDippin == true`, `generatedDot` is non-empty valid DOT, `sourceMap` non-empty. Editing text reparses. Invalid dippin sets `parseError`.
- `DotDocumentMappingTests`: `dippinRangeForDotOffset` and `dotOffsetForDippinRange` round-trip on a known map.
- No UI tests; linking logic lives in pure Swift helpers.

### Web app

- Vitest: `+page.test.ts` covering `handleFileOpen` with a `.dip` filename, `handleEditorChange` re-parsing dippin edits, and the two offset translation helpers.
- Playwright e2e: one scenario — open a `.dip` fixture via the file input, assert preview renders without error, click a node, assert the editor highlights a range inside the dippin source.

### Fixtures

Reuse `dippin-parser/tests/fixtures/*.dip`. Copy one canonical small workflow into `DotViewer/DotViewerTests/Fixtures/` and `web/tests/fixtures/` so platform tests don't reach across the workspace.

---

## Rollout order

Each step is independently testable and can be committed separately.

1. **`dippin-parser`**: add `parse_to_dot_with_map` + source map generation + tests. Thread byte spans through IR. No FFI changes yet.
2. **`dot-core`**: add `parse_dippin` UniFFI export + `DippinConversionResult` / `SourceMapEntry` records + error mapping. Run `make generate-bindings`.
3. **`dot-core-wasm`**: add `parseDippin` wasm-bindgen export. Rebuild wasm pkg.
4. **`DotViewer`**: UTI registration + xcodegen. Extend `DotDocument` with dippin state + translation helpers. Wire preview to `generatedDot`. Tests.
5. **Web**: update Toolbar accept list, extend `+page.svelte` state, add translation helpers, update `wasm.ts` types. Tests.
6. **End-to-end**: open a `.dip` fixture in both macOS and web; verify rendering and bidirectional linking.

---

## Risks & mitigations

| Risk | Mitigation |
|------|------------|
| Source map accuracy across defaults/stylesheets/edges | Build the map incrementally (nodes, then edges, then config attrs) with golden tests per construct |
| IR doesn't carry byte spans today — only `Location { line, col }` | Thread spans from lexer into IR; contained to `dippin-parser` |
| UniFFI bindings drift at Xcode link time | `make generate-bindings` after any `dot-core` export change; documented in the plan |
| Wasm bundle growth from adding `dippin-parser` | Accept ~20-40 KB for v1; lazy-load split is a follow-up if needed |
| Editor re-parse cost on keystroke | Debounce the reparse alongside the existing render debounce |
| `.dot` Word template collision | Unrelated; already handled. Dippin uses `.dip` — no collision |

---

## Out of scope (follow-ups)

- Structured `DippinDiagnostic` UniFFI record with per-diagnostic line/col/severity — flattened for v1.
- `.dippin` long-form extension — plist addition if desired later.
- Dippin syntax highlighting in the editor — stays plain text for now.
- Save-as-DOT command — user explicitly chose raw-dippin editing, no conversion on save.
- Lazy-loaded wasm split for the dippin parser.
