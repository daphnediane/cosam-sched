# Migrate cosam-editor to schedule-data

## Summary

Port cosam-editor from schedule-core to schedule-data for the GPUI desktop editor.

## Status

Not Started

## Priority

High

## Description

cosam-editor currently uses schedule-core data types and its snapshot-based undo system. Migrate to schedule-data's entity model, mutation API, and (optionally) edit history. Evaluate whether to keep snapshot undo or migrate to command-based undo (from old EDITOR-025).

## Implementation Details

- Replace schedule-core `Panel`, `Presenter`, `Room`, `PanelType` with schedule-data entities
- Use schedule-data `ScheduleFile` for load/save (XLSX and JSON)
- Use schedule-data query API for panel/presenter/room lookups in UI
- Use schedule-data mutation API for edits triggered by UI actions
- Evaluate undo strategy: keep snapshot undo (simpler for GUI) or migrate to edit history (unified with CLI)
- Update all UI views to read from schedule-data entity fields
- Update display export for browser preview generation

## Acceptance Criteria

- Editor loads and displays schedules via schedule-data
- All existing editing operations work with schedule-data mutations
- Save produces correct output via schedule-data serialization
- Browser preview uses schedule-data display export
- No dependency on schedule-core remains
- Undo/redo decision documented
