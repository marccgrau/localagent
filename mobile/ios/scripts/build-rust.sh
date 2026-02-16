#!/bin/bash
#
# Build the localgpt-mobile Rust library for iOS targets and create an XCFramework.
#
# Usage:
#   ./build-rust.sh              # Release build (default)
#   ./build-rust.sh debug        # Debug build
#
# Prerequisites:
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim
#   cargo install uniffi-bindgen-cli --version 0.29.5

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
MOBILE_CRATE="$REPO_ROOT/crates/mobile"
TARGET_DIR="$REPO_ROOT/target"
OUT_DIR="$SCRIPT_DIR/../Generated"

PROFILE="${1:-release}"
if [ "$PROFILE" = "release" ]; then
    CARGO_FLAG="--release"
    PROFILE_DIR="release"
else
    CARGO_FLAG=""
    PROFILE_DIR="debug"
fi

TARGETS="aarch64-apple-ios aarch64-apple-ios-sim"

echo "==> Building localgpt-mobile for iOS ($PROFILE)..."

for TARGET in $TARGETS; do
    echo "    Building for $TARGET..."
    cargo build -p localgpt-mobile --target "$TARGET" $CARGO_FLAG
done

echo "==> Generating Swift bindings..."
mkdir -p "$OUT_DIR"

# Generate Swift bindings from the compiled library
uniffi-bindgen generate \
    --library "$TARGET_DIR/aarch64-apple-ios/$PROFILE_DIR/liblocalgpt_mobile.a" \
    --language swift \
    --out-dir "$OUT_DIR"

echo "==> Creating XCFramework..."

XCFRAMEWORK="$SCRIPT_DIR/../LocalGPTCore.xcframework"
rm -rf "$XCFRAMEWORK"

xcodebuild -create-xcframework \
    -library "$TARGET_DIR/aarch64-apple-ios/$PROFILE_DIR/liblocalgpt_mobile.a" \
    -library "$TARGET_DIR/aarch64-apple-ios-sim/$PROFILE_DIR/liblocalgpt_mobile.a" \
    -output "$XCFRAMEWORK"

echo "==> Done!"
echo "    XCFramework: $XCFRAMEWORK"
echo "    Swift bindings: $OUT_DIR/"
echo ""
echo "Add the XCFramework to your Xcode project and import the generated Swift files."
