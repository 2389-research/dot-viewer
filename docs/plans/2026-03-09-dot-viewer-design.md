# Dot Viewer — Design Document

## Overview

A native macOS app for viewing and editing Graphviz `.dot` files, similar to mermaid.live but local. Split-pane interface with a text editor on the left and live SVG preview on the right. Tabbed multi-file editing via native macOS window tabs.

## Architecture

Three layers in a single monorepo:

1. **`dot-core` (Rust library)** — Takes DOT text in, produces SVG string out. Wraps libgraphviz (cgraph + gvc) via C FFI. Exposes a clean API via UniFFI.
2. **UniFFI bridge** — Auto-generated Swift bindings from Rust.
3. **SwiftUI macOS app** — Split pane UI, file management, debounced rendering.

```
┌─────────────────────────────────────┐
│         SwiftUI macOS App           │
│  ┌──────────┐  ┌──────────────────┐ │
│  │  Text    │  │  WKWebView       │ │
│  │  Editor  │  │  (SVG Preview)   │ │
│  └────┬─────┘  └───────▲──────────┘ │
│       │    UniFFI Bridge│           │
│───────┼─────────────────┼───────────│
│       ▼                 │           │
│  ┌──────────────────────┴─────────┐ │
│  │       dot-core (Rust lib)      │ │
│  │  DOT text → libgraphviz → SVG │ │
│  └────────────────────────────────┘ │
└─────────────────────────────────────┘
```

## Rust Core API

```rust
fn render_dot(dot_source: String, engine: LayoutEngine) -> Result<String, DotError>
fn validate_dot(dot_source: String) -> Result<(), DotError>

enum LayoutEngine {
    Dot,      // hierarchical (default)
    Neato,    // spring model
    Fdp,      // force-directed
    Circo,    // circular
    Twopi,    // radial
    Sfdp,     // scalable force-directed
}

enum DotError {
    SyntaxError { message: String, line: u32, column: u32 },
    LayoutError { message: String },
    RenderError { message: String },
}
```

- `render_dot` is synchronous — Swift side handles debounce + background threading.
- `validate_dot` is separate for real-time syntax checking without full layout cost.
- Structured errors carry line/column for inline editor markers.

## SwiftUI App

### Window & Tabs
- `DocumentGroup`-based app using `ReferenceFileDocument`.
- Native macOS tab system (Cmd+T, Cmd+Shift+]).
- File association with `.dot` and `.gv` extensions.
- Autosave via `ReferenceFileDocument`.

### Split Pane
- `HSplitView` with draggable divider.
- Left: NSTextView wrapper (syntax highlighting, error markers).
- Right: WKWebView displaying SVG in a minimal HTML shell.
- Toolbar: layout engine picker, render mode toggle (live/manual), refresh button.

### Render Flow
1. User types in editor.
2. Live mode: 300ms debounce timer fires after idle.
3. `validate_dot()` runs on background thread — update editor error markers.
4. If valid, `render_dot()` runs on background thread.
5. SVG pushed into WKWebView via `evaluateJavaScript`.
6. Manual mode: steps 2-5 only on Cmd+S or refresh button click.
7. On error, preview keeps showing last valid render.

### Editor (v1 → v2)
- v1: NSTextView with basic DOT syntax highlighting, inline error markers.
- v2: Line numbers, bracket matching, autocomplete for DOT keywords.

## Project Structure

```
dot-viewer/
├── dot-core/                    # Rust library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs               # UniFFI exports
│   │   ├── graphviz.rs          # libgraphviz C FFI bindings
│   │   └── render.rs            # render/validate logic
│   ├── uniffi.toml
│   └── build.rs                 # links libgraphviz, generates UniFFI scaffolding
├── DotViewer/                   # SwiftUI macOS app
│   ├── DotViewer.xcodeproj
│   ├── DotViewer/
│   │   ├── DotViewerApp.swift
│   │   ├── DotDocument.swift    # ReferenceFileDocument
│   │   ├── ContentView.swift    # HSplitView container
│   │   ├── EditorView.swift     # NSTextView wrapper
│   │   ├── PreviewView.swift    # WKWebView wrapper
│   │   └── DotCore.swift        # Generated UniFFI bindings
│   └── Resources/
│       └── preview.html         # Minimal HTML shell for SVG display
├── Makefile                     # Orchestrates cargo build + xcode build
└── docs/plans/
```

## Build System

1. `make build` runs `cargo build --release` for `dot-core`.
2. Copies `.a` static library and generated Swift bindings into the Xcode project.
3. Xcode builds the Swift app, links against the Rust library.

Graphviz is statically linked via vendored build (`cc` crate or `graphviz-sys`) so the app has no runtime dependency on a Graphviz install.

## Future Considerations

- Extract `dot-core` into its own repo when building iOS/Android clients.
- iOS app via UniFFI + SwiftUI (same Rust core).
- Android app via UniFFI + Kotlin.
