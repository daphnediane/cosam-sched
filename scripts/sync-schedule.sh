#!/bin/bash

# Build the current-year layout PDFs and data files in a temp dir, then
# sync them to OneDrive.
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause
#
# Usage: scripts/sync-schedule.sh [--dry-run|-n] [--year YYYY]
#   Reads from input/<YEAR> Schedule.xlsx
#   Builds into a temporary <tmp>/{pdf,data}/ tree (ramdisk preferred)
#   Syncs both pdf/ and data/ to the OneDrive CosAm schedule folder in one
#   rsync (--relative), leaving any sibling files in that folder untouched

set -eo pipefail

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Globals
declare DRYRUN=""
declare YEAR="2026"
declare WORK_DIR=""

# Functions
fail() {
    echo "$@" >&2
    exit 1
}

usage() {
    echo "Usage: $0 [--dry-run|-n] [--year YYYY]"
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
    is_mount_point /Volumes/ramdisk && { printf '%s\n' /Volumes/ramdisk; return 0; }
    local d
    for d in "${XDG_RUNTIME_DIR:-}" "${TMPDIR:-}" /tmp /var/tmp; do
        [[ -n "$d" && -d "$d" && -w "$d" ]] && { printf '%s\n' "${d%/}"; return 0; }
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
        *)
            usage
            ;;
        esac
    done

    cd "$ROOT_DIR"

    local input="input/${YEAR} Schedule.xlsx"
    local sched_base="${HOME}/Library/CloudStorage/OneDrive-Personal/Cosplay America - CosAm/CosAm - Schedule/${YEAR} - CosAm - Schedule"

    [[ -f "${input}" ]] || fail "input not found: ${input}"

    # Build into a temporary tree; clean it up on exit
    local tmproot
    tmproot="$(pick_tmproot)" || fail "no usable temp directory found"
    WORK_DIR="$(mktemp -d "${tmproot}/cosam-sched-${YEAR}.XXXXXX")"
    trap cleanup EXIT
    echo "Working in ${WORK_DIR}"

    # Subdir names match the OneDrive destination so --relative maps cleanly
    local pdf_dir="${WORK_DIR}/pdf"
    local data_dir="${WORK_DIR}/data"
    cmd mkdir -p "${pdf_dir}" "${data_dir}"

    # Generate the public schedule layout PDFs and data files
    cmd cargo run --release -p cosam-convert -- \
        --input "${input}" \
        --title "Cosplay America ${YEAR} Schedule" \
        --public \
        --embed-as-html \
        --output "${data_dir}/cos${YEAR}.xlsx" \
        --export-embed "${data_dir}/embed.html" \
        --export-test "${data_dir}/preview.html" \
        --layout-config config/layout.toml \
        --export-layout "${pdf_dir}" ||
        fail "cosam-convert failed"

    # Sync pdf/ and data/ to OneDrive in one pass. --relative preserves the
    # pdf/ and data/ path components at the destination; --delete-after only
    # prunes within those two trees, leaving other files in sched_base alone.
    cmd rsync -aPHAX --relative --checksum --delete-after \
        "${WORK_DIR}/./pdf" "${WORK_DIR}/./data" \
        "${sched_base}/" ||
        fail "rsync to ${sched_base} failed"
}

main "$@"
