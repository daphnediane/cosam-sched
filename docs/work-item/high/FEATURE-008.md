# Schedule Container and EntityStorage

## Summary

Implement the `Schedule` struct and `EntityStorage` for managing all entities
and relationships.

## Status

In Progress

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

### Edge Storage

Edge-entities are stored in the same `EntityStorage` as regular entities (they
have UUIDs via `#[derive(EntityFields)]`). Additionally:

- `EdgeIndex` per edge type: two `HashMap<NonNilUuid, Vec<NonNilUuid>>` for
  from→[to] and to→[from] lookups, kept in sync with the entity storage
- `GenericEdgeStorage<E: DirectedEdge>` wrapping `EntityStorage` + `EdgeIndex`
  with `add_edge`, `remove_edge`, `find_outgoing`, `find_incoming` operations
- Specialized `PresenterToGroupStorage` with group detection, group-marker
  self-loops, and transitive closure cache (carried over from old design)

### Builder Conflict Validation

When inserting an entity produced by a builder, `EntityStorage::insert` must
check for conflicts before committing:

- **UUID conflict**: reject if the UUID already exists (same entity kind or
  cross-kind collision in the registry)
- **Edge uniqueness**: for edge types where only one edge is valid between a
  given from/to pair (e.g. `PanelToEventRoom`), reject or replace the existing
  edge — policy configurable per edge type (`Reject`, `Replace`, `Allow`)
- Conflict errors should be typed (`InsertError::UuidCollision`,
  `InsertError::EdgeConflict`) so callers can handle replace-vs-reject explicitly

### Schedule API

High-level methods on `Schedule`:

- `add_entity`, `update_entity`, `get_entity`
- `identify(uuid)` → `EntityUUID` (typed wrapper)
- `fetch_entity`, `fetch_typed`, `lookup_typed` — zero-dispatch entity access
- `connect_*` methods for creating edges (validate endpoints exist before insert)
- `get_panel_presenters`, `get_panel_event_room`, etc. — relationship queries
- `find_related` — generic relationship query by edge type and direction

### Default and Serialization

- `Schedule::new()` and `Default` impl
- `ScheduleMetadata` with auto-generated v7 UUID and timestamps

## Acceptance Criteria

- Schedule can hold entities of all types
- UUID registry correctly identifies entity kinds
- Typed access methods compile without runtime dispatch
- Edge add/remove/query by either endpoint works for all five edge types
- Relationship convenience methods return correct typed IDs
- Builder insert rejects UUID collisions and configurable edge conflicts
- Unit tests for add/get/update/connect workflows and conflict handling
