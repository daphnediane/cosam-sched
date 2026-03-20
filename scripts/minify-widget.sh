#!/bin/bash

# CosAm Calendar Widget Minification Script
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause

set -euo pipefail

# Parse arguments
SCHEDULE_JSON_FILE=""
OUTPUT_FILE="cosam-mini-embed.html"

while [[ $# -gt 0 ]]; do
    case $1 in
    --json | -j)
        SCHEDULE_JSON_FILE="$2"
        shift 2
        ;;
    --output | -o)
        OUTPUT_FILE="$2"
        shift 2
        ;;
    --help | -h)
        echo "Usage: $0 [--json schedule.json] [--output output.html]"
        echo "  --json: Path to schedule JSON file to embed"
        echo "  --output: Output HTML file (default: cosam-mini-embed.html)"
        exit 0
        ;;
    *)
        echo "Unknown option: $1"
        exit 1
        ;;
    esac
done

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WIDGET_DIR="$PROJECT_ROOT/widget"
CSS_FILE="$WIDGET_DIR/cosam-calendar.css"
JS_FILE="$WIDGET_DIR/cosam-calendar.js"
FINAL_OUTPUT="$WIDGET_DIR/$OUTPUT_FILE"

# Node modules binaries
NPM_BIN="$PROJECT_ROOT/node_modules/.bin"
TERSER="$NPM_BIN/terser"
POSTCSS="$NPM_BIN/postcss"

# Create temporary directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf '$TEMP_DIR'" EXIT

echo "🔧 CosAm Widget Minifier"
echo "========================"

# Check if source files exist
if [[ ! -f "$CSS_FILE" ]]; then
    echo "❌ Error: CSS file not found: $CSS_FILE"
    exit 1
fi

if [[ ! -f "$JS_FILE" ]]; then
    echo "❌ Error: JS file not found: $JS_FILE"
    exit 1
fi

# Check if tools exist
if [[ ! -f "$TERSER" ]]; then
    echo "❌ Error: terser not found. Run 'npm install' first."
    exit 1
fi

if [[ ! -f "$POSTCSS" ]]; then
    echo "❌ Error: postcss not found. Run 'npm install' first."
    exit 1
fi

# Check JSON file if provided
if [[ -n "$SCHEDULE_JSON_FILE" && ! -f "$SCHEDULE_JSON_FILE" ]]; then
    echo "❌ Error: JSON file not found: $SCHEDULE_JSON_FILE"
    exit 1
fi

echo "📁 Source files:"
echo "   CSS: $CSS_FILE"
echo "   JS:  $JS_FILE"
if [[ -n "$SCHEDULE_JSON_FILE" ]]; then
    echo "   JSON: $SCHEDULE_JSON_FILE"
fi
echo ""

# Function to format bytes (check for numfmt/gnumfmt first)
format_bytes() {
    local bytes=$1

    # Try numfmt (GNU coreutils)
    if command -v numfmt >/dev/null 2>&1; then
        numfmt --to=iec "$bytes"
        return
    fi

    # Try gnumfmt (macOS with GNU coreutils)
    if command -v gnumfmt >/dev/null 2>&1; then
        gnumfmt --to=iec "$bytes"
        return
    fi

    # Fallback to simple formatting
    if [[ $bytes -lt 1024 ]]; then
        echo "${bytes}B"
    elif [[ $bytes -lt 1048576 ]]; then
        echo "$((bytes / 1024))KB"
    elif [[ $bytes -lt 1073741824 ]]; then
        echo "$((bytes / 1048576))MB"
    else
        echo "$((bytes / 1073741824))GB"
    fi
}

# Get file sizes
CSS_SIZE=$(stat -f%z "$CSS_FILE")
JS_SIZE=$(stat -f%z "$JS_FILE")
TOTAL_SIZE=$((CSS_SIZE + JS_SIZE))

if [[ -n "$SCHEDULE_JSON_FILE" ]]; then
    JSON_SIZE=$(stat -f%z "$SCHEDULE_JSON_FILE")
    TOTAL_SIZE=$((TOTAL_SIZE + JSON_SIZE))
fi

echo "📊 Original sizes:"
echo "   CSS: $(format_bytes $CSS_SIZE)"
echo "   JS:  $(format_bytes $JS_SIZE)"
if [[ -n "$SCHEDULE_JSON_FILE" ]]; then
    echo "   JSON: $(format_bytes $JSON_SIZE)"
fi
echo "   Total: $(format_bytes $TOTAL_SIZE)"
echo ""

# Minify CSS
echo "🎨 Minifying CSS..."
TEMP_CSS="$TEMP_DIR/cosam-calendar.min.css"

# Create postcss config for cssnano
cat >"$TEMP_DIR/postcss.config.js" <<EOF
const cssnano = require('${PROJECT_ROOT}/node_modules/cssnano');

