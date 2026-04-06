# Populate metadata from spreadsheet extra columns

## Summary

Populate the `metadata` field on all item types from non-standard spreadsheet columns during xlsx import.

## Status

Open

## Priority

Medium

## Description

The `PanelSession` struct has an `extras: ExtraFields` field (renamed to `metadata` in v7) that is defined but never populated during xlsx import — it is always initialized as `IndexMap::new()`. The `row_to_map` function in `xlsx_import.rs` reads all columns into a HashMap, but only known fields are extracted via `get_field()`. The remaining unknown columns are silently discarded.

### Implementation Details

1. After extracting all standard columns from the `row_to_map` result, collect remaining entries into a `metadata` field
2. Apply this pattern to all item types that have `metadata` in v7:
   - `PanelSession` (schedule sheet extra columns)
   - `Panel` (base-level extra columns, if any)
   - `Room` (rooms sheet extra columns)
   - `PanelType` (panel types sheet extra columns)
   - `Presenter` (unlikely to have extras, but support it)
   - `TimelineEntry` (timeline extra columns)
3. Preserve formula values where possible (using `FormulaValue` with both formula string and computed result)
4. Ensure metadata round-trips through JSON serialization and xlsx export

### Dependencies

- Requires v7 struct changes to be completed first (metadata field on all item types)

## Acceptance Criteria

- Non-standard spreadsheet columns are preserved in the `metadata` field during import
- Metadata round-trips correctly through JSON save/load
- Metadata round-trips correctly through xlsx export/update
- No regression in existing import behavior for standard columns
