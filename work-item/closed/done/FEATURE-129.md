# FEATURE-129: Heads-based undo/redo system

## Summary

Replace EditCommand-returns-inverse undo/redo with CRDT heads checkpoints so that bulk operations (XLSX import) become a single undoable step with a user-visible label.

## Status

Completed

## Priority

High

## Blocked By

None

## Related Work Items

- **META-128**: Undo/redo system design review (this implements Option 3)
- **FEATURE-099**: Undo/redo history persistence (depends on this)
- **EDITOR-033**: cosam-editor UI undo/redo menu items

## Description

Replace the current `EditCommand::execute() -> Result<EditCommand, EditError>` inverse pattern
with CRDT-heads-based checkpoints. Each `UndoEntry` stores:

- `label`: human-readable string for menu items ("Update prefix", "Import XLSX", …)
- `pre_heads`: `Vec<ChangeHash>` — fork target for undo
- `changes`: `Vec<Vec<u8>>` — raw automerge change bytes for redo

**Undo**: `Schedule::fork_at_heads(pre_heads)` + cache rebuild  
**Redo**: `apply_changes(changes)` + cache rebuild

A new `EditContext::run_checkpoint(label, f)` method wraps any `&mut Schedule` closure
as a single undoable step — used by XLSX import so the entire import is one undo entry.

## Implementation Notes

**`push_undo` must not clear the redo stack.** The clearing is done explicitly in
`apply()` and `run_checkpoint()` — not inside `EditHistory::push_undo`. If `push_undo`
cleared the redo stack, `redo()` would destroy the remaining redo entries when it
called `push_undo` to promote the redone entry back to the undo stack.  The fix is
`EditHistory::clear_redo()`, called only from the two apply paths.

**AutoCommit flushes on `get_heads()`.** `AutoCommit::get_heads()` takes `&mut self`
because it flushes pending ops before reporting the current tips.  `fork_at_heads()`
calls `self.doc.get_heads()` first for the same reason — ensures any in-flight writes
are committed before the fork so the fork is consistent.

**No-op detection is free.** If `get_changes_since(pre_heads)` returns an empty slice,
no `UndoEntry` is pushed and `dirty_count` is not incremented.  This handles write-only
fields whose idempotency guards suppress the automerge op, and rehydration paths that
run under `with_mirror_disabled`.

**XLSX import callers must opt in to `run_checkpoint`.** `update_schedule_from_xlsx`
still takes `&mut Schedule`, not `&mut EditContext`.  The checkpoint is the caller's
responsibility:

```rust
ctx.run_checkpoint("Import XLSX", |sched| {
    update_schedule_from_xlsx(sched, path, options).map_err(Into::into)
})?;
```

`import_xlsx` (creates a fresh schedule) does not use `run_checkpoint` since there is
no prior state to undo to.

**`RemoveFromField` inverse is no longer stored.** The old system stored the *actually
removed* delta so undo could re-add exactly those items. The heads-based system handles
this automatically via `fork_at` — no delta bookkeeping needed.

## Implementation Phases

- [x] Phase 1: `UndoEntry` struct + `EditHistory` refactor
- [x] Phase 2: `EditCommand::execute` drops inverse return, gains `label()`
- [x] Phase 3: `Schedule::fork_at_heads`
- [x] Phase 4: `EditContext` rewired — `apply(cmd, label)`, `undo`, `redo`
- [x] Phase 5: `EditContext::run_checkpoint`
- [x] Phase 6: XLSX import wired through `run_checkpoint`
- [x] Phase 7: Tests rewritten, undo-after-import test added
