# XLSX Export Support

## Summary
Add the ability to export schedule data to XLSX spreadsheets.

## Status
Completed

## Priority
High

## Description

XLSX export functionality has been updated to work with the v7 format. The export now correctly iterates over `schedule.panels` instead of the old `schedule.events` format.

### Current Implementation Status

**What works:**

- XLSX file creation with proper sheet structure (Schedule, Rooms, People)
- Headers are written correctly
- Room and presenter data export correctly
- **Panel sessions are now exported correctly from the v7 panels format**
- Round-trip conversion preserves all panel data

**Fixed issues:**

- ✅ `write_schedule_sheet()` function now iterates over `schedule.panels` via `flatten_panel_sessions()`
- ✅ Handles the new panel/part/session hierarchy correctly
- ✅ Exports panel sessions as individual rows like the old events
- ✅ Round-trip conversion now preserves all panels (tested: 42 panels → 42 panels)
- ✅ **cosam-editor's `xlsx_update` module also updated to use panels format**
- ✅ **In-place XLSX updates now work correctly with v7 panels format**

**Root Cause Fixed:**

Both XLSX export and update functionality have been updated to handle the v7 format migration from `events` to `panels`. The logic now:

1. ✅ Iterates over `schedule.panels` instead of `schedule.events` (both export and update)
2. ✅ Handles the new panel/part/session hierarchy via `flatten_panel_sessions()`
3. ✅ Exports/updates panel sessions as individual rows like the old events
4. ✅ Preserves all change tracking and in-place update capabilities for cosam-editor

### Testing Results

- Test: `cosam-convert --input "input/2026 Schedule.xlsx" --output output/2026/2026-test.xlsx`
- Result: ✅ XLSX file created with 42 panels (original had 42 panels)
- Round-trip test: `cosam-convert --input "output/2026/2026-test.xlsx" --output output/2026/2026-roundtrip.xlsx`
- Result: ✅ Round-trip preserves all 42 panels

## Implementation Details

- Use `rust_xlsxwriter` to generate XLSX output
- Write Schedule, Rooms, and PanelTypes as separate sheets
- Preserve formatting conventions (header rows, column ordering)
- Support "Save As" dialog for choosing output path
- Validate data before export (warn on missing required fields)
