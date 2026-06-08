# FEATURE-135: Grid-only XLSX export

## Summary

Add a `--export-xlsx-grid` option to `cosam-convert` that writes only the
per-day grid reference sheets, and wire it into `sync-schedule.sh`.

## Status

Completed

## Priority

Medium

## Description

The full XLSX export (`--output foo.xlsx`) writes both the data tables
(Schedule, Timeline, Rooms, Hotel, PanelTypes, People) and a grid reference
sheet per logical day. For posting a quick at-a-glance reference, only the grid
sheets are wanted, without the editable data tables.

`cosam-convert` now supports `--export-xlsx-grid <file.xlsx>`, which produces a
workbook containing only the grid sheets. `sync-schedule.sh` uses it to publish
`cos<YEAR>grid.xlsx` alongside the existing artifacts.

## Implementation Details

- `schedule-core`: extracted the grid-sheet loop in `xlsx::write::export` into a
  shared `write_grid_sheets` helper, and added `build_grid_spreadsheet` /
  `export_xlsx_grid` (re-exported as `xlsx::export_xlsx_grid`). The grid-only
  workbook reuses the default sheet for the first day so no empty sheet remains.
- `cosam-convert`: new `--export-xlsx-grid` output command and
  `OutputType::ExportXlsxGrid` handler; usage text updated.
- `sync-schedule.sh`: renamed the populated `data/` directory to `generated/`
  and added `--export-xlsx-grid generated/cos<YEAR>grid.xlsx`.

## Acceptance Criteria

- `cosam-convert --input schedule.xlsx --export-xlsx-grid grid.xlsx` writes a
  workbook whose sheets are exactly the `Grid - <day>` sheets, with no data
  tables.
- `sync-schedule.sh` builds and syncs `cos<YEAR>grid.xlsx` into `generated/`.
