# Dot Viewer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a native macOS split-pane editor for Graphviz .dot files with live SVG preview.

**Architecture:** Rust core library (`dot-core`) wraps libgraphviz via C FFI, exposes `render_dot`/`validate_dot` via UniFFI proc macros. SwiftUI app provides split-pane UI with NSTextView editor and WKWebView SVG preview. Monorepo with Makefile orchestrating Cargo + Xcode builds.

**Tech Stack:** Rust, UniFFI 0.30 (proc macros), libgraphviz (vendored static build via CMake), SwiftUI, AppKit (NSTextView), WebKit (WKWebView)

---

### Task 1: Initialize Rust Crate with UniFFI Scaffolding

**Files:**
- Create: `dot-core/Cargo.toml`
- Create: `dot-core/src/lib.rs`
- Create: `dot-core/uniffi-bindgen.rs`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "dot-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["lib", "staticlib"]
name = "dot_core"

[dependencies]
uniffi = { version = "0.30", features = ["cli"] }

[[bin]]
name = "uniffi-bindgen"
path = "uniffi-bindgen.rs"

[profile.release]
lto = "fat"
panic = "abort"
strip = true
```

**Step 2: Create uniffi-bindgen.rs**

```rust
// ABOUTME: Entry point for the UniFFI bindgen CLI tool.
// ABOUTME: Generates Swift bindings from the compiled Rust library.

fn main() {
    uniffi::uniffi_bindgen_main()
}
```

**Step 3: Create src/lib.rs with stub API**

```rust
// ABOUTME: Public API for the dot-core library, exposed to Swift via UniFFI.
// ABOUTME: Provides DOT parsing, validation, and SVG rendering via Graphviz.

uniffi::setup_scaffolding!();

#[derive(uniffi::Enum)]
pub enum LayoutEngine {
    Dot,
    Neato,
    Fdp,
    Circo,
    Twopi,
    Sfdp,
}

#[derive(Debug, uniffi::Error)]
pub enum DotError {
    SyntaxError { message: String, line: u32, column: u32 },
    LayoutError { message: String },
    RenderError { message: String },
}

#[uniffi::export]
pub fn render_dot(dot_source: String, engine: LayoutEngine) -> Result<String, DotError> {
    Err(DotError::RenderError { message: "not yet implemented".to_string() })
}

#[uniffi::export]
pub fn validate_dot(dot_source: String) -> Result<(), DotError> {
    Err(DotError::RenderError { message: "not yet implemented".to_string() })
}
```

**Step 4: Verify it compiles**

Run: `cd dot-core && cargo build`
Expected: Compiles successfully (stubs only, no Graphviz yet)

**Step 5: Generate Swift bindings to verify UniFFI works**

Run: `cargo build && cargo run --bin uniffi-bindgen generate --library target/debug/libdot_core.a --language swift --out-dir generated`
Expected: Creates `generated/dot_core.swift`, `generated/dot_coreFFI.h`, `generated/dot_coreFFI.modulemap`

**Step 6: Commit**

```bash
git add dot-core/
git commit -m "feat: initialize dot-core Rust crate with UniFFI stub API"
```

---

### Task 2: Vendor and Build Graphviz Static Libraries

**Files:**
- Create: `dot-core/build.rs`
- Create: `dot-core/graphviz-vendor/` (git submodule or downloaded source)
- Create: `dot-core/wrapper.h`
- Modify: `dot-core/Cargo.toml` (add build dependencies)

**Step 1: Add build dependencies to Cargo.toml**

Add to `dot-core/Cargo.toml`:

```toml
[build-dependencies]
cmake = "0.1"
bindgen = "0.71"
```

**Step 2: Add Graphviz source as a vendored dependency**

Run: `cd dot-core && git clone --depth 1 --branch 12.2.1 https://gitlab.com/graphviz/graphviz.git graphviz-vendor`

Then remove the `.git` directory so it's vendored, not a submodule:
Run: `rm -rf dot-core/graphviz-vendor/.git`

