# Drag-and-Drop Panel Scheduling

## Summary

Enable drag-and-drop to move panels between time slots and rooms in the editor.

## Status

Not Started

## Priority

Low

## Description

Implement a grid or timeline view in the editor where panels can be dragged to change their time or room assignment, providing an intuitive visual scheduling experience.

## Implementation Details

- Timeline grid view with rooms as columns and time as rows
- Drag panels to change start time (snap to configurable intervals)
- Drag panels between rooms to reassign via edge mutations
- Visual feedback during drag (ghost element, valid/invalid drop zones)
- Undo support for drag operations via edit history
- Conflict highlighting during drag

## Acceptance Criteria

- Panels can be dragged to new time slots and rooms
- Time snapping works at configurable intervals
- Visual feedback clearly shows valid/invalid drop zones
- Drag operations are undoable
- Conflicts shown in real-time during drag
