#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST_PATH="$ROOT_DIR/editor/Cargo.toml"
WINDOWS_TARGET="x86_64-pc-windows-gnu"

echo "Building Rust CLI and GUI for macOS (native)..."
cargo build --manifest-path "$MANIFEST_PATH" --bin cosam-convert --bin cosam-editor

echo "Checking Windows cross-compile prerequisites..."
if ! rustup target list --installed | grep -q "^${WINDOWS_TARGET}$"; then
    echo "Missing Rust target ${WINDOWS_TARGET}."
    echo "Install with: rustup target add ${WINDOWS_TARGET}"
    exit 1
fi

if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    echo "Missing MinGW linker: x86_64-w64-mingw32-gcc"
    echo "On macOS with MacPorts: sudo port install mingw-w64"
    exit 1
fi

echo "Building Rust CLI and GUI for Windows (${WINDOWS_TARGET})..."
cargo build --manifest-path "$MANIFEST_PATH" --target "$WINDOWS_TARGET" --bin cosam-convert --bin cosam-editor

echo "Build complete."
