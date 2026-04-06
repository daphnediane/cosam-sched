# Edit Commands, History, and Batch Undo/Redo

## Summary

Implement command-based edit history with undo/redo stacks and atomic batch operations.

## Status

Not Started

## Priority

High

## Description

Port `EditCommand` equivalents from `schedule-core/src/edit/command.rs` and `schedule-core/src/edit/history.rs` into `schedule-data`. Support atomic multi-step mutations as single undo steps.

## Implementation Details

- Define `EditCommand` enum covering entity, field, state, and edge mutations
- Implement `EditHistory` with undo/redo stacks and configurable max depth
- Batch operations: multiple mutations compose into one undo step
- Ensure restore/deactivate transitions and dependent-object cleanup are reversible
- Replay-safe: undo then redo produces identical state
- Port relevant command logic from `schedule-core/src/edit/command.rs` (39KB)

## Acceptance Criteria

- All mutation operations generate reversible `EditCommand` records
- Undo/redo stacks work correctly with configurable depth
- Batch operations undo as single step
- State transitions (active/inactive) are fully reversible
- Edge operations included in undo/redo
