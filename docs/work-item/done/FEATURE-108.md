# FEATURE-108: cosam-convert --export-layout integration

## Summary

Add an `--export-layout <DIR>` flag to `cosam-convert` that runs a default set of `cosam-layout` outputs after the schedule JSON export.

## Status

Completed

## Priority

Low

## Blocked By

- FEATURE-107: Requires cosam-layout binary

## Description

Convenience integration: running `cosam-convert` with `--export-layout output/layout` invokes the same default layout set as the old `dump_flyers` script, without a separate `cosam-layout` invocation.

## Implementation Details

- Add `--export-layout <DIR>` argument to `cosam-convert` CLI
- After successful JSON export, call into `schedule-layout` (or spawn `cosam-layout` as a subprocess) with a fixed default set of layouts matching the `dump_flyers` defaults:
  - Tabloid schedule (half-day splits)
  - Workshop poster (Tabloid, premium only)
  - Room signs (Tabloid, per day)
  - Guest postcards (4×6, per half-day)
  - Descriptions (Tabloid, per day)
- Uses the same `--brand-config` discovery (`config/brand.toml`)
- Lower priority; defer until FEATURE-107 is stable

## Acceptance Criteria

- `cosam-convert --input file.xlsx --export output/2026.json --export-layout output/layout` succeeds
- Output directory contains the expected PDF files
- Flag is optional; existing behavior unchanged when omitted
