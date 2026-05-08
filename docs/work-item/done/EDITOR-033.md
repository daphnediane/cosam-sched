# cosam-editor: Schedule Grid View and Entity Editing

## Summary

Implement the main schedule grid view and entity editing UI in cosam-editor.

## Status

Completed

## Priority

Low

## Blocked By

- EDITOR-032: cosam-editor: GUI framework selection and scaffold (Completed)

## Description

The core editing experience for the GUI application: a filtered list view
showing panels for the selected day and room, with a detail pane for
inline name editing.

Implemented in both evaluation scaffolds:

- `apps/cosam-editor-gpui/` — GPUI 0.2, entity/component model
- `apps/cosam-editor-dioxus/` — Dioxus 0.7, signals model

Both share `ui/schedule_data.rs` (duplicated pending framework choice);
a shared helper crate (`crates/cosam-editor-shared`) is deferred to
EDITOR-034 once the framework decision is made.

## Acceptance Criteria

- [x] Schedule grid (filter-list-detail layout) displays panels by time/room
- [x] Day tab selector filters panels by date
- [x] Room sidebar filters panels by event room
- [x] Panel cards show name, time range, room, code, and change-state badge
- [x] Can select panel and edit name field inline
- [x] Changes go through the edit command system (`EditContext::update_field_cmd` + `apply`)
- [x] Undo/redo works from the UI (clears selection and rebuilds list)
