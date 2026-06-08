# FEATURE-099: Undo/redo history persistence in binary file

## Summary

Serialize the `EditHistory` undo/redo stacks into the `.schedule` binary file so that
undo/redo works across `cosam-modify` invocations.

## Status

Open

## Priority

Low

## Blocked By

- CLI-098: cosam-modify help text, exit codes, integration tests, polish
- IDEA-101: decide what ScheduleMetadata.version is for
- IDEA-130: Collaborative undo via inverse writes past sync horizon

## Description

Currently `EditHistory` is in-memory only. A fresh invocation of `cosam-modify` always
starts with empty undo/redo stacks even if the previous invocation made changes.

With the heads-based undo/redo system (FEATURE-129), each `UndoEntry` stores only:

- `label: Cow<'static, str>`
- `pre_heads: Vec<ChangeHash>` (array of 32-byte hashes)
- `changes: Vec<Vec<u8>>` (raw automerge change bytes already in the document)

This is significantly simpler to serialize than the old `EditCommand` approach.
Implementing cross-invocation undo requires:

1. A serialization format for `UndoEntry` — CBOR or JSON for the label and head hashes;
   the change bytes are already raw bytes.
2. A binary file format change — bump `FILE_FORMAT_VERSION` and add a history section to
   the envelope after the automerge document bytes.
3. On load, validate that all `pre_heads` and change hashes still exist in the loaded
   document; discard entries whose heads are no longer reachable (handles diverged replicas).
4. A maximum history depth limit for the on-disk representation (default 100 already in
   `EditHistory::DEFAULT_MAX_DEPTH`).

## Acceptance Criteria

- [ ] `cosam-modify set` followed by a new `cosam-modify undo` invocation reverses the change
- [ ] History does not bleed between CRDT replicas on merge
- [ ] Binary format version bumped correctly
- [ ] Existing files without a history section load cleanly (empty stacks)
