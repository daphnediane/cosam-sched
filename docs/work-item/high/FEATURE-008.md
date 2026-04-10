# Schedule Container and EntityStorage

## Summary

Implement the `Schedule` struct and `EntityStorage` for managing all entities
and relationships.

## Status

Open

## Priority

High

## Description

The `Schedule` struct is the top-level container holding:

- `EntityStorage` — typed collections for each entity type
- Edge storages for all relationship types
- Entity registry (`HashMap<NonNilUuid, EntityKind>`) for UUID → kind lookup
- `ScheduleMetadata` — version, timestamps, generator info, schedule ID
- Edge entity query engine with caching

### EntityStorage

Per-entity-type `HashMap<NonNilUuid, Data>` collections with:

- `TypedStorage` trait for compile-time dispatch to correct collection
- `get`, `get_by_uuid`, `add_with_uuid`, `update`, `find`, `get_many` operations
- Field-based updates via `(String, FieldValue)` pairs

### Schedule API

High-level methods on `Schedule`:

- `add_entity`, `update_entity`, `get_entity`
- `identify(uuid)` → `EntityUUID` (typed wrapper)
- `fetch_entity`, `fetch_typed`, `lookup_typed` — zero-dispatch entity access
- `connect_*` methods for creating edges
- `get_panel_presenters`, `get_panel_event_room`, etc. — relationship queries
- `find_related` — generic relationship query by edge type and direction

### Default and Serialization

- `Schedule::new()` and `Default` impl
- `ScheduleMetadata` with auto-generated v7 UUID and timestamps

## Acceptance Criteria

- Schedule can hold entities of all types
- UUID registry correctly identifies entity kinds
- Typed access methods compile without runtime dispatch
- Relationship convenience methods return correct typed IDs
- Unit tests for add/get/update/connect workflows
