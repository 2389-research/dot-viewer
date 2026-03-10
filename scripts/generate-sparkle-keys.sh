#!/usr/bin/env bash
# ABOUTME: Generates a Sparkle EdDSA keypair for signing appcasts.
# ABOUTME: Run once locally, then store keys as GitHub Actions secrets.

set -euo pipefail

ACCOUNT_NAME="${1:-2389-research}"
WORK_DIR="$(mktemp -d)"
SPARKLE_SOURCE="$WORK_DIR/Sparkle"
SPARKLE_BUILD="$WORK_DIR/build"
PRIVATE_KEY_FILE="$WORK_DIR/sparkle-private-key"

cleanup() {
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

echo "Cloning Sparkle..."
git clone --depth 1 https://github.com/sparkle-project/Sparkle.git "$SPARKLE_SOURCE" >/dev/null

echo "Building generate_keys tool..."
xcodebuild \
    -project "$SPARKLE_SOURCE/Sparkle.xcodeproj" \
    -scheme generate_keys \
    -configuration Release \
    -derivedDataPath "$SPARKLE_BUILD" \
    build >/dev/null

GENERATE_KEYS="$SPARKLE_BUILD/Build/Products/Release/generate_keys"

"$GENERATE_KEYS" --account "$ACCOUNT_NAME" >/dev/null
PUBLIC_KEY="$("$GENERATE_KEYS" --account "$ACCOUNT_NAME" -p)"
"$GENERATE_KEYS" --account "$ACCOUNT_NAME" -x "$PRIVATE_KEY_FILE" >/dev/null
PRIVATE_KEY="$(cat "$PRIVATE_KEY_FILE")"

cat <<EOF

Sparkle EdDSA keys for account: $ACCOUNT_NAME

--- Add these as GitHub Actions secrets ---

SPARKLE_PUBLIC_ED_KEY:
$PUBLIC_KEY

SPARKLE_PRIVATE_ED_KEY:
$PRIVATE_KEY

--- Also add SPARKLE_PUBLIC_ED_KEY to project.yml build settings for local testing ---
EOF
