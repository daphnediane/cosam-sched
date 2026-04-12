# Virtual Edge Refactor — EntityStorage Reverse Indexes and Hook System

## Summary

Add entity type insertion/removal hooks and per-relationship reverse lookup
indexes to EntityStorage; remove all edge HashMap and EdgeIndex infrastructure.

## Status

Completed

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

### Work completed

- [x] Reverse index fields added to EntityStorage (panels_by_panel_type, panels_by_event_room, panels_by_presenter, event_rooms_by_hotel_room, presenters_by_group)
- [x] EntityType trait has on_insert/on_remove/on_update hook methods with default no-op implementations
- [x] Hook calls integrated into EntityStorage add_entity/remove_entity
- [x] Edge HashMap fields removed from EntityStorage (panel_to_presenter, panel_to_event_room, panel_to_panel_type, event_room_to_hotel_room, presenter_to_group)
- [x] Removed edge HashMap fields from EntityStorage (no longer needed with virtual edges):
  - `panel_to_panel_type: HashMap<NonNilUuid, Vec<NonNilUuid>>`
  - `panel_to_event_room: HashMap<NonNilUuid, Vec<NonNilUuid>>`
  - `panel_to_presenter: HashMap<NonNilUuid, Vec<NonNilUuid>>`
  - `event_room_to_hotel_room: HashMap<NonNilUuid, Vec<NonNilUuid>>`
  - `presenter_to_group: HashMap<NonNilUuid, Vec<NonNilUuid>>`
- [x] Updated computed field closures in Panel, EventRoom, and Presenter to use stored fields and reverse indexes instead of edge lookups
- [x] Added hook functions to PanelEntityType for reverse index maintenance (on_insert_hook, on_remove_hook, on_update_hook)
- [x] Removed EdgeIndex struct and edge_index.rs file
- [x] Deleted five edge entity files (panel_to_presenter.rs, panel_to_event_room.rs, panel_to_panel_type.rs, event_room_to_hotel_room.rs, presenter_to_group.rs)
- [x] Fixed hotel_rooms_of method to read from backing field instead of reverse index
- [x] All tests pass (105 passed)

### Work remaining

None - all tasks completed. Edge infrastructure removal is complete.

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

- [x] `EntityType` trait has `on_insert` / `on_remove` / `on_update` with
      default no-op implementations
- [x] Hook calls integrated into `EntityStorage` CRUD
- [x] Five reverse index fields in `EntityStorage`
- [x] `PanelEntityType`, `EventRoomEntityType`, `PresenterEntityType` hooks
      maintain their respective indexes correctly
- [x] All edge HashMap fields removed from `EntityStorage`
- [x] EdgeIndex struct and edge_index.rs file removed
- [x] `TypedEdgeStorage`, `EdgeEntityType`, `EdgePolicy` removed
- [x] `add_edge` / `remove_edge` removed from `EntityStorage`
- [x] Tests verify reverse index correctness on insert/remove/update
- [x] `cargo test` clean

## Dependencies

- REFACTOR-036: Entity fields must exist before hooks can read them
