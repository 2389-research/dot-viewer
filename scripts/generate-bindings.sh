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

# Generate Swift bindings from the compiled library.
# This produces files for both dot-core and dot-parser since
# dot-parser types are statically linked into libdot_core.a.
cargo run --bin uniffi-bindgen generate \
  --library "target/release/libdot_core.a" \
  --language swift \
  --out-dir "$OUT_DIR"

# Combine both FFI headers and modulemaps into a single module for Xcode.
# The uniffi-bindgen generates separate files for each crate (dot_core, dot_parser).
mkdir -p "$OUT_DIR/include"
cp "$OUT_DIR/dot_coreFFI.h" "$OUT_DIR/include/"
cp "$OUT_DIR/dot_parserFFI.h" "$OUT_DIR/include/"
cat > "$OUT_DIR/include/module.modulemap" <<'MODULEMAP'
module dot_coreFFI {
    header "dot_coreFFI.h"
    export *
    use "Darwin"
    use "_Builtin_stdbool"
    use "_Builtin_stdint"
}
module dot_parserFFI {
    header "dot_parserFFI.h"
    export *
    use "Darwin"
    use "_Builtin_stdbool"
    use "_Builtin_stdint"
}
MODULEMAP

echo "Swift bindings generated in $OUT_DIR"
echo "Static library at: $CORE_DIR/target/release/libdot_core.a"
