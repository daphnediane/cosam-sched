# Edit Command System With Undo/Redo History

## Summary

Implement a command-based edit system with full undo/redo support.

## Status

Open

## Priority

High

## Description

All mutations to the schedule go through an edit command system that captures
changes as reversible operations, enabling undo/redo in both CLI and GUI contexts.

### EditCommand Enum

Operations such as:

- `UpdateField` — change a single field on an entity
- `AddEntity` — create a new entity
- `RemoveEntity` — soft-delete an entity
- `AddEdge` / `RemoveEdge` — create or remove a relationship
- `MovePanel` — change time/room assignment
- `BatchEdit` — group multiple commands as an atomic unit

Each command must be able to produce its inverse for undo.

### EditHistory

- Stack-based undo/redo with configurable max depth
- `apply(command)` → executes and pushes to undo stack, clears redo stack
- `undo()` → pops from undo stack, applies inverse, pushes to redo stack
- `redo()` → pops from redo stack, applies, pushes to undo stack
- Batch commands undo/redo atomically

### EditContext

- Wraps `Schedule` + `EditHistory`
- Provides the public API for all mutations
- Tracks dirty state for save prompts

### CRDT Integration Point

The edit command system is the natural integration point for CRDT (Phase 3).
Each applied command can generate CRDT operations that are broadcast to peers.
Design the command interface so CRDT ops can be derived from commands.

## Acceptance Criteria

- All entity mutations go through EditCommand
- Undo reverses the last operation exactly
- Redo re-applies an undone operation
- Batch commands undo/redo atomically
- EditContext tracks dirty state
- Unit tests for undo/redo sequences including batches
