# Schedule Container + EntityStorage

## Summary

Implement the `Schedule` struct and `EntityStorage` for managing all entities and relationships.

## Status

Open

## Priority

High

## Blocked By

- FEATURE-018: Relationship storage (EdgeMap / reverse indexes)

## Description

The `Schedule` struct is the top-level container holding:

- `EntityStorage` — typed collections for each entity type
- EdgeMap instances for all relationship types
- Entity registry (`HashMap<NonNilUuid, EntityKind>`) for UUID → kind lookup
- `ScheduleMetadata` — version, timestamps, generator info, schedule ID

Schedule is a **proxy, not an owner** — entity types own their storage; Schedule
provides UUID-keyed coordination.

### EntityStorage

Per-entity-type `HashMap<NonNilUuid, Data>` collections with:

- `TypedStorage` trait for compile-time dispatch to correct collection
- `get`, `get_by_uuid`, `add_with_uuid`, `update`, `find`, `get_many` operations
- Field-based updates via `(String, FieldValue)` pairs

### Schedule API

- `add_entity`, `update_entity`, `get_entity`
- `identify(uuid)` → `EntityKind`
- Relationship convenience methods
- `Schedule::new()` and `Default` impl
- `ScheduleMetadata` with auto-generated v7 UUID and timestamps

## Acceptance Criteria

- Schedule can hold entities of all types
- UUID registry correctly identifies entity kinds
- Typed access methods compile without runtime dispatch
- Relationship convenience methods return correct typed IDs
- Unit tests for add/get/update workflows
