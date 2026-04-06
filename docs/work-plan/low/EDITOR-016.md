# Panel Editing UI

## Summary

Implement inline editing of individual panel properties in the editor.

## Status

Not Started

## Priority

Medium

## Description

Allow users to click on a panel card to edit its properties: name, description, time, room assignment, panel type, presenters, and flags. Changes should update the in-memory schedule model via schedule-data mutations and mark the file as dirty.

## Implementation Details

- Panel detail panel or modal dialog on click
- Editable fields for all panel properties via schedule-data field system
- Time picker for start/end times with duration auto-calculation using TimeRange
- Room and panel type dropdowns populated from schedule-data queries
- Presenter management (add/remove from panel) via edge mutations
- Dirty state tracking with unsaved changes indicator
- Conflict indicators shown inline during editing

## Acceptance Criteria

- Users can edit all panel properties through the UI
- Changes persist to in-memory schedule model
- Time editing respects TimeRange semantics
- Room/type/presenter assignments update edges correctly
- Dirty state indicator works
