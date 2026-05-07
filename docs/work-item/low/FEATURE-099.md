# FEATURE-099: Undo/redo history persistence in binary file

## Summary

Serialize the `EditHistory` undo/redo stacks into the `.schedule` binary file so that
undo/redo works across `cosam-modify` invocations.

## Status

Not Started

## Priority

Low

## Blocked By

- CLI-098: cosam-modify help text, exit codes, integration tests, polish
- IDEA-101: decide what ScheduleMetadata.version is for

## Description

Currently `EditHistory` is in-memory only. A fresh invocation of `cosam-modify` always
starts with empty undo/redo stacks even if the previous invocation made changes.

Implementing cross-invocation undo requires:

1. A serialization format for `EditCommand` (and thus `FieldValue`, `RuntimeEntityId`, etc.)
2. A binary file format change — either bumping `FILE_FORMAT_VERSION` and adding an undo
   section to the envelope, or storing the history inside the automerge document.
3. Care that CRDT `apply_changes` / `merge` paths do not restore stale undo state from a
   diverged replica.
4. A maximum history depth limit for the on-disk representation.

## Acceptance Criteria

- [ ] `cosam-modify set` followed by a new `cosam-modify undo` invocation reverses the change
- [ ] History does not bleed between CRDT replicas on merge
- [ ] Binary format version bumped correctly
- [ ] Existing files without a history section load cleanly (empty stacks)
