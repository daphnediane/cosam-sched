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

Atomic operations:

- `UpdateField { kind, uuid, field_name, old_value, new_value }` — change a
  single field on an entity, using `FieldValue` for old/new. Relationship
  mutations (add/remove presenter from panel) are field updates on the owning
  entity's `Vec<TypedId>` field.
- `CreateEntity { kind, uuid, data_snapshot }` — create a new entity. Captures
  full entity state for undo (as serialized `FieldValue` map or snapshot type).
- `RemoveEntity { kind, uuid, data_snapshot }` — soft-delete an entity.
  Captures the entity's prior state for undo.
- `CompoundOp { label, commands: Vec<EditCommand> }` — group multiple commands
  that undo/redo as a single user step. Undo reverses inner commands in reverse
  order; redo replays in forward order. Nesting is valid.

Each command must be able to produce its inverse for undo.

### Compound Operations

Many user-visible actions touch multiple entities. For example, adding a tagged
presenter `"G:Alice=TeamA"` to a panel can produce:

1. `CreateEntity` — member presenter "Alice"
2. `CreateEntity` — group presenter "TeamA"
3. `UpdateField` — set `is_explicit_group = true` on "TeamA"
4. `UpdateField` — add "TeamA" to Alice's `group_ids`
5. `UpdateField` — add "Alice" to the panel's `presenter_ids`

All five must undo/redo as a single user step via `CompoundOp`.

### EditHistory

- Stack-based undo/redo with configurable max depth
- `apply(command)` → executes and pushes to undo stack, clears redo stack
- `undo()` → pops from undo stack, applies inverse, pushes to redo stack
- `redo()` → pops from redo stack, applies, pushes to undo stack
- `CompoundOp` is a single entry on the history stack

### EditContext

- Wraps `Schedule` + optional `EditHistory`
- `EditContext::new(schedule, history)` — edit mode with undo tracking
- `EditContext::import(schedule)` — import mode, no history (bulk operations
  skip the undo stack but still use the same command tree)
- Tracks dirty state for save prompts

### CRDT Integration Point

The edit command system is the natural integration point for CRDT (Phase 3).
Each applied command can generate CRDT operations that are broadcast to peers.
Design the command interface so CRDT ops can be derived from commands.

## Acceptance Criteria

- All entity mutations go through EditCommand
- Undo reverses the last operation exactly
- Redo re-applies an undone operation
- CompoundOp commands undo/redo atomically as one user step
- EditContext tracks dirty state
- Unit tests for undo/redo sequences including compound operations
