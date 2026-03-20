#!/bin/bash

# Helper script to rebuild all JSON files for testing
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
INPUT_DIR="$ROOT_DIR/input"
OUTPUT_DIR="$ROOT_DIR/output"

echo "Rebuilding JSON files for testing..."
echo "Script directory: $SCRIPT_DIR"
echo "Input directory: $INPUT_DIR"
echo "Output directory: $OUTPUT_DIR"
echo ""

mkdir -p "$OUTPUT_DIR"

declare -a built=()

for year in $(seq 2016 $(date +%Y)); do
    src="$INPUT_DIR/${year} Schedule.xlsx"
    dest="$OUTPUT_DIR/${year}.json"
    embed="$OUTPUT_DIR/${year}-embed.html"
    test_html="$OUTPUT_DIR/${year}-test.html"
    if [ ! -f "$src" ]; then
        echo "Skipping ${year} - file not found"
        continue
    fi

    # Build files for this year
    echo "Building ${year} files..."
    cd "$ROOT_DIR"

    echo "  Building ${year}.json, embed, and test page..."
    cargo run -p cosam-convert -- \
        --input "$src" \
        --export "$dest" \
        --export-embed "$embed" \
        --export-test "$test_html" \
        --title "Cosplay America ${year} Schedule" &&
        built+=("${year} (json + embed + test)") ||
        built+=("${year} - FAILED")

done

echo "All JSON files rebuilt successfully!"
echo ""
echo "Files created:"
for file in "${built[@]}"; do
    echo "  - $file"
done
