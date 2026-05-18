# META-128: Undo/redo system design review

## Summary

Review and redesign the undo/redo system to ensure it integrates with all mutation paths and supports the intended checkpoint-based optimization for bulk operations.

## Status

Completed

## Priority

High

## Blocked By

None

## Related Work Items

- **FEATURE-129**: Heads-based undo/redo — the implementation that resolved this design review
- **FEATURE-099**: Undo/redo history persistence in binary file (cross-invocation undo — still open)
- **IDEA-101**: Decide what ScheduleMetadata.version is for (edit version counter consideration)
- **IDEA-036**: Per-Membership Edge Flags (interaction with CRDT/undo model)
- **EDITOR-033**: cosam-editor UI depends on undo/redo working correctly

## Description

The current undo/redo system has multiple architectural issues that prevent it from working as intended:

### Resolved by FEATURE-129

**Field writes bypass undo/redo entirely** — RESOLVED

- `EditContext::apply(cmd, label)` snapshots CRDT heads before/after the command; no
  inverse computation required
- `EditContext::run_checkpoint(label, f)` wraps any `&mut Schedule` closure (including
  bulk imports) as a single undoable step
- Direct `schedule_mut()` access is still possible but explicitly documented as bypassing
  history — the intended API for all data writes is `apply` or `run_checkpoint`

**CRDT rehydration vs normal operation confusion** — RESOLVED

- No-op writes (no CRDT changes between pre and post heads) produce no undo entry
- Rehydration uses `with_mirror_disabled` as before; it produces no CRDT ops and thus
  no undo entries automatically — the distinction is structural, not flag-based

**Checkpoint optimization not implemented** — RESOLVED

- All undo is now fork-based (`Schedule::fork_at_heads`) — inherently checkpoint-based
- Bulk operations use `run_checkpoint` and produce exactly one undo entry regardless of
  how many field writes they contain

### Remaining Open Item

**History persistence not designed** — still open as FEATURE-099

- `EditHistory` is still in-memory only
- With the new model, persistence is much simpler: serialize `Vec<ChangeHash>` and raw
  change bytes per entry — no need to serialize `EditCommand` variants or `FieldValue`
- Cross-invocation undo still needs a file format change (FEATURE-099)

### Implemented Design (FEATURE-129)

Each `UndoEntry` stores:

- `label: Cow<'static, str>` — human-readable for menu items
- `pre_heads: Vec<ChangeHash>` — fork target for undo
- `changes: Vec<Vec<u8>>` — raw automerge change bytes for redo

**Undo**: `Schedule::fork_at_heads(pre_heads)` + cache rebuild  
**Redo**: `apply_changes(changes)` + cache rebuild  
**New action**: snapshots heads, executes, captures delta, pushes entry + clears redo stack

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
