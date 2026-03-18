# Undo/Redo Support

## Summary

Implement undo/redo for all editing operations.

## Status

Open

## Priority

Medium

## Description

Track all changes to the schedule model and allow users to undo and redo them. Essential for a comfortable editing experience.

## Implementation Details

- Command pattern for all state mutations
- Undo stack with configurable depth
- Redo stack (cleared on new edits)
- Keyboard shortcuts: Cmd+Z / Ctrl+Z for undo, Cmd+Shift+Z / Ctrl+Y for redo
- UI indicators showing undo/redo availability