module.exports = {
  plugins: [
    cssnano({
      preset: ['default', {
        discardComments: { removeAll: true },
        normalizeWhitespace: true,
        minifySelectors: true,
        minifyFontValues: true,
        minifyParams: true,
        convertValues: true,
        reduceIdents: true,
        minifySelectors: true,
      }]
    })
  ]
}
EOF

"$POSTCSS" "$CSS_FILE" --config "$TEMP_DIR" --output "$TEMP_CSS"

# Minify JavaScript
echo "⚡ Minifying JavaScript..."
TEMP_JS="$TEMP_DIR/cosam-calendar.min.js"

"$TERSER" "$JS_FILE" \
    --compress \
    --mangle \
    --toplevel \
    --ecma 2018 \
    --output "$TEMP_JS"

# Get minified sizes
MIN_CSS_SIZE=$(stat -f%z "$TEMP_CSS")
MIN_JS_SIZE=$(stat -f%z "$TEMP_JS")
MIN_TOTAL_SIZE=$((MIN_CSS_SIZE + MIN_JS_SIZE))

if [[ -n "$SCHEDULE_JSON_FILE" ]]; then
    MIN_TOTAL_SIZE=$((MIN_TOTAL_SIZE + JSON_SIZE))
fi

echo ""
echo "📊 Minified sizes:"
CSS_REDUCTION=$(echo "scale=1; 100 * ($CSS_SIZE - $MIN_CSS_SIZE) / $CSS_SIZE" | bc 2>/dev/null || echo "0")
JS_REDUCTION=$(echo "scale=1; 100 * ($JS_SIZE - $MIN_JS_SIZE) / $JS_SIZE" | bc 2>/dev/null || echo "0")
TOTAL_REDUCTION=$(echo "scale=1; 100 * ($TOTAL_SIZE - $MIN_TOTAL_SIZE) / $TOTAL_SIZE" | bc 2>/dev/null || echo "0")
echo "   CSS: $(format_bytes $MIN_CSS_SIZE) ($(format_bytes $((CSS_SIZE - MIN_CSS_SIZE))) saved, ${CSS_REDUCTION}% reduction)"
echo "   JS:  $(format_bytes $MIN_JS_SIZE) ($(format_bytes $((JS_SIZE - MIN_JS_SIZE))) saved, ${JS_REDUCTION}% reduction)"
echo "   Total: $(format_bytes $MIN_TOTAL_SIZE) ($(format_bytes $((TOTAL_SIZE - MIN_TOTAL_SIZE))) saved, ${TOTAL_REDUCTION}% reduction)"
echo ""

# Create single HTML block output
echo "📦 Creating single HTML block..."

# Read minified content
MIN_CSS_CONTENT=$(cat "$TEMP_CSS")
MIN_JS_CONTENT=$(cat "$TEMP_JS")

# Prepare JSON data
if [[ -n "$SCHEDULE_JSON_FILE" ]]; then
    JSON_DATA=$(cat "$SCHEDULE_JSON_FILE")
else
    JSON_DATA="{}"
fi

# Create the complete HTML block
cat <<EOF >"$FINAL_OUTPUT"
<style>
${MIN_CSS_CONTENT}
</style>
<div id="cosam-calendar-root"></div>
<script>
// CosAm Calendar Widget - Embeddable Version
// Copyright (c) 2026 Daphne Pfister
// SPDX-License-Identifier: BSD-2-Clause
// Project: https://github.com/daphnediane/cosam-sched

// Schedule data
window.cosamScheduleData = ${JSON_DATA};

// Minified JavaScript
${MIN_JS_CONTENT}

// Initialize widget
(function() {
    if (typeof CosAmCalendar !== 'undefined' && window.cosamScheduleData) {
        CosAmCalendar.init({
            el: document.getElementById('cosam-calendar-root'),
            data: window.cosamScheduleData
        });
    }
})();
</script>
EOF

OUTPUT_SIZE=$(stat -f%z "$FINAL_OUTPUT")
echo "✅ HTML block created:"
echo "   File: $FINAL_OUTPUT"
echo "   Size: $(format_bytes $OUTPUT_SIZE)"
echo ""

echo "🎉 Minification complete!"
echo ""
echo "📋 Usage instructions:"
echo "   1. Open '$FINAL_OUTPUT' and copy the entire content"
echo "   2. Paste into a single HTML box"
echo "   3. The widget will automatically initialize with your schedule data"
echo ""
if [[ -z "$SCHEDULE_JSON_FILE" ]]; then
    echo "💡 To include schedule data:"
    echo "   $0 --json path/to/your/schedule.json"
    echo ""
fi
echo "🔗 Project links:"
echo "   Repository: https://github.com/daphnediane/cosam-sched"
echo "   Widget: https://github.com/daphnediane/cosam-sched/tree/main/widget"
