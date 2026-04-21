# Change Tracking, Merge, and Conflict Surfacing

## Summary

Expose automerge change tracking and merge through `Schedule`, and surface
concurrent scalar conflicts to the caller.

## Status

Completed

## Priority

Medium

## Blocked By

- FEATURE-023: CRDT-backed edges via relationship lists

## Description

Build on the authoritative automerge document (FEATURE-022) and CRDT edges
(FEATURE-023) to expose sync / merge primitives on `Schedule`:

- `Schedule::save() -> Vec<u8>` — already added in FEATURE-022; confirmed here.
- `Schedule::load(&[u8]) -> Schedule` — already added in FEATURE-022.
- `Schedule::get_changes() -> Vec<Vec<u8>>` — all encoded changes since doc
  creation.
- `Schedule::get_changes_since(&[ChangeHash]) -> Vec<Vec<u8>>` — delta from
  a known state.
- `Schedule::apply_changes(&[Vec<u8>])` — apply remote changes, then rebuild
  the cache in full.
- `Schedule::merge(&mut other: Schedule)` — convenience wrapper.
- `Schedule::conflicts_for(entity_id, field_name) -> Vec<FieldValue>` —
  returns all concurrent values for a scalar field (empty or singleton when
  no conflict; multiple entries under concurrent writes). Primary read
  still returns one deterministic value (automerge-selected LWW winner).

After any `apply_changes` / `merge`, the cache is rebuilt in full (simple,
correct; incremental rebuild is a later optimization).

## Acceptance Criteria

- `get_changes` / `apply_changes` / `merge` work on real `Schedule`
  instances with entities and edges.
- Concurrent-edit tests from `docs/crdt-design.md` § Merge Semantics pass:
  - Different scalars on same entity → both preserved.
  - Same scalar → LWW winner, alternative visible via `conflicts_for`.
  - Concurrent text edits on a `Text` field merge without loss.
  - Concurrent relationship-list add/remove → add wins.
- `save` → `load` after merge reproduces the merged state.
