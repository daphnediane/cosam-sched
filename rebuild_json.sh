#!/bin/bash

# Helper script to rebuild all JSON files for testing
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EDITOR_DIR="$SCRIPT_DIR/editor"
INPUT_DIR="$SCRIPT_DIR/input"

echo "Rebuilding JSON files for testing..."
echo "Script directory: $SCRIPT_DIR"
echo "Editor directory: $EDITOR_DIR"
echo "Input directory: $INPUT_DIR"
echo ""

declare -a built=()

for year in $(seq 2016 $(date +%Y)); do
    src="$INPUT_DIR/${year} Schedule.xlsx"
    if [ ! -f "$src" ]; then
        echo "Skipping ${year} - file not found"
        continue
    fi

    # Build files for this year
    echo "Building ${year} files..."
    cd "$EDITOR_DIR"

    echo "  Building ${year}.json with Perl converter..."
    ../converter/schedule_to_json --input "$src" --output ../widget/${year}.json --title "Cosplay America ${year} Schedule" &&
        built+=("${year}.json (Perl converter)") ||
        built+=("${year}.json (Perl converter) - FAILED")

    echo "  Building ${year}-editor.json with Rust converter CLI..."
    cargo run --bin cosam-convert -- --input "$src" --output ../widget/${year}-editor.json --title "Cosplay America ${year} Schedule" &&
        built+=("${year}-editor.json (Rust converter CLI)") ||
        built+=("${year}-editor.json (Rust converter CLI) - FAILED")

done

echo "All JSON files rebuilt successfully!"
echo ""
echo "Files created:"
for file in "${built[@]}"; do
    echo "  - ../widget/$file"
done
