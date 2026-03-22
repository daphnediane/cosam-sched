# Implement v7 JSON format in Rust structs and converters

## Summary

Implement the v7 JSON schedule format changes in the Rust codebase: panelTypes hashmap, named color sets, merged timeTypes, stable presenter IDs, baked-in breaks, and metadata fields.

## Status

Open

## Priority

High

## Description

The v7 JSON format has been documented in `docs/json-schedule/v7-*.md`. This work item covers implementing the format changes in the Rust code.

### Scope

#### Phase 3: Rust struct changes (schedule-core)

1. **`panel_type.rs`**: Remove `uid` field, add `is_timeline`, `is_private`, replace `color`+`bw_color` with `colors: IndexMap<String, String>`, add `metadata`
2. **`timeline.rs`**: Update `TimelineEntry.time_type` → `panel_type` referencing prefix directly, add `metadata`
3. **`presenter.rs`**: Add `id: u32`, `always_shown: bool`, `metadata`
4. **`panel.rs`**: Rename `PanelSession.extras` → `metadata`, add `metadata` to `Panel`
5. **`room.rs`**: Add `is_break: bool`, `metadata`
6. **`schedule.rs`**: Add `next_presenter_id` to `Meta`, update variant to `"display"`, drop legacy `events`
7. **`xlsx_import.rs`**: Read new columns, build hashmap, read colors into hashmap
8. **`public_export.rs`** → `display_export.rs`: Generate implicit breaks, use break expansion, `"display"` variant

#### Phase 4: Widget compatibility

- Update `cosam-calendar.js` for hashmap panelTypes, prefix-based lookup, remove implicit break JS

#### Phase 5: Tests

- Update existing tests, add new tests for all v7 features

### Dependencies

- v7 documentation (COMPLETE)
- spreadsheet-format.md updates (COMPLETE)

### Related Items

- BUGFIX-007: Fix `==Group` parsing (can be done as part of this work)
- FEATURE-019: Populate metadata from spreadsheet (separate, after v7 structs exist)
- FEATURE-020: Credit display logic (separate, after v7 presenter changes)
- FEATURE-021: `<Name` prefix support (separate or bundled with BUGFIX-007)

## Acceptance Criteria

- All v7 JSON format docs are faithfully implemented in Rust structs
- panelTypes is a hashmap keyed by prefix in both serialization and deserialization
- Named color sets work for theming
- Timeline types merged into panelTypes with `isTimeline` flag
- Presenter IDs are stable and monotonically increasing
- Display export generates implicit break panels
- Widget correctly renders v7 format
- All existing tests pass with updated struct shapes
- New tests cover v7-specific features
