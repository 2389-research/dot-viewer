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
