# Drag-and-Drop Event Scheduling

## Summary

Enable drag-and-drop to move events between time slots and rooms.

## Status

Open

## Priority

Medium

## Description

Implement a grid or timeline view where events can be dragged to change their time or room assignment. This provides an intuitive visual scheduling experience.

## Implementation Details

- Timeline grid view with rooms as columns and time as rows
- Drag events to change start time (snap to configurable intervals)
- Drag events between rooms to reassign
- Visual feedback during drag (ghost element, valid/invalid drop zones)
- Undo support for drag operations
- Conflict highlighting during drag
