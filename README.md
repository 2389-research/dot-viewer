# Dot Viewer

A native macOS app for viewing and editing [Graphviz](https://graphviz.org/) `.dot` and `.gv` files with a split-pane interface and live SVG preview.

## Download

Grab the latest release from the [Releases page](https://github.com/2389-research/dot-viewer/releases). The DMG is code signed and notarized with Developer ID — just drag to Applications and launch.

Requires **macOS 14.0** or later. The app checks for updates automatically via Sparkle.

## Features

- **Live preview** — edits render to SVG in real time with debounced updates (300ms)
- **Multiple layout engines** — dot, neato, fdp, circo, twopi, sfdp selectable from the toolbar
- **Bidirectional navigation** — click a node in the preview to jump to its definition in the editor, and vice versa
- **Tabbed editing** — open multiple `.dot`/`.gv` files as native macOS window tabs
- **Zoom and pan** — navigate large graphs in the SVG preview
- **Undo/redo** — standard document undo support
- **Error display** — inline error bar shows Graphviz rendering errors

## Architecture

Three-layer design:

1. **dot-core** (Rust) — wraps Graphviz (cgraph + gvc) via C FFI, exposes a clean API through [UniFFI](https://mozilla.github.io/uniffi-rs/) bindings
2. **UniFFI bridge** — auto-generates Swift bindings from the compiled Rust static library
3. **DotViewer** (SwiftUI) — split-pane editor + preview, file handling, tabbed windows

Graphviz 12.2.1 is compiled from source as a static library — no system Graphviz installation required.

## Building from Source

### Prerequisites

- macOS 14.0+
- Xcode 16.0+
- Rust toolchain (`rustup`)
- Homebrew

### Steps

```bash
git clone https://github.com/2389-research/dot-viewer.git
cd dot-viewer

# Install build tools
brew install xcodegen bison flex

# Clone Graphviz source (built by Cargo via CMake)
cd dot-core
git clone --depth 1 --branch 12.2.1 https://gitlab.com/graphviz/graphviz.git graphviz-vendor
cd ..

# Build everything (Rust core + Swift bindings + Xcode app)
make
```

To open in Xcode after building:

```bash
cd DotViewer && xcodegen generate
open DotViewer.xcodeproj
```

## Project Structure

```
.github/workflows/    GitHub Actions release pipeline
dot-core/             Rust library (Graphviz FFI + UniFFI bindings)
  build.rs            CMake build orchestration for vendored Graphviz
  src/                Rust source
DotViewer/            SwiftUI macOS app
  project.yml         XcodeGen spec
  DotViewer/          App source (views, document model, Sparkle updater)
scripts/              Build and release helper scripts
Makefile              Top-level build orchestration
```

## Releases

Releases are automated via GitHub Actions. Pushing a `v*` tag triggers the full pipeline: Rust build, Swift build, Developer ID signing, Apple notarization, DMG packaging, Sparkle appcast generation, and GitHub Release creation.
