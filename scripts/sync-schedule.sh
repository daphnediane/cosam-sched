#!/bin/bash

# Build the current-year layout PDFs and data files in a temp dir, then
# sync them to OneDrive.
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
#
# Usage: scripts/sync-schedule.sh [--dry-run|-n] [--year YYYY]
#   Reads from input/<YEAR> Schedule.xlsx
#   Builds into a temporary <tmp>/{pdf,generated}/ tree (ramdisk preferred)
#   Syncs both pdf/ and generated/ to the OneDrive CosAm schedule folder in one
#   rsync (--relative), leaving any sibling files in that folder untouched

set -eo pipefail

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Globals
declare DRYRUN=""
declare YEAR="2026"
declare OUTPUT_DIR=""
declare WORK_DIR=""

# Functions
fail() {
    echo "$@" >&2
    exit 1
}

usage() {
    echo "Usage: $0 [--dry-run|-n] [--year YYYY] [--output-dir DIR]"
    exit 1
}

cmd() {
    local args=("$@")
    echo "+ ${args[*]}"
    [[ -z "${DRYRUN:-}" ]] || return 0
    "${args[@]}"
}

# True if the given path is itself a filesystem mount point (portable: macOS
# lacks mountpoint(1), so compare df's "Mounted on" field to the path).
is_mount_point() {
    local d="$1"
    [[ -d "$d" ]] || return 1
    [[ "$(df -P "$d" 2>/dev/null | awk 'NR==2 {print $NF}')" == "$d" ]]
}

# Echo the first usable temp root: a mounted ramdisk, else the standard
# tmpfs/temp locations.
pick_tmproot() {
    is_mount_point /Volumes/ramdisk && {
        printf '%s\n' /Volumes/ramdisk
        return 0
    }
    local d
    for d in "${XDG_RUNTIME_DIR:-}" "${TMPDIR:-}" /tmp /var/tmp; do
        [[ -n "$d" && -d "$d" && -w "$d" ]] && {
            printf '%s\n' "${d%/}"
            return 0
        }
    done
    return 1
}

cleanup() {
    [[ -n "${WORK_DIR}" && -d "${WORK_DIR}" ]] && rm -rf "${WORK_DIR}"
}

main() {
    while [ $# -gt 0 ]; do
        case "$1" in
        --dry-run | -n)
            DRYRUN="echo"
            shift
            ;;
        --year)
            [[ -n "${2:-}" ]] || usage
            YEAR="$2"
            shift 2
            ;;
        --output-dir)
            [[ -n "${2:-}" ]] || usage
            OUTPUT_DIR="$2"
            shift 2
            ;;
        *)
            usage
            ;;
        esac
    done

    cd "$ROOT_DIR"

    local input="input/${YEAR} Schedule.xlsx"
    local sched_base="${HOME}/Library/CloudStorage/OneDrive-Personal/Cosplay America - CosAm/CosAm - Schedule/${YEAR} - CosAm - Schedule"

    # Use custom output directory if specified
    if [[ -n "${OUTPUT_DIR:-}" ]]; then
        sched_base="${OUTPUT_DIR}"
    fi

    [[ -f "${input}" ]] || fail "input not found: ${input}"

    # Build into a temporary tree; clean it up on exit
    local tmproot
    tmproot="$(pick_tmproot)" || fail "no usable temp directory found"
    WORK_DIR="$(mktemp -d "${tmproot}/cosam-sched-${YEAR}.XXXXXX")"
    trap cleanup EXIT
    echo "Working in ${WORK_DIR}"

    # Subdir name matches the OneDrive destination so --relative maps cleanly
    local generated_dir="${WORK_DIR}/generated"
    cmd mkdir -p "${generated_dir}"

    # Generate the public schedule layout PDFs and data files
    cmd cargo run --release -p cosam-convert -- \
        --input "${input}" \
        --title "Cosplay America ${YEAR} Schedule" \
        --public \
        --embed-as-html \
        --output "${generated_dir}/cos${YEAR}.xlsx" \
        --export-xlsx-grid "${generated_dir}/cos${YEAR}grid.xlsx" \
        --export-embed "${generated_dir}/embed.html" \
        --export-embed-head "${generated_dir}/embed-head.html" \
        --export-embed-body "${generated_dir}/embed-body.html" \
        --export-test "${generated_dir}/preview.html" \
        --layout-config config/layout.toml \
        --export-layout "${generated_dir}" ||
        fail "cosam-convert failed"

    # Generate 4-up booklets for any quarter-sized PDFs
    local quarter_dir="${generated_dir}/quarter"
    if [[ -d "${quarter_dir}" ]]; then
        local booklet_dir="${generated_dir}/booklet"
        cmd mkdir -p "${booklet_dir}"
        for pdf in "${quarter_dir}"/*.pdf; do
            [[ -f "${pdf}" ]] || continue
            local basename=$(basename "${pdf}" .pdf)
            local output="${booklet_dir}/${basename%-quarter}-booklet.pdf"
            echo "Creating 4-up booklet: ${pdf} -> ${output}"
            cmd python3 "${SCRIPT_DIR}/generate_booklet_pages.py" "${pdf}" "${output}" ||
                echo "Warning: failed to create booklet for ${basename}"
            
            # Also create folded version
            local folded_output="${booklet_dir}/${basename%-quarter}-folded.pdf"
            echo "Creating 4-up folded booklet: ${pdf} -> ${folded_output}"
            cmd python3 "${SCRIPT_DIR}/generate_booklet_folded.py" "${pdf}" "${folded_output}" ||
                echo "Warning: failed to create folded booklet for ${basename}"
        done
    fi

    # Sync generated/ to OneDrive. --relative preserves the generated/ path
    # component at the destination; --delete-after only prunes within that tree,
    # leaving other files in sched_base alone.
    cmd rsync -aPHAX --relative --checksum --delete-after \
        "${WORK_DIR}/./generated" \
        "${sched_base}/" ||
        fail "rsync to ${sched_base} failed"
}

main "$@"
