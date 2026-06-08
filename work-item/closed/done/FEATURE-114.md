# FEATURE-114: XLSX grid reference sheets per day

## Summary

Add one grid-view reference sheet per day to the exported XLSX, mirroring the HTML schedule grid with merged cells for multi-slot and multi-room events.

## Status

Completed

## Priority

Medium

## Blocked By (optional)

## Description

The XLSX export currently produces tabular data sheets (Schedule, Rooms, People, PanelTypes).
This feature adds one additional worksheet per convention day (named e.g. `Grid - Fri Jun 27`)
that renders the schedule as a visual grid — matching the HTML table layout shown in the UI —
with rows for time slots and columns for rooms, using merged cells for events that span
multiple time slots or multiple rooms.

### Day boundary logic

Uses the same "logical day" split as the overnight-break detection in `synthesize_breaks`:
a day ends at its last event end-time, not strictly at midnight.  Any gap of > 4 hours or a
date-line crossing (whichever comes first) is treated as the overnight break; events after
that gap belong to the next day's sheet.

### Layout

- **Row 1**: Day title merged across all columns.
- **Row 2**: Header row — "Time" in col 1, then one column per room (short name + long name if
  available) in the same sort order used by the HTML grid (room `sort_key`, then `room_name`).
- **Remaining rows**: One row per time-slot boundary (every unique start/end time across all
  panels on that day).
  - Column 1 shows the human-readable time label (e.g. "2 PM", "2:30"); on-the-hour slots get
    the full label, off-hour slots may show only the minutes.
  - Each panel occupies the cell(s) at its start-row × room-column, merged downward for
    duration and rightward if it spans multiple rooms.
  - Cell text: panel name (line 1) + presenters credit string (line 2, if any).
  - Break panels (is_break = true) span all room columns and show the break label.

### Implementation

Add `write_grid_sheet` in `crates/schedule-core/src/xlsx/write/export.rs` (or a sibling
module). Called from `export_xlsx` once per logical day. Relies on data already available
in `Schedule` (panels, event rooms, panel types).

No new dependencies required; uses existing `umya-spreadsheet` `add_merge_cells` API.

## Acceptance Criteria

- Each day in the schedule produces one `Grid - <day label>` worksheet.
- Time slots are rows; rooms are columns; order matches HTML grid.
- Events span the correct number of merged rows (duration) and merged columns (rooms).
- Break events span the full room-column width.
- Day boundary matches the overnight-break heuristic (> 4 h gap or date line).
- `cargo test` passes with a regression test covering basic cell placement and merges.

## Notes

- The grid sheets are reference/print-support sheets; they are not Excel Tables (no
  `add_table` call needed).
- Sheet names must be ≤ 31 characters (Excel limit); truncate the day label if needed.
