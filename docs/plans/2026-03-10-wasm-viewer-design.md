# WASM Web Viewer Design

## Goal

Build a full web-based DOT editor with live SVG preview, powered by the same Rust/Graphviz core as the macOS app compiled to WebAssembly.

## Architecture

Cross-compile the existing `dot-core` Rust library (which wraps Graphviz 12.2.1 via C FFI) to `wasm32-unknown-emscripten`. Emscripten handles the C library compilation (Graphviz's ~23 static libraries) while Rust targets the same Emscripten environment for seamless linking.

JS/TS bindings are generated via `uniffi-bindgen-javascript` — the same `#[uniffi::export]` annotations that produce Swift bindings also produce TypeScript bindings for the web. This keeps a single API definition for all platforms.

The web app is a SvelteKit static site in `web/` at the repo root.

### Repo Structure

```
dot-viewer/
  dot-core/                # Rust lib (expanded API)
    src/
      lib.rs                # #[uniffi::export] — existing + new parser API
      graphviz.rs           # Unchanged C FFI wrapper
      parser.rs             # New: DotParser ported from Swift to Rust
    build.rs                # Extended: Emscripten CMake toolchain for wasm32
    Cargo.toml
  web/                      # New: SvelteKit app
    src/
      lib/
        dot-core/            # Generated JS/TS bindings from uniffi-bindgen-javascript
        components/          # Svelte components (Editor, Preview, Toolbar, etc.)
      routes/
        +page.svelte         # Main editor page
    static/
    package.json
    svelte.config.js         # adapter-static for host-agnostic deployment
  DotViewer/                 # Existing macOS app (updated to use Rust parser)
```

## API Surface

The `dot-core` Rust library exposes these functions via UniFFI to all clients:

```rust
// Existing
render_dot(source: String, engine: LayoutEngine) -> Result<String, DotError>
validate_dot(source: String) -> Result<(), DotError>

// New — DotParser ported from Swift to Rust
parse_dot(source: String) -> DotGraph

// DotGraph methods:
//   statements() -> Vec<DotStatement>
//   statement_at(offset: u32) -> Option<DotStatement>
//   node_id_at(offset: u32) -> Option<String>
//   definition_for_node(id: String) -> Option<DotStatement>
```

The macOS app drops its Swift `DotParser` and uses the Rust one via UniFFI, ensuring parsing behavior is identical across all platforms.

## JS Bindings

`uniffi-bindgen-javascript` generates TypeScript bindings from the UniFFI proc-macro annotations. This is the same tool that will eventually serve Kotlin and other future clients.

A Makefile target runs `uniffi-bindgen-javascript` to produce the TS bindings + WASM output into `web/src/lib/dot-core/`.

## UI Components

### Editor Pane (left)
- CodeMirror 6 with DOT syntax grammar
- Line numbers, bracket matching, syntax highlighting (CodeMirror built-ins)
- Debounced input (300ms) triggers WASM `render_dot` call

### Preview Pane (right)
- Rendered SVG injected directly into the DOM
- Pan and zoom via CSS transforms + pointer events
- Click a node to highlight its definition in the editor (bidirectional navigation)

### Toolbar (top)
- Layout engine picker (dot, neato, fdp, circo, twopi, sfdp)
- Error display bar when Graphviz returns an error

### WASM Loading
- Editor renders immediately on page load
- WASM binary fetches in parallel (lazy loaded)
- Subtle loading indicator on preview pane until WASM is ready
- Initial render triggers once WASM is loaded

## Data Flow

```
User types in CodeMirror
  → 300ms debounce
  → call renderDot(source, engine) on WASM module
  → returns SVG string or DotError
  → SVG: inject into preview pane DOM
  → Error: show in error bar, keep last good SVG visible
```

Bidirectional navigation uses the Rust `DotParser` (via WASM):
- Cursor moves in editor → `node_id_at(offset)` → highlight node in SVG
- Click node in SVG → `definition_for_node(id)` → CodeMirror scrolls to definition

## Bundle Size

Graphviz compiled to WASM is ~2-4MB uncompressed. With brotli compression, ~800KB-1.5MB over the wire.

Mitigations:
- Lazy load WASM — editor is usable immediately
- `wasm-opt` optimization passes (~10-20% reduction)
- Cache headers — cached after first load
- Brotli compression (~30-40% smaller than gzip for WASM)

## Deployment

Static files via `@sveltejs/adapter-static`. Host-agnostic — works on Cloudflare Pages, Fly, Firebase Hosting, or local `npx serve`.

## Testing

### Rust (dot-core)
- Port existing 29 `DotParserTests` from Swift to Rust unit tests
- WASM integration tests via `wasm-pack test` (headless browser)
- Existing `render_dot`/`validate_dot` native tests unchanged

### Web (SvelteKit)
- Component tests: Vitest + `@testing-library/svelte`
- E2E tests: Playwright — load app, type DOT, verify SVG renders, test engine switching, test bidirectional navigation

### CI
- GitHub Actions job for WASM build (Emscripten + uniffi-bindgen-javascript)
- Playwright tests against built static site

## Key Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| WASM approach | Rust + Graphviz via Emscripten | Single codebase, rendering parity with macOS app |
| JS bindings | uniffi-bindgen-javascript | Single API definition for all platforms (Swift, JS, future Kotlin) |
| Frontend | SvelteKit + CodeMirror 6 | Lightweight runtime, minimal overhead alongside WASM |
| DOT parser | Port to Rust, expose via UniFFI | Single source of truth for all clients |
| Deployment | Static files, adapter-static | Host-agnostic, no server required |
| Repo structure | Monorepo (`web/` directory) | Shares Rust core directly |
