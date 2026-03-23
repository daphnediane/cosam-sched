#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Windows targets to try
WINDOWS_TARGETS=("x86_64-pc-windows-msvc" "aarch64-pc-windows-msvc")

echo "Building Rust CLI and GUI for native platform..."
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p cosam-convert -p cosam-editor -p cosam-modify

echo "Checking Windows cross-compile prerequisites..."
for target in "${WINDOWS_TARGETS[@]}"; do
    echo "Testing Windows target: $target"

    if ! rustup target list --installed | grep -q "^${target}$"; then
        echo "Missing Rust target ${target}."
        echo "Install with: rustup target add ${target}"
        echo "Skipping ${target}..."
        continue
    fi

    # Check for appropriate linker (this is a basic check - actual setup may vary)
    case $target in
    "x86_64-pc-windows-msvc")
        # MSVC target - check for MSVC toolchain
        if ! command -v cl >/dev/null 2>&1 && ! command -v clang-cl >/dev/null 2>&1; then
            echo "Missing MSVC linker (cl.exe or clang-cl.exe)"
            echo "Install Visual Studio Build Tools or LLVM with clang-cl"
            echo "Skipping ${target}..."
            continue
        fi
        ;;
    "aarch64-pc-windows-msvc")
        # ARM64 MSVC target - check for MSVC toolchain with ARM64 support
        if ! command -v cl >/dev/null 2>&1 && ! command -v clang-cl >/dev/null 2>&1; then
            echo "Missing MSVC linker (cl.exe or clang-cl.exe) with ARM64 support"
            echo "Install Visual Studio Build Tools or LLVM with clang-cl"
            echo "Skipping ${target}..."
            continue
        fi
        ;;
    esac

    echo "Building Rust CLI and GUI for Windows (${target})..."
    if cargo build --manifest-path "$ROOT_DIR/Cargo.toml" --target "$target" -p cosam-convert -p cosam-editor -p cosam-modify; then
        echo "Successfully built for ${target}"
    else
        echo "Failed to build for ${target}, continuing..."
    fi
done

echo "Build complete."
