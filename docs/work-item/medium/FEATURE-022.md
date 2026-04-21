# Automerge-backed Schedule Storage

## Summary

Make an `automerge::AutoCommit` document the authoritative storage inside
`Schedule`; the in-memory `HashMap` entity store becomes a derived cache.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-021: Edit command system with undo/redo

## Description

Replace the current in-memory `HashMap<TypeId, HashMap<Uuid, …>>` as the
source of truth with an automerge document. The HashMap stays, but only as a
cache that mirrors the document state after every write and is rebuilt in
full on load.

CRDT is **not optional** — there is no `crdt` feature flag, no
`Option<Box<dyn CrdtStorage>>`. `automerge` is a plain workspace dependency
and `Schedule` owns an `AutoCommit` directly.

Document layout:

```text
/meta/schedule_id, /meta/created_at, /meta/generator, /meta/version
/entities/{type_name}/{uuid}/{field_name}     (per CrdtFieldType)
/entities/{type_name}/{uuid}/__deleted        (soft delete)
```

Field routing by `CrdtFieldType`:

| CrdtFieldType | automerge op             |
| ------------- | ------------------------ |
| `Scalar`      | `put` / `get` (LWW)      |
| `Text`        | `splice_text` / `text`   |
| `List`        | `insert` / `delete`      |
| `Derived`     | not stored               |

A small internal helper module (`crdt/`) exposes typed `read_field` /
`write_field` / `list_entities` / `put_deleted` helpers that take a
`FieldDescriptor` and a `FieldValue` so no entity-specific CRDT code is
written.

## Acceptance Criteria

- `Schedule` owns a non-optional `automerge::AutoCommit` field.
- Every entity create / update / soft-delete writes to the document first,
  then updates the cache from the new document state.
- `Schedule::save() -> Vec<u8>` and `Schedule::load(&[u8]) -> Schedule`
  round-trip the full schedule (entities + metadata) through the automerge
  doc, with cache rebuilt on load.
- Existing entity CRUD tests pass unchanged.
- Soft-delete via `__deleted` scalar; normal reads filter deleted entities.
