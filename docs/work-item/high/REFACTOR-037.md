# Virtual Edge Refactor — EntityStorage Reverse Indexes and Hook System

## Summary

Add entity type insertion/removal hooks and per-relationship reverse lookup
indexes to EntityStorage; remove all edge HashMap and EdgeIndex infrastructure.

## Status

Open

## Priority

High

## Description

Implements the storage side of the virtual edge design (see META-035).
Supersedes FEATURE-033 (which proposed a similar hook system for EdgeIndex
maintenance).

### EntityType Hook Methods

Add default no-op methods to the `EntityType` trait:

```rust
fn on_insert(storage: &mut EntityStorage, data: &Self::Data) {}
fn on_remove(storage: &mut EntityStorage, data: &Self::Data) {}
fn on_update(storage: &mut EntityStorage, old: &Self::Data, new: &Self::Data) {}
```

`EntityStorage::add_entity`, `remove_entity`, and any mutation path must call
these hooks after/before the HashMap operation.

### Reverse Lookup Indexes in EntityStorage

Add five reverse index fields (replacing all EdgeIndex + edge HashMap fields):

- `panels_by_panel_type: HashMap<NonNilUuid, Vec<NonNilUuid>>`
- `panels_by_event_room: HashMap<NonNilUuid, Vec<NonNilUuid>>`
- `panels_by_presenter: HashMap<NonNilUuid, Vec<NonNilUuid>>`
- `event_rooms_by_hotel_room: HashMap<NonNilUuid, Vec<NonNilUuid>>`
- `presenters_by_group: HashMap<NonNilUuid, Vec<NonNilUuid>>`

### Hook Implementations

`PanelEntityType::on_insert` — for each of `panel_type`, `event_room`,
`presenters`, add the panel UUID to the appropriate reverse index.

`PanelEntityType::on_remove` — remove panel UUID from all reverse indexes.

`PanelEntityType::on_update` — diff old vs new fields; add/remove from indexes
as needed.

`EventRoomEntityType` — same pattern for `hotel_rooms`.

`PresenterEntityType` — same pattern for `groups`.

### Infrastructure Removal

Remove from `EntityStorage`:

- All five edge `HashMap<NonNilUuid, *Data>` fields
- All five `EdgeIndex` fields
- `add_edge`, `add_edge_with_policy`, `remove_edge` methods
- `edge_uuids_from`, `edge_uuids_to`, `edges_from`, `edges_to`, `edge_exists`,
  `edge_count` methods

Remove traits: `TypedEdgeStorage`, `EdgeEntityType`, `EdgePolicy`.

## Acceptance Criteria

- [ ] `EntityType` trait has `on_insert` / `on_remove` / `on_update` with
      default no-op implementations
- [ ] Hook calls integrated into `EntityStorage` CRUD
- [ ] Five reverse index fields in `EntityStorage`
- [ ] `PanelEntityType`, `EventRoomEntityType`, `PresenterEntityType` hooks
      maintain their respective indexes correctly
- [ ] All edge HashMap and EdgeIndex fields removed
- [ ] `TypedEdgeStorage`, `EdgeEntityType`, `EdgePolicy` removed
- [ ] `add_edge` / `remove_edge` removed from `EntityStorage`
- [ ] Tests verify reverse index correctness on insert/remove/update
- [ ] `cargo test` clean

## Dependencies

- REFACTOR-036: Entity fields must exist before hooks can read them