Add to `.gitignore` at repo root: `dot-core/graphviz-vendor/` (we'll track this separately or download in CI).

**Step 3: Create wrapper.h**

```c
/* ABOUTME: C header wrapper for Graphviz includes. */
/* ABOUTME: Used by bindgen to generate Rust FFI bindings. */

#include <graphviz/gvc.h>
```

**Step 4: Create build.rs**

```rust
// ABOUTME: Build script that compiles vendored Graphviz and generates Rust FFI bindings.
// ABOUTME: Produces static libraries and C bindings via cmake and bindgen.

use std::env;
use std::path::PathBuf;

fn main() {
    let dst = cmake::Config::new("graphviz-vendor")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("enable_ltdl", "OFF")
        .define("with_gvedit", "OFF")
        .define("with_smyrna", "OFF")
        .define("with_expat", "ON")
        .build();

    let lib_dir = dst.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Link Graphviz static libraries (order matters)
    for lib in &[
        "gvplugin_dot_layout",
        "gvplugin_neato_layout",
        "gvplugin_core",
        "gvc",
        "cgraph",
        "cdt",
        "pathplan",
        "xdot",
    ] {
        println!("cargo:rustc-link-lib=static={}", lib);
    }

    // System libraries available on macOS
    println!("cargo:rustc-link-lib=expat");
    println!("cargo:rustc-link-lib=z");

    // Generate Rust bindings from the C headers
    let include_dir = dst.join("include");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", include_dir.display()))
        .allowlist_function("gvContext")
        .allowlist_function("gvContextPlugins")
        .allowlist_function("gvAddLibrary")
        .allowlist_function("gvLayout")
        .allowlist_function("gvFreeLayout")
        .allowlist_function("gvRenderData")
        .allowlist_function("gvFreeRenderData")
        .allowlist_function("gvFreeContext")
        .allowlist_function("agmemread")
        .allowlist_function("agclose")
        .allowlist_function("agerrors")
        .allowlist_function("aglasterr")
        .opaque_type("FILE")
        .generate()
        .expect("Unable to generate Graphviz bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("graphviz_bindings.rs"))
        .expect("Couldn't write bindings");

    // Re-run if graphviz source or wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=graphviz-vendor/");
}
```

**Step 5: Verify the vendored build compiles**

Run: `cd dot-core && cargo build 2>&1 | head -50`
Expected: CMake configures and builds Graphviz, bindgen generates bindings, crate compiles.

Note: This step may require debugging. Graphviz's CMake build is finicky. Common issues:
- Missing system deps (install via `brew install autoconf automake libtool` if needed)
- CMake version requirements
- Plugin library names may differ — check `graphviz-vendor/lib/` output

**Step 6: Commit**

```bash
git add dot-core/build.rs dot-core/wrapper.h dot-core/Cargo.toml .gitignore
git commit -m "feat: add vendored Graphviz build with bindgen FFI bindings"
```

---

### Task 3: Implement Graphviz Rendering in Rust

**Files:**
- Create: `dot-core/src/graphviz.rs`
- Modify: `dot-core/src/lib.rs`

**Step 1: Write a basic test for render_dot**

Add to `dot-core/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_graph() {
        let dot = "digraph { a -> b }".to_string();
        let svg = render_dot(dot, LayoutEngine::Dot).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_render_invalid_dot() {
        let dot = "not a valid dot string {{{".to_string();
        let result = render_dot(dot, LayoutEngine::Dot);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_dot() {
        let dot = "digraph { a -> b }".to_string();
        assert!(validate_dot(dot).is_ok());
    }

    #[test]
    fn test_validate_invalid_dot() {
        let dot = "not valid {{{".to_string();
        assert!(validate_dot(dot).is_err());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cd dot-core && cargo test`
Expected: All 4 tests FAIL (stubs return Err)

**Step 3: Create graphviz.rs with safe wrapper**

```rust
// ABOUTME: Safe Rust wrapper around the Graphviz C library (cgraph + gvc).
// ABOUTME: Handles DOT parsing, layout computation, and SVG rendering.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/graphviz_bindings.rs"));

use std::ffi::{CStr, CString};
use std::ptr;

use crate::{DotError, LayoutEngine};

impl LayoutEngine {
    fn as_c_str(&self) -> &'static [u8] {
        match self {
            LayoutEngine::Dot => b"dot\0",
            LayoutEngine::Neato => b"neato\0",
            LayoutEngine::Fdp => b"fdp\0",
            LayoutEngine::Circo => b"circo\0",
            LayoutEngine::Twopi => b"twopi\0",
            LayoutEngine::Sfdp => b"sfdp\0",
        }
    }
}

/// Render a DOT source string to SVG using the specified layout engine.
pub fn render_to_svg(dot_source: &str, engine: &LayoutEngine) -> Result<String, DotError> {
    let c_source = CString::new(dot_source).map_err(|e| DotError::SyntaxError {
        message: format!("DOT source contains null byte: {}", e),
        line: 0,
        column: 0,
    })?;

    unsafe {
        let gvc = gvContext();
        if gvc.is_null() {
            return Err(DotError::RenderError {
                message: "Failed to create Graphviz context".to_string(),
            });
        }

        let graph = agmemread(c_source.as_ptr());
        if graph.is_null() {
            gvFreeContext(gvc);
            return Err(DotError::SyntaxError {
                message: "Failed to parse DOT source".to_string(),
                line: 0,
                column: 0,
            });
        }

        let engine_cstr = engine.as_c_str();
        let layout_result = gvLayout(gvc, graph, engine_cstr.as_ptr() as *const i8);
        if layout_result != 0 {
            agclose(graph);
            gvFreeContext(gvc);
            return Err(DotError::LayoutError {
                message: format!("Layout failed with code {}", layout_result),
            });
        }

        let mut result_ptr: *mut i8 = ptr::null_mut();
        let mut result_len: u32 = 0;
        let render_result = gvRenderData(
            gvc,
            graph,
            b"svg\0".as_ptr() as *const i8,
            &mut result_ptr as *mut *mut i8 as *mut *mut i8,
            &mut result_len,
        );

        if render_result != 0 || result_ptr.is_null() {
            gvFreeLayout(gvc, graph);
            agclose(graph);
            gvFreeContext(gvc);
            return Err(DotError::RenderError {
                message: format!("SVG render failed with code {}", render_result),
            });
        }

        let svg = CStr::from_ptr(result_ptr)
            .to_string_lossy()
            .into_owned();

        gvFreeRenderData(result_ptr);
        gvFreeLayout(gvc, graph);
        agclose(graph);
        gvFreeContext(gvc);

        Ok(svg)
    }
}

/// Validate DOT syntax by attempting to parse it.
pub fn validate_syntax(dot_source: &str) -> Result<(), DotError> {
    let c_source = CString::new(dot_source).map_err(|e| DotError::SyntaxError {
        message: format!("DOT source contains null byte: {}", e),
        line: 0,
        column: 0,
    })?;

    unsafe {
        let graph = agmemread(c_source.as_ptr());
        if graph.is_null() {
            return Err(DotError::SyntaxError {
                message: "Failed to parse DOT source".to_string(),
                line: 0,
                column: 0,
            });
        }
        agclose(graph);
        Ok(())
    }
}
```

**Step 4: Update lib.rs to use graphviz module**

Replace the stub implementations in `lib.rs`:

```rust
// ABOUTME: Public API for the dot-core library, exposed to Swift via UniFFI.
// ABOUTME: Provides DOT parsing, validation, and SVG rendering via Graphviz.

uniffi::setup_scaffolding!();

mod graphviz;

#[derive(uniffi::Enum)]
pub enum LayoutEngine {
    Dot,
    Neato,
    Fdp,
    Circo,
    Twopi,
    Sfdp,
}

#[derive(Debug, uniffi::Error)]
pub enum DotError {
    SyntaxError { message: String, line: u32, column: u32 },
    LayoutError { message: String },
    RenderError { message: String },
}

#[uniffi::export]
pub fn render_dot(dot_source: String, engine: LayoutEngine) -> Result<String, DotError> {
    graphviz::render_to_svg(&dot_source, &engine)
}

#[uniffi::export]
pub fn validate_dot(dot_source: String) -> Result<(), DotError> {
    graphviz::validate_syntax(&dot_source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_graph() {
        let dot = "digraph { a -> b }".to_string();
        let svg = render_dot(dot, LayoutEngine::Dot).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_render_invalid_dot() {
        let dot = "not a valid dot string {{{".to_string();
        let result = render_dot(dot, LayoutEngine::Dot);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_dot() {
        let dot = "digraph { a -> b }".to_string();
        assert!(validate_dot(dot).is_ok());
    }

    #[test]
    fn test_validate_invalid_dot() {
        let dot = "not valid {{{".to_string();
        assert!(validate_dot(dot).is_err());
    }
}
```

**Step 5: Run tests to verify they pass**

Run: `cd dot-core && cargo test`
Expected: All 4 tests PASS

Note: Static plugin registration may be needed. If `gvLayout` fails because no layout engine is found, add plugin registration calls in `render_to_svg` after `gvContext()`. See Graphviz docs for `gvAddLibrary` with static plugin symbols.

**Step 6: Commit**

```bash
git add dot-core/src/
git commit -m "feat: implement Graphviz rendering via C FFI"
```

---

### Task 4: Build Script and Swift Binding Generation

**Files:**
- Create: `Makefile`
- Create: `scripts/generate-bindings.sh`

**Step 1: Create the binding generation script**

```bash
#!/bin/bash
# ABOUTME: Generates Swift bindings from the compiled dot-core Rust library.
# ABOUTME: Produces .swift, .h, and module.modulemap files for Xcode.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CORE_DIR="$ROOT_DIR/dot-core"
OUT_DIR="$ROOT_DIR/DotViewer/DotViewer/Generated"

cd "$CORE_DIR"

# Build the Rust library as a static lib for macOS
cargo build --release

# Generate Swift bindings from the compiled library
cargo run --bin uniffi-bindgen generate \
  --library "target/release/libdot_core.a" \
  --language swift \
  --out-dir "$OUT_DIR"

# Xcode requires the modulemap to be named module.modulemap
mkdir -p "$OUT_DIR/include"
cp "$OUT_DIR/dot_coreFFI.h" "$OUT_DIR/include/"
cp "$OUT_DIR/dot_coreFFI.modulemap" "$OUT_DIR/include/module.modulemap"

echo "Swift bindings generated in $OUT_DIR"
echo "Static library at: $CORE_DIR/target/release/libdot_core.a"
```

**Step 2: Create the Makefile**

```makefile
# ABOUTME: Top-level build orchestration for the dot-viewer project.
# ABOUTME: Coordinates Rust library build, Swift binding generation, and Xcode build.

.PHONY: all build-core generate-bindings build-app clean

all: build-app

build-core:
	cd dot-core && cargo build --release

generate-bindings: build-core
	bash scripts/generate-bindings.sh

build-app: generate-bindings
	xcodebuild -project DotViewer/DotViewer.xcodeproj \
		-scheme DotViewer \
		-configuration Release \
		build

clean:
	cd dot-core && cargo clean
	rm -rf DotViewer/DotViewer/Generated
```

**Step 3: Run the binding generation**

Run: `chmod +x scripts/generate-bindings.sh && make generate-bindings`
Expected: Static library built, Swift bindings generated in `DotViewer/DotViewer/Generated/`

**Step 4: Commit**

```bash
git add Makefile scripts/
git commit -m "feat: add Makefile and Swift binding generation script"
```

---

### Task 5: Create SwiftUI App Shell with Document Model

**Files:**
- Create: `DotViewer/DotViewer.xcodeproj` (via Xcode or `swift package init`)
- Create: `DotViewer/DotViewer/DotViewerApp.swift`
- Create: `DotViewer/DotViewer/DotDocument.swift`
- Create: `DotViewer/DotViewer/ContentView.swift`
- Create: `DotViewer/DotViewer/Info.plist`

**Step 1: Create the Xcode project structure**

Use Swift Package Manager to bootstrap, or create manually. The project needs:
- macOS deployment target: 14.0
- App Sandbox: YES (with file access)
- Hardened Runtime: YES

**Step 2: Create DotViewerApp.swift**

```swift
// ABOUTME: Main app entry point for the Dot Viewer macOS application.
// ABOUTME: Uses DocumentGroup to support multi-file tabbed editing of .dot files.

import SwiftUI

@main
struct DotViewerApp: App {
    var body: some Scene {
        DocumentGroup(newDocument: DotDocument()) { file in
            ContentView(document: file.$document)
        }
    }
}
```

**Step 3: Create DotDocument.swift**

```swift
// ABOUTME: Document model for .dot files using ReferenceFileDocument.
// ABOUTME: Handles file reading, writing, and change tracking for Graphviz DOT files.

import SwiftUI
import UniformTypeIdentifiers

extension UTType {
    static let dotFile = UTType(exportedAs: "com.2389.dot-viewer.dot",
                                conformingTo: .plainText)
    static let gvFile = UTType(exportedAs: "com.2389.dot-viewer.gv",
                                conformingTo: .plainText)
}

final class DotDocument: ReferenceFileDocument {
    typealias Snapshot = String

    @Published var text: String

    static var readableContentTypes: [UTType] { [.dotFile, .gvFile, .plainText] }
    static var writableContentTypes: [UTType] { [.dotFile, .gvFile, .plainText] }

    init(text: String = "digraph {\n    a -> b\n    b -> c\n}") {
        self.text = text
    }

    init(configuration: ReadConfiguration) throws {
        guard let data = configuration.file.regularFileContents,
              let string = String(data: data, encoding: .utf8) else {
            throw CocoaError(.fileReadCorruptFile)
        }
        self.text = string
    }

    func snapshot(contentType: UTType) throws -> String {
        text
    }

    func fileWrapper(snapshot: String, configuration: WriteConfiguration) throws -> FileWrapper {
        let data = snapshot.data(using: .utf8)!
        return FileWrapper(regularFileWithContents: data)
    }
}
```

**Step 4: Create ContentView.swift (placeholder split pane)**

```swift
// ABOUTME: Main content view with split-pane layout for editing and previewing DOT files.
// ABOUTME: Left pane is the text editor, right pane is the SVG preview.

import SwiftUI

struct ContentView: View {
    @Binding var document: DotDocument

    var body: some View {
        HSplitView {
            // Left: Text editor
            TextEditor(text: $document.text)
                .font(.system(.body, design: .monospaced))
                .frame(minWidth: 300)

            // Right: Preview (placeholder for now)
            Text("SVG Preview")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color(nsColor: .controlBackgroundColor))
                .frame(minWidth: 300)
        }
        .frame(minWidth: 800, minHeight: 500)
    }
}
```

**Step 5: Build and run the app in Xcode**

Open the project in Xcode, build and run. Verify:
- App launches with a new document containing default DOT text
- Split pane shows text editor on left, placeholder on right
- Can open/save .dot files
- Tabs work (Cmd+T opens new document)

**Step 6: Commit**

```bash
git add DotViewer/
git commit -m "feat: create SwiftUI app shell with DocumentGroup and split pane"
```

---

### Task 6: Integrate Rust Library into Xcode Project

**Files:**
- Modify: `DotViewer/DotViewer.xcodeproj` (Xcode build settings)
- Add: Generated Swift bindings to Xcode target
- Add: Static library to link

**Step 1: Run binding generation**

Run: `make generate-bindings`

**Step 2: Configure Xcode project**

In Xcode:
1. Add `DotViewer/DotViewer/Generated/dot_core.swift` to the DotViewer target
2. Add `dot-core/target/release/libdot_core.a` to "Link Binary with Libraries"
3. Add `DotViewer/DotViewer/Generated/include/` to "Header Search Paths"
4. Add `DotViewer/DotViewer/Generated/include/` to "Import Paths" (Swift Compiler settings)
5. Set "Other Linker Flags": `-lexpat -lz`

**Step 3: Create a test Swift file to verify the bridge works**

Add a quick test in ContentView to verify:

```swift
// In ContentView.swift, add to body or onAppear:
.onAppear {
    do {
        let svg = try renderDot(dotSource: "digraph { a -> b }", engine: .dot)
        print("Render succeeded: \(svg.prefix(100))...")
    } catch {
        print("Render failed: \(error)")
    }
}
```

**Step 4: Build and run**

Expected: Console prints SVG output or a meaningful error. If it prints SVG, the bridge is working.

**Step 5: Commit**

```bash
git add DotViewer/ Makefile
git commit -m "feat: integrate dot-core Rust library into Xcode via UniFFI"
```

---

### Task 7: Implement WKWebView SVG Preview

**Files:**
- Create: `DotViewer/DotViewer/PreviewView.swift`
- Create: `DotViewer/DotViewer/Resources/preview.html`
- Modify: `DotViewer/DotViewer/ContentView.swift`

**Step 1: Create preview.html**

```html
<!DOCTYPE html>
<!-- ABOUTME: Minimal HTML shell for displaying SVG output from Graphviz rendering. -->
<!-- ABOUTME: Supports zoom/pan and receives SVG updates via JavaScript injection. -->
<html>
<head>
<meta charset="utf-8">
<style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
        background: #1e1e1e;
        display: flex;
        align-items: center;
        justify-content: center;
        min-height: 100vh;
        overflow: auto;
    }
    #container {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 20px;
    }
    #container svg {
        max-width: 100%;
        height: auto;
    }
    #error {
        color: #ff6b6b;
        font-family: -apple-system, sans-serif;
        font-size: 14px;
        padding: 20px;
        white-space: pre-wrap;
        display: none;
    }
</style>
</head>
<body>
    <div id="container"></div>
    <div id="error"></div>
    <script>
        function updateSVG(svgString) {
            const container = document.getElementById('container');
            const error = document.getElementById('error');
            container.innerHTML = svgString;
            container.style.display = 'flex';
            error.style.display = 'none';
        }

        function showError(message) {
            const container = document.getElementById('container');
            const error = document.getElementById('error');
            error.textContent = message;
            error.style.display = 'block';
            // Keep showing last valid SVG in container
        }
    </script>
</body>
</html>
```

**Step 2: Create PreviewView.swift**

```swift
// ABOUTME: WKWebView wrapper that displays SVG output from Graphviz rendering.
// ABOUTME: Receives SVG strings and injects them into a minimal HTML shell.

import SwiftUI
import WebKit

struct PreviewView: NSViewRepresentable {
    let svgContent: String
    let errorMessage: String?

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        let webView = WKWebView(frame: .zero, configuration: config)
        webView.setValue(false, forKey: "drawsBackground")

        if let htmlURL = Bundle.main.url(forResource: "preview", withExtension: "html") {
            webView.loadFileURL(htmlURL, allowingReadAccessTo: htmlURL.deletingLastPathComponent())
        }

        context.coordinator.webView = webView
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        context.coordinator.updateContent(svg: svgContent, error: errorMessage)
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    class Coordinator: NSObject, WKNavigationDelegate {
        var webView: WKWebView?
        private var pendingSVG: String?
        private var pendingError: String?
        private var isLoaded = false

        func updateContent(svg: String, error: String?) {
            guard isLoaded, let webView else {
                pendingSVG = svg
                pendingError = error
                webView?.navigationDelegate = self
                return
            }

            if let error {
                let escaped = error.replacingOccurrences(of: "\\", with: "\\\\")
                    .replacingOccurrences(of: "'", with: "\\'")
                    .replacingOccurrences(of: "\n", with: "\\n")
                webView.evaluateJavaScript("showError('\(escaped)')")
            } else if !svg.isEmpty {
                let escaped = svg.replacingOccurrences(of: "\\", with: "\\\\")
                    .replacingOccurrences(of: "'", with: "\\'")
                    .replacingOccurrences(of: "\n", with: "\\n")
                webView.evaluateJavaScript("updateSVG('\(escaped)')")
            }
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            isLoaded = true
            if let svg = pendingSVG {
                updateContent(svg: svg, error: pendingError)
                pendingSVG = nil
                pendingError = nil
            }
        }
    }
}
```

**Step 3: Update ContentView.swift to use PreviewView with rendering**

```swift
// ABOUTME: Main content view with split-pane layout for editing and previewing DOT files.
// ABOUTME: Left pane is the text editor, right pane is the live SVG preview.

import SwiftUI

struct ContentView: View {
    @Binding var document: DotDocument
    @State private var svgOutput: String = ""
    @State private var errorMessage: String?
    @State private var selectedEngine: LayoutEngine = .dot
    @State private var liveMode: Bool = true
    @State private var renderTask: Task<Void, Never>?

    var body: some View {
        HSplitView {
            TextEditor(text: $document.text)
                .font(.system(.body, design: .monospaced))
                .frame(minWidth: 300)
                .onChange(of: document.text) {
                    if liveMode {
                        scheduleRender()
                    }
                }

            PreviewView(svgContent: svgOutput, errorMessage: errorMessage)
                .frame(minWidth: 300)
        }
        .frame(minWidth: 800, minHeight: 500)
        .toolbar {
            ToolbarItem {
                Picker("Engine", selection: $selectedEngine) {
                    Text("dot").tag(LayoutEngine.dot)
                    Text("neato").tag(LayoutEngine.neato)
                    Text("fdp").tag(LayoutEngine.fdp)
                    Text("circo").tag(LayoutEngine.circo)
                    Text("twopi").tag(LayoutEngine.twopi)
                    Text("sfdp").tag(LayoutEngine.sfdp)
                }
                .frame(width: 100)
            }
            ToolbarItem {
                Toggle("Live", isOn: $liveMode)
                    .toggleStyle(.switch)
            }
            ToolbarItem {
                Button("Refresh") {
                    performRender()
                }
                .keyboardShortcut("r", modifiers: .command)
            }
        }
        .onChange(of: selectedEngine) {
            performRender()
        }
        .onAppear {
            performRender()
        }
    }

    private func scheduleRender() {
        renderTask?.cancel()
        renderTask = Task {
            try? await Task.sleep(nanoseconds: 300_000_000) // 300ms debounce
            if !Task.isCancelled {
                performRender()
            }
        }
    }

    private func performRender() {
        let source = document.text
        let engine = selectedEngine
        Task.detached {
            do {
                let svg = try renderDot(dotSource: source, engine: engine)
                await MainActor.run {
                    svgOutput = svg
                    errorMessage = nil
                }
            } catch {
                await MainActor.run {
                    errorMessage = "\(error)"
                    // Keep showing last valid SVG
                }
            }
        }
    }
}
```

**Step 4: Build and run**

Expected: App shows split pane with text editor and live SVG preview. Typing updates the preview after 300ms. Invalid DOT shows error while keeping last valid render.

**Step 5: Commit**

```bash
git add DotViewer/
git commit -m "feat: add WKWebView SVG preview with live rendering"
```

---

### Task 8: Add Basic DOT Syntax Highlighting

**Files:**
- Create: `DotViewer/DotViewer/EditorView.swift`
- Modify: `DotViewer/DotViewer/ContentView.swift`

**Step 1: Create EditorView.swift with NSTextView wrapper**

```swift
// ABOUTME: NSTextView wrapper providing a code editor for DOT files.
// ABOUTME: Supports basic syntax highlighting for DOT keywords, strings, and comments.

import SwiftUI
import AppKit

struct EditorView: NSViewRepresentable {
    @Binding var text: String

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSTextView.scrollableTextView()
        let textView = scrollView.documentView as! NSTextView

        textView.font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticTextReplacementEnabled = false
        textView.isRichText = false
        textView.allowsUndo = true
        textView.usesFindPanel = true

        textView.delegate = context.coordinator
        textView.string = text
        context.coordinator.applyHighlighting(to: textView)

        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        let textView = scrollView.documentView as! NSTextView
        if textView.string != text {
            textView.string = text
            context.coordinator.applyHighlighting(to: textView)
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text)
    }

    class Coordinator: NSObject, NSTextViewDelegate {
        var text: Binding<String>

        // DOT keywords
        private let keywords = [
            "digraph", "graph", "subgraph", "node", "edge", "strict"
        ]

        // DOT attribute names (common ones)
        private let attributes = [
            "label", "color", "fillcolor", "style", "shape", "fontname",
            "fontsize", "fontcolor", "bgcolor", "rankdir", "rank",
            "dir", "weight", "penwidth", "arrowhead", "arrowtail",
            "width", "height", "fixedsize", "pos", "xlabel",
        ]

        init(text: Binding<String>) {
            self.text = text
        }

        func textDidChange(_ notification: Notification) {
            guard let textView = notification.object as? NSTextView else { return }
            text.wrappedValue = textView.string
            applyHighlighting(to: textView)
        }

        func applyHighlighting(to textView: NSTextView) {
            let text = textView.string
            let fullRange = NSRange(location: 0, length: (text as NSString).length)
            let storage = textView.textStorage!

            // Default color
            storage.addAttribute(.foregroundColor,
                                 value: NSColor.textColor,
                                 range: fullRange)

            // Keywords
            for keyword in keywords {
                let pattern = "\\b\(keyword)\\b"
                if let regex = try? NSRegularExpression(pattern: pattern) {
                    let matches = regex.matches(in: text, range: fullRange)
                    for match in matches {
                        storage.addAttribute(.foregroundColor,
                                             value: NSColor.systemPurple,
                                             range: match.range)
                    }
                }
            }

            // Attribute names (word before =)
            if let regex = try? NSRegularExpression(pattern: "\\b(\\w+)\\s*=") {
                let matches = regex.matches(in: text, range: fullRange)
                for match in matches {
                    storage.addAttribute(.foregroundColor,
                                         value: NSColor.systemBlue,
                                         range: match.range(at: 1))
                }
            }

            // Strings (double-quoted)
            if let regex = try? NSRegularExpression(pattern: "\"[^\"]*\"") {
                let matches = regex.matches(in: text, range: fullRange)
                for match in matches {
                    storage.addAttribute(.foregroundColor,
                                         value: NSColor.systemGreen,
                                         range: match.range)
                }
            }

            // Comments (// and /* */)
            if let regex = try? NSRegularExpression(pattern: "//[^\n]*") {
                let matches = regex.matches(in: text, range: fullRange)
                for match in matches {
                    storage.addAttribute(.foregroundColor,
                                         value: NSColor.systemGray,
                                         range: match.range)
                }
            }
            if let regex = try? NSRegularExpression(pattern: "/\\*.*?\\*/",
                                                      options: .dotMatchesLineSeparators) {
                let matches = regex.matches(in: text, range: fullRange)
                for match in matches {
                    storage.addAttribute(.foregroundColor,
                                         value: NSColor.systemGray,
                                         range: match.range)
                }
            }

            // Arrow operators
            if let regex = try? NSRegularExpression(pattern: "->|--") {
                let matches = regex.matches(in: text, range: fullRange)
                for match in matches {
                    storage.addAttribute(.foregroundColor,
                                         value: NSColor.systemOrange,
                                         range: match.range)
                }
            }
        }
    }
}
```

**Step 2: Update ContentView to use EditorView instead of TextEditor**

Replace `TextEditor(text: $document.text)` with `EditorView(text: $document.text)`.

**Step 3: Build and run**

Expected: Editor shows DOT keywords in purple, strings in green, comments in gray, attributes in blue, arrows in orange.

**Step 4: Commit**

```bash
git add DotViewer/
git commit -m "feat: add DOT syntax highlighting via NSTextView"
```

---

### Task 9: Add Error Display in Editor

**Files:**
- Modify: `DotViewer/DotViewer/ContentView.swift`

**Step 1: Add error banner below editor**

Update the left side of the HSplitView in ContentView:

```swift
VStack(spacing: 0) {
    EditorView(text: $document.text)

    if let errorMessage {
        HStack {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundColor(.red)
            Text(errorMessage)
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(.red)
                .lineLimit(2)
            Spacer()
        }
        .padding(8)
        .background(Color.red.opacity(0.1))
    }
}
.frame(minWidth: 300)
```

**Step 2: Build and run**

Expected: When DOT is invalid, a red error banner appears below the editor. When DOT is valid, it disappears.

**Step 3: Commit**

```bash
git add DotViewer/
git commit -m "feat: add inline error banner for DOT syntax errors"
```

---

### Task 10: File Type Registration and Polish

**Files:**
- Modify: `DotViewer/DotViewer/Info.plist`
- Create: `DotViewer/DotViewer/Assets.xcassets` (app icon if desired)

**Step 1: Register .dot and .gv file types in Info.plist**

Add UTType declarations so macOS associates .dot and .gv files with the app:

```xml
<key>UTImportedTypeDeclarations</key>
<array>
    <dict>
        <key>UTTypeIdentifier</key>
        <string>com.2389.dot-viewer.dot</string>
        <key>UTTypeDescription</key>
        <string>Graphviz DOT File</string>
        <key>UTTypeConformsTo</key>
        <array>
            <string>public.plain-text</string>
        </array>
        <key>UTTypeTagSpecification</key>
        <dict>
            <key>public.filename-extension</key>
            <array>
                <string>dot</string>
            </array>
        </dict>
    </dict>
    <dict>
        <key>UTTypeIdentifier</key>
        <string>com.2389.dot-viewer.gv</string>
        <key>UTTypeDescription</key>
        <string>Graphviz GV File</string>
        <key>UTTypeConformsTo</key>
        <array>
            <string>public.plain-text</string>
        </array>
        <key>UTTypeTagSpecification</key>
        <dict>
            <key>public.filename-extension</key>
            <array>
                <string>gv</string>
            </array>
        </dict>
    </dict>
</array>
```

**Step 2: Add keyboard shortcuts**

Ensure these work:
- Cmd+S: Save (handled by DocumentGroup)
- Cmd+R: Refresh preview (already added in toolbar)
- Cmd+T: New tab (handled by macOS)

**Step 3: Test end-to-end**

1. Launch app — default graph renders in preview
2. Edit text — preview updates after 300ms
3. Toggle live mode off — preview only updates on Cmd+R
4. Switch layout engines — preview re-renders
5. Open a .dot file from Finder — opens in app
6. Cmd+T — new tab with default document
7. Break the DOT syntax — error banner appears, last valid SVG stays

**Step 4: Commit**

```bash
git add DotViewer/
git commit -m "feat: register .dot/.gv file types and finalize v1"
```
