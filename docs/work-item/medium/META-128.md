# META-128: Undo/redo system design review

## Summary

Review and redesign the undo/redo system to ensure it integrates with all mutation paths and supports the intended checkpoint-based optimization for bulk operations.

## Status

New

## Priority

High

## Blocked By

None

## Related Work Items

- **FEATURE-099**: Undo/redo history persistence in binary file (cross-invocation undo)
- **IDEA-101**: Decide what ScheduleMetadata.version is for (edit version counter consideration)
- **IDEA-036**: Per-Membership Edge Flags (interaction with CRDT/undo model)
- **EDITOR-033**: cosam-editor UI depends on undo/redo working correctly

## Description

The current undo/redo system has multiple architectural issues that prevent it from working as intended:

### Current Issues

**Field writes bypass undo/redo entirely**

- `FieldSet::write_field_value()` and `FieldSet::write_multiple()` directly mutate the schedule without creating undo/redo records
- XLSX imports use direct field writes, making imports non-undoable
- Any code path that writes fields directly bypasses undo/redo tracking
- The `EditContext` history system only tracks operations executed through `EditCommand`s via `execute()`

**CRDT rehydration vs normal operation confusion**

- During CRDT rehydration, `mirror_enabled` is set to `false` to avoid tracking changes
- But there's no clear mechanism to distinguish "normal writes should create history" from "rehydration writes should not"
- The current design assumes all writes go through `EditCommand`, but this isn't enforced

**History persistence not designed**

- FEATURE-099 notes that `EditHistory` is in-memory only
- Cross-invocation undo requires serialization of `EditCommand`, `FieldValue`, `RuntimeEntityId`, etc.
- Need to ensure history doesn't bleed between CRDT replicas on merge
- Maximum history depth limits for on-disk representation not defined

**Checkpoint optimization not implemented**

- Intended design includes checkpoint-based undo/redo for bulk operations
- Undo/redo past last CRDT merge point or file save should rewind to checkpoint instead of replaying actions
- This optimization is not currently implemented

### Intended Design

The undo/redo system should:

1. **Map to CRDT changes**: History should track the relationship between undo/redo actions and the underlying automerge document
2. **Undo replays reverse actions**: Undo should execute the inverse of each operation
3. **Redo replays inverse of undo**: Redo should replay the reverse of an undo as long as it's the most recent thing at the undo state
4. **Checkpoint optimization**: For bulk operations (like import) or undo/redo past save points, rewind to a CRDT checkpoint instead of replaying individual actions
5. **Single undoable import**: Import operations should be a single undoable history item, not thousands of individual field writes
6. **Cross-invocation persistence**: History should persist across tool invocations (FEATURE-099)
7. **CRDT merge safety**: History should not restore stale state from diverged replicas

### Design Questions

1. Should all mutations go through `EditCommand`? Or should field writes automatically create history?
2. How do we distinguish between "normal writes" (create history) and "rehydration writes" (no history)?
3. Should history be stored in the automerge document or as a separate section in the binary file?
4. What's the right granularity for checkpoint optimization (number of operations, time threshold, manual)?
5. How does `ScheduleMetadata.version` relate to edit history (IDEA-101)?

## How Found

Code review during discussion of XLSX import validation and checkpoint rollback.

## Reproduction

1. Perform an XLSX import using `update_schedule_from_xlsx()`
2. Attempt to undo the import via `EditContext::undo()`
3. The import will not be in the undo stack

**Expected:** Import creates a single undoable history item

**Actual:** Import bypasses undo/redo system entirely

## Steps to Fix

**Option 1: Integrate field writes with undo/redo system**

1. Add a mechanism to `Schedule` or `FieldSet` to track whether writes should create undo/redo records
2. Modify `write_field_value` and `write_multiple` to create appropriate `EditCommand`s when not in rehydration mode
3. Ensure import operations create a single composite undoable operation

**Option 2: Route all writes through EditCommand system**

1. Refactor XLSX import to use `EditCommand`s instead of direct field writes
2. Ensure all code paths use `EditContext::execute()` for mutations
3. Deprecate direct field writes in favor of command-based mutations

**Option 3: Hybrid approach with checkpoint optimization**

1. Implement checkpoint-based undo/redo for bulk operations (like import)
2. Use command-based undo/redo for incremental edits
3. Automatically choose checkpoint vs command based on operation size
4. For undo/redo past save points, rewind to CRDT checkpoint instead of replaying actions

## Testing

- Add tests for undo/redo after XLSX import
- Add tests for undo/redo after individual field writes
- Add tests for checkpoint-based undo/redo past save points
- Verify CRDT rehydration still bypasses undo/redo tracking
