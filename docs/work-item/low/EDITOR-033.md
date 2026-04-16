# cosam-editor: Schedule Grid View and Entity Editing

## Summary

Implement the main schedule grid view and entity editing UI in cosam-editor.

## Status

Open

## Priority

Low

## Blocked By

- EDITOR-032: cosam-editor: GUI framework selection and scaffold

## Description

The core editing experience for the GUI application: a grid view showing
panels arranged by time and room, with inline editing of entity fields.

## Acceptance Criteria

- Schedule grid displays panels by time/room
- Can select and edit entity fields inline
- Changes go through the edit command system
- Undo/redo works from the UI
