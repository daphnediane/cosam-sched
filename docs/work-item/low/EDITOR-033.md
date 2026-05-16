# cosam-editor: Schedule Grid View and Entity Editing

## Summary

Implement the main schedule grid view and entity editing UI in cosam-editor.

## Status

Open

## Priority

Low

## Blocked By

- EDITOR-032: cosam-editor: GUI framework selection and scaffold (Completed)

## Description

The core editing experience for cosam-editor (Dioxus 0.7). The initial
scaffold delivered a filter-list-detail layout with inline name editing.
This item tracks the remaining editing features needed for a usable editor.

## Acceptance Criteria

### List view (currently partial)

- [x] Day tab selector filters panels by date
- [ ] Room sidebar filters panels by event room (should not include pseudo rooms)
- [x] Panel cards show name, time range, room, code, and change-state badge
- [x] Can select panel and edit name field inline
- [x] Changes go through the edit command system (`EditContext::update_field_cmd` + `apply`)
- [x] Undo/redo works from the UI (clears selection and rebuilds list)
- [ ] List view scrolls correctly (panel cards compress to fit instead of scrolling — need `flex-shrink: 0` on `.panel-card`)

### Grid view

- [ ] Grid view: time-slot rows × room columns, panels placed in their cell
- [ ] View toggle: switch between list view and grid view
- [ ] Grid cells show panel name, code, and change-state color
- [ ] Selecting a grid cell opens the same detail pane as list view

### Detail pane — additional fields

- [ ] Edit start/end times (time pickers or text fields, validated)
- [ ] Edit room assignment (dropdown or picker from loaded rooms)
- [ ] Edit description field
- [ ] Edit panel code
- [ ] All edits go through `EditContext` with undo/redo support
