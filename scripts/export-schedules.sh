#!/bin/bash

# Helper script to export all schedule files
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

# Build cosam-convert once at the start
echo "Building cosam-convert..."
cd "$ROOT_DIR"
cargo build -p cosam-convert --release
CONVERT_BIN="$ROOT_DIR/target/release/cosam-convert"

declare -a built=()
declare -a failed=()

for year in $(seq 2016 $(date +%Y)); do
    year_dir="$OUTPUT_DIR/$year"
    mkdir -p "$year_dir"
    src="$INPUT_DIR/${year} Schedule.xlsx"
    dest="$year_dir/public.json"
    embed="$year_dir/embed.html"
    test_html="$year_dir/test.html"
    style_page="$year_dir/style-page.html"
    style_embed="$year_dir/style-embed.html"
    if [ ! -f "$src" ]; then
        echo "Skipping ${year} - file not found"
        continue
    fi

    # Build files for this year
    echo "Building ${year} files..."

    echo "  Building ${year}.json, embed, and test page..."
    "$CONVERT_BIN" \
        --input "$src" \
        --export "$dest" \
        --export-embed "$embed" \
        --export-test "$test_html" \
        --title "Cosplay America ${year} Schedule" &&
        built+=("$dest" "$embed" "$test_html") ||
        failed+=("$dest" "$embed" "$test_html")
    echo "  Building ${year} style page..."
    "$CONVERT_BIN" \
        --input "$src" \
        --export-test "$style_page" \
        --export-embed "$style_embed" \
        --style-page \
        --title "Cosplay America ${year} Schedule" &&
        built+=("$style_page" "$style_embed") ||
        failed+=("$style_page" "$style_embed")
done

echo "All JSON files rebuilt successfully!"
echo ""
echo "Files created:"
for file in "${built[@]}"; do
    echo "  - $file"
done

if [ ${#failed[@]} -gt 0 ]; then
    echo ""
    echo "Files failed:"
    for file in "${failed[@]}"; do
        echo "  - $file"
    done
    exit 10
fi
