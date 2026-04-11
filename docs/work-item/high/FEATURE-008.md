# Schedule Container and EntityStorage

## Summary

Implement the `Schedule` struct and `EntityStorage` for managing all entities
and relationships.

## Status

In Progress (entity storage and generic CRUD complete; edge indexes pending)

## Priority

High

## Description

The `Schedule` struct is the top-level container holding:

- `EntityStorage` — typed collections for each entity type
- UUID registry (`HashMap<NonNilUuid, EntityKind>`) for UUID → kind lookup
- `ScheduleMetadata` — version, timestamps, generator info, schedule ID

### EntityStorage (done)

Per-entity-type `HashMap<NonNilUuid, Data>` collections with:

- `TypedStorage` trait for compile-time dispatch to correct `HashMap`
- `EntityStore<T>` trait providing generic CRUD (`get_entity`, `get_entity_mut`,
  `insert_entity`, `remove_entity`, `contains_entity`)
- Blanket `impl<T: TypedStorage> EntityStore<T> for EntityStorage`
- All 5 node and 5 edge entity types stored in one `EntityStorage` struct

### EntityType Associated Id (done)

`EntityType` trait includes `type Id: TypedId<EntityType = Self>`, linking each
entity type to its typed ID wrapper. The derive macro generates this automatically.
This enables generic methods like `Schedule::add_entity::<T>()` to return the
correct typed ID.

### Schedule Generic CRUD (done)

`Schedule` implements `EntityStore<T>` for all `T: TypedStorage`, adding UUID
registry management on top of raw storage. Convenience methods:

- `add_entity::<T>(data)` → `Result<T::Id, InsertError>`
- `get_entity::<T>(id)` / `get_entity_mut::<T>(id)`
- `remove_entity::<T>(id)` / `contains_entity::<T>(id)`
- `get_entity_by_uuid::<T>(uuid)`
- `identify(uuid)` → `Option<EntityUUID>` (typed wrapper)

### Builder Integration (done)

`Builder::build(&mut Schedule)` validates, resolves UUID, builds data, and
inserts in one step. Returns `Result<TypedId, BuildError>` where `BuildError`
combines `ValidationError` and `InsertError`. `Builder::build_data()` produces
the data struct standalone (for tests or deferred insertion).

### Default and Serialization (done)

- `Schedule::new()` and `Default` impl
- `ScheduleMetadata` with auto-generated v7 UUID and timestamps

### Edge Storage (not yet implemented)

Edge-entities are stored in `EntityStorage` as regular entities (they have UUIDs
via `#[derive(EntityFields)]`). Still needed:

- `EdgeIndex` per edge type: two `HashMap<NonNilUuid, Vec<NonNilUuid>>` for
  from→[to] and to→[from] lookups, kept in sync with entity storage
- Specialized `PresenterToGroupStorage` with group detection, group-marker
  self-loops, and transitive closure cache
- Edge uniqueness policies (`Reject`, `Replace`, `Allow`) per edge type
- Relationship convenience queries (`get_panel_presenters`, etc.) — stubs exist

## Acceptance Criteria

- [x] Schedule can hold entities of all types (node and edge)
- [x] UUID registry correctly identifies entity kinds
- [x] Typed access methods compile without runtime dispatch
- [x] Generic `EntityStore<T>` trait with blanket impl for `EntityStorage`
- [x] `Schedule` implements `EntityStore<T>` with UUID registry management
- [x] `EntityType` trait includes associated `type Id`
- [x] Builder `build()` takes `&mut Schedule` and returns typed ID
- [x] `BuildError` combines `ValidationError` and `InsertError`
- [ ] Edge add/remove/query by either endpoint works for all five edge types
- [ ] Relationship convenience methods return correct typed IDs
- [ ] Builder insert rejects configurable edge conflicts
- [ ] Unit tests for add/get/update/connect workflows and conflict handling
