#!/bin/bash

# Helper script to rebuild all JSON files for testing
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EDITOR_DIR="$SCRIPT_DIR/editor"

echo "Rebuilding JSON files for testing..."
echo "Script directory: $SCRIPT_DIR"
echo "Editor directory: $EDITOR_DIR"
echo ""

declare -a built=()
# Build both years
for year in 2025 2026; do

    # Build files for this year
    echo "Building ${year} files..."
    cd "$EDITOR_DIR"

    echo "  Building ${year}.json with Perl converter..."
    ../converter/schedule_to_json --input "../input/${year} Schedule.xlsx" --output ../widget/${year}.json --title "Cosplay America ${year} Schedule"
    built+=("${year}.json (Perl converter)")

    echo "  Building ${year}-editor.json with Rust editor..."
    cargo run -- --input "../input/${year} Schedule.xlsx" --output ../widget/${year}-editor.json --title "Cosplay America ${year} Schedule"
    built+=("${year}-editor.json (Rust editor)")

done

echo "All JSON files rebuilt successfully!"
echo ""
echo "Files created:"
for file in "${built[@]}"; do
    echo "  - ../widget/$file"
done
