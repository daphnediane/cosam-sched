# XLSX Export Support

## Summary

Add the ability to export schedule data to XLSX spreadsheets.

## Status

Open - XLSX export incomplete

## Priority

High

## Description

XLSX export functionality exists but is incomplete - it exports the file structure but no panel data because it's still using the old `schedule.events` format instead of the new v7 `schedule.panels` format.

### Current Implementation Status

**What works:**

- XLSX file creation with proper sheet structure (Schedule, Rooms, People)
- Headers are written correctly
- Room and presenter data export correctly
- Unit tests pass (but use old events format)

**What's broken:**

- `write_schedule_sheet()` function iterates over `schedule.events` (line 365 in xlsx_export.rs)
- v7 format uses `schedule.panels` instead of `schedule.events`
- Round-trip conversion produces empty panels (0 panels exported)
- Export creates valid XLSX but with no schedule data

**Root Cause:**

The XLSX export was never updated to handle the v7 format migration from `events` to `panels`. The export logic needs to be updated to:

1. Iterate over `schedule.panels` instead of `schedule.events`
2. Handle the new panel/part/session hierarchy
3. Export panel sessions as individual rows like the old events

### Testing Results

- Test: `cosam-convert --input "input/2026 Schedule.xlsx" --output output/2026/2026.xlsx`
- Result: XLSX file created but round-trip shows 0 panels (original had 42 panels)
- Unit tests pass because they use synthetic data with old events format

## Implementation Details

- Use `rust_xlsxwriter` to generate XLSX output
- Write Schedule, Rooms, and PanelTypes as separate sheets
- Preserve formatting conventions (header rows, column ordering)
- Support "Save As" dialog for choosing output path
- Validate data before export (warn on missing required fields)
