# Virtual Edge Refactor — Schedule Methods, Macro Cleanup, Edge File Deletion

## Summary

Update Schedule convenience methods to use field access and reverse indexes;
remove DirectedEdge trait, edge macro attributes, edge EntityKind/EntityUUID
variants, and delete the five edge entity files.

## Status

Open

## Priority

High

## Description

Final cleanup phase of the virtual edge refactor (see META-035).

### Schedule Method Updates (`schedule/mod.rs`)

Replace edge-based queries with field access + reverse index lookups:

- `get_panel_presenters` → reads `panel_data.presenters` directly
- `get_presenter_panels` → reads `panels_by_presenter` reverse index
- `get_panel_event_room` → reads `panel_data.event_room` directly
- `get_event_room_panels` → reads `panels_by_event_room` reverse index
- `get_panel_type` → reads `panel_data.panel_type` directly
- `get_panels_by_type` → reads `panels_by_panel_type` reverse index
- `get_event_room_hotel_rooms` → reads `event_room_data.hotel_rooms` directly
- `get_presenter_groups` → reads `presenter_data.groups` directly
- `get_presenter_members` → reads `presenters_by_group` reverse index
- `is_presenter_group` → reads `presenter_data.is_explicit_group`

Membership mutation helpers updated to mutate fields (not add/remove edges).
Remove: `add_edge`, `add_edge_with_policy`, `remove_edge`, `edge_uuids_from`,
`edge_uuids_to`, `edges_from`, `edges_to`, `edge_exists`, `edge_count` wrappers.

### Macro Cleanup (`schedule-macro`)

Remove support for `#[edge_from]` and `#[edge_to]` macro attributes.
Remove `DirectedEdge` trait generation.

### Entity and UUID System Cleanup (`entity/mod.rs`)

Remove `EntityKind` variants: `PanelToPresenter`, `PanelToEventRoom`,
`PanelToPanelType`, `EventRoomToHotelRoom`, `PresenterToGroup`.

Remove `EntityUUID` variants: same five.

Remove `DirectedEdge` trait definition.

Remove re-exports of all five edge entity types and their `Data`/`Id`/`EntityType`
structs.

### UuidPreference Cleanup (`uuid_preference.rs`)

Remove `Edge { from, to }` variant. Remove any builder logic that auto-upgrades
`GenerateNew` to `Edge`.

### Delete Edge Entity Files

Delete:

- `crates/schedule-data/src/entity/panel_to_presenter.rs`
- `crates/schedule-data/src/entity/panel_to_event_room.rs`
- `crates/schedule-data/src/entity/panel_to_panel_type.rs`
- `crates/schedule-data/src/entity/event_room_to_hotel_room.rs`
- `crates/schedule-data/src/entity/presenter_to_group.rs`

### Test Rewrites

Rewrite all edge-based tests in `schedule/mod.rs` to use field-based
relationship manipulation. Update any entity-level tests that reference
edge types.

## Acceptance Criteria

- [ ] `Schedule` convenience methods use field access + reverse indexes
- [ ] `add_edge` / `remove_edge` and related methods removed from `Schedule`
- [ ] `#[edge_from]` / `#[edge_to]` attributes removed from macro
- [ ] `DirectedEdge` trait removed from entity system
- [ ] `EntityKind` and `EntityUUID` edge variants removed
- [ ] `UuidPreference::Edge` variant removed
- [ ] Five edge entity files deleted
- [ ] All tests rewritten; `cargo test` passes clean

## Dependencies

- REFACTOR-036: Entity fields
- REFACTOR-037: Reverse indexes and hooks
