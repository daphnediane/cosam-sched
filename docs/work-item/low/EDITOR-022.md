# cosam-editor: Schedule Grid View and Entity Editing

## Summary

Implement the main schedule grid view and entity editing UI in cosam-editor.

## Status

Open

## Priority

Low

## Description

The core editing experience for the GUI application.

### Schedule Grid View

- Day tabs for navigating between convention days
- Time-slot × room grid showing scheduled panels
- Color coding by panel type
- Drag-and-drop for rescheduling panels
- Conflict highlighting (overlapping panels, double-booked presenters)
- Zoom and scroll controls

### Entity Editing

- Detail pane showing fields for the selected entity
- Inline editing with validation feedback
- Presenter assignment (add/remove from panel)
- Room assignment
- Panel type selection

### Edit Integration

- All changes go through EditContext (FEATURE-010)
- Undo/redo via Ctrl+Z / Ctrl+Shift+Z
- Dirty state indicator and save prompts
- Batch operations (e.g., move all panels in a room)

### Entity List Views

- Panel list with filtering and sorting
- Presenter list with group membership display
- Room list with capacity and sort order
- Panel type list with color swatches

## Acceptance Criteria

- Grid view displays panels in correct time/room positions
- Panels can be edited via the detail pane
- Undo/redo works for all edit operations
- Conflicts are visually highlighted
- Entity list views support filtering
