# Schedule Container and EntityStorage

## Summary

Implement the `Schedule` struct and `EntityStorage` for managing all entities
and relationships.

## Status

Completed

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

### Edge Storage (done)

Edge-entities are stored in `EntityStorage` as regular entities (they have UUIDs
via `#[derive(EntityFields)]`). Implemented:

- `EdgeIndex` per edge type: two `HashMap<NonNilUuid, Vec<NonNilUuid>>` for
  from→[to] and to→[from] lookups, kept in sync via `add_edge`/`remove_edge`
- `TypedEdgeStorage` trait for compile-time dispatch to correct `EdgeIndex`
- Convenience query methods on each edge `EntityType` (`presenters_of`,
  `panels_of`, `event_room_of`, `panels_in`, `panel_type_of`, `panels_of_type`,
  `hotel_rooms_of`, `event_rooms_in`, `groups_of`, `members_of`, `is_group`)
- `Schedule` convenience wrappers delegating to edge type methods
- Panel computed fields (`presenters`, `event_room`, `panel_type`) read via
  edge type convenience methods on `EntityStorage` (not through `Schedule`)
- All edge-based computed fields have `#[write]` closures (replace edges on
  assignment); `Presenter.groups` / `Presenter.members` write closures preserve
  self-loop group markers
- `WritableField::write` takes `&mut Schedule` so write closures can mutate edges
- Membership mutation helpers on `Schedule`:
  - `mark_presenter_group` / `unmark_presenter_group` — manage the self-loop group marker
  - `add_member(member, group)` — add membership with default flags (no-op if exists)
  - `add_grouped_member(member, group)` — add/update with `always_grouped = true`
  - `add_shown_member(member, group)` — add/update with `always_shown_in_group = true`
  - `remove_member(member, group)` — remove membership edge
- Macro-generated `build()` calls `add_edge` for edge entities
- 10 comprehensive integration tests for add/remove/query/collision/identify

Still needed:

- Edge uniqueness policies (`Reject`, `Replace`, `Allow`) per edge type
- Specialized `PresenterToGroupStorage` with transitive closure cache
- Computed fields for all node entities: `Panel` (`presenters`, `event_room`,
  `panel_type`), `Presenter` (`groups`, `members`), `EventRoom` (`hotel_rooms`,
  `panels`), `HotelRoom` (`event_rooms`), `PanelType` (`panels`) — all with
  read + write closures

## Acceptance Criteria

- [x] Schedule can hold entities of all types (node and edge)
- [x] UUID registry correctly identifies entity kinds
- [x] Typed access methods compile without runtime dispatch
- [x] Generic `EntityStore<T>` trait with blanket impl for `EntityStorage`
- [x] `Schedule` implements `EntityStore<T>` with UUID registry management
- [x] `EntityType` trait includes associated `type Id`
- [x] Builder `build()` takes `&mut Schedule` and returns typed ID
- [x] `BuildError` combines `ValidationError` and `InsertError`
- [x] Edge add/remove/query by either endpoint works for all five edge types
- [x] Relationship convenience methods return correct typed IDs
- [x] Builder insert rejects configurable edge conflicts
- [x] Unit tests for add/get/update/connect workflows and conflict handling
- [x] Edge-based computed fields implemented for all node entity types
- [x] Membership mutation helpers (`add_member`, `add_grouped_member`,
  `add_shown_member`, `remove_member`, `mark_presenter_group`,
  `unmark_presenter_group`) on `Schedule`

## Design Revision

The edge HashMap + EdgeIndex storage approach implemented here was superseded
by the virtual edge design in REFACTOR-036/037/038:

- Edge HashMaps and `EdgeIndex` per edge type removed from `EntityStorage`
- Replaced with five reverse lookup indexes maintained by entity type hooks
- `TypedEdgeStorage`, `EdgeEntityType`, `EdgePolicy` traits removed
- `add_edge` / `remove_edge` removed from `EntityStorage` and `Schedule`
- Membership mutation helpers simplified to field mutations on `PresenterData`
- `PresenterToGroup` self-loop group marker replaced by `is_explicit_group: bool`
