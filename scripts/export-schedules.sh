#!/bin/bash

# Helper script to export all schedule files
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
#
# NOTE: When updating this script, also update export-schedules.ps1 to maintain parity
#
# Usage: scripts/export-schedules.sh
#   Reads from input/<YEAR> Schedule.xlsx
#   Writes to output/<YEAR>/{schedule.xlsx,public.json,private.json,embed.html,test.html,style-embed.html,style-page.html}
#   Also generates PDFs to output/<YEAR>/ via schedule-layout (built into cosam-convert)

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
INPUT_DIR="$ROOT_DIR/input"
OUTPUT_DIR="$ROOT_DIR/output"

echo "Rebuilding schedule output files..."
echo "Script directory: $SCRIPT_DIR"
echo "Input directory:  $INPUT_DIR"
echo "Output directory: $OUTPUT_DIR"
echo ""

mkdir -p "$OUTPUT_DIR"

# Build cosam-convert (schedule-layout is linked in via the 'layout' feature)
echo "Building cosam-convert..."
cd "$ROOT_DIR"
cargo build -p cosam-convert --release
CONVERT_BIN="$ROOT_DIR/target/release/cosam-convert"

declare -a built=()
declare -a failed=()
declare -a conflict_years=()

echo ""
echo "Validating all schedules..."
for year in $(seq 2016 "$(date +%Y)"); do
    src="$INPUT_DIR/${year} Schedule.xlsx"
    if [ ! -f "$src" ]; then
        echo "  Skipping ${year} - file not found"
        continue
    fi

    echo "  Validating ${year}..."
    if ! "$CONVERT_BIN" --input "$src" --check >/dev/null 2>&1; then
        echo "    ${year} has conflicts"
        conflict_years+=("$year")
    else
        echo "    ${year} - OK"
    fi
done

if [ ${#conflict_years[@]} -gt 0 ]; then
    echo ""
    echo "Schedules with conflicts: ${conflict_years[*]}"
    echo ""
fi

echo "Building all output files..."

for year in $(seq 2016 "$(date +%Y)"); do
    year_dir="$OUTPUT_DIR/$year"
    mkdir -p "$year_dir"
    src="$INPUT_DIR/${year} Schedule.xlsx"

    if [ ! -f "$src" ]; then
        echo "  Skipping ${year} - file not found"
        continue
    fi

    # Output paths for this year
    copy="$year_dir/schedule.xlsx"
    dest="$year_dir/public.json"
    private_dest="$year_dir/private.json"
    embed="$year_dir/embed.html"
    test_html="$year_dir/test.html"
    style_embed="$year_dir/style-embed.html"
    style_page="$year_dir/style-page.html"
    layout_dir="$year_dir/layout"

    echo "  Building ${year} files..."

    if "$CONVERT_BIN" \
        --input "$src" \
        --title "Cosplay America ${year} Schedule" \
        --output "$copy" \
        --export "$dest" \
        --private \
        --export "$private_dest" \
        --public \
        --export-embed "$embed" \
        --export-test "$test_html" \
        --style-page \
        --export-embed "$style_embed" \
        --export-test "$style_page" \
        --export-layout "$layout_dir"; then
        built+=("$copy" "$dest" "$private_dest" "$embed" "$test_html" "$style_embed" "$style_page" "$layout_dir")
    else
        failed+=("$copy" "$dest" "$private_dest" "$embed" "$test_html" "$style_embed" "$style_page" "$layout_dir")
    fi
done

echo ""
echo "Done!"
echo ""

if [ ${#built[@]} -gt 0 ]; then
    echo "Files built:"
    for file in "${built[@]}"; do
        echo "  - $file"
    done
fi

if [ ${#conflict_years[@]} -gt 0 ]; then
    echo ""
    echo "Schedules with conflicts (still exported):"
    for year in "${conflict_years[@]}"; do
        echo "  - ${year}"
    done
fi

if [ ${#failed[@]} -gt 0 ]; then
    echo ""
    echo "Files that failed to build:"
    for file in "${failed[@]}"; do
        echo "  - $file"
    done
    exit 10
fi
