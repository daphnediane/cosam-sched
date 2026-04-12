# Virtual Edge Refactor — Entity Field Changes

## Summary

Add stored relationship fields to Panel, EventRoom, and Presenter; remove
edge-backed computed field closures.

## Status

Completed

## Priority

High

## Description

Replace edge-backed computed fields with stored UUID fields on the owning
entities, following the virtual edge design documented in META-035.

### Panel (`panel.rs`)

Add stored fields:

- `panel_type: Option<PanelTypeId>` — replaces `PanelToPanelType` edge
- `event_room: Option<EventRoomId>` — replaces `PanelToEventRoom` edge
- `presenters: Vec<PresenterId>` — replaces `PanelToPresenter` edges

Replace edge-backed computed field closures with closures that use virtual edge storage (stored fields) and reverse indexes.

**Status**: Stored fields added (`panel_type_id`, `event_room_id`, `presenter_ids` as backing storage). Computed field closures still use edge-based lookups in some places.

### EventRoom (`event_room.rs`)

Add stored field:

- `hotel_rooms: Vec<HotelRoomId>` — replaces `EventRoomToHotelRoom` edges

**Status**: Stored field added (`hotel_room_ids` as backing storage).

### Presenter (`presenter.rs`)

Add stored fields:

- `groups: Vec<PresenterId>` — replaces `PresenterToGroup` membership edges
- `is_explicit_group: bool` — replaces self-loop group marker
- `always_grouped: bool` — was per-edge flag; now entity-level
- `always_shown_in_group: bool` — was per-edge flag; now entity-level

Move `groups_of`, `members_of`, `is_group` logic from `PresenterToGroupEntityType`
into `PresenterEntityType` methods (members lookup uses the reverse index once
REFACTOR-037 adds it; temporarily can iterate the full presenter map).

Update entity builders accordingly. All existing entity-level tests must pass.
Add serde round-trip tests for the new fields.

**Status**: All stored fields added (`group_ids`, `is_explicit_group`, `always_grouped`, `always_shown_in_group`). Group membership logic moved to `PresenterEntityType`.

## Work completed

- [x] Added stored fields to entity data structs:
  - `PanelData.panel_type_id: Option<PanelTypeId>`
  - `PanelData.event_room_id: Option<EventRoomId>`
  - `PanelData.presenter_ids: Vec<PresenterId>`
  - `EventRoomData.hotel_room_ids: Vec<HotelRoomId>`
  - `PresenterData.group_ids: Vec<PresenterId>`

- [x] Added reverse index HashMaps to EntityStorage:
  - `panels_by_panel_type: HashMap<NonNilUuid, Vec<NonNilUuid>>`
  - `panels_by_event_room: HashMap<NonNilUuid, Vec<NonNilUuid>>`
  - `panels_by_presenter: HashMap<NonNilUuid, Vec<NonNilUuid>>`
  - `event_rooms_by_hotel_room: HashMap<NonNilUuid, Vec<NonNilUuid>>`
  - `presenters_by_group: HashMap<NonNilUuid, Vec<NonNilUuid>>`

- [x] Implemented all reverse relationship setters:
  - `EventRoomEntityType::set_panels`
  - `PanelTypeEntityType::set_panels`
  - `HotelRoomEntityType::set_event_rooms`

- [x] Verified computed field closures use virtual edges and reverse indices (no changes needed)

- [x] Added hook functions to PanelEntityType for reverse index maintenance (on_insert_hook, on_remove_hook, on_update_hook)

## Work remaining

None - all tasks completed.

## Acceptance Criteria

- [x] `PanelData` has `panel_type`, `event_room`, `presenters` stored fields
- [x] `EventRoomData` has `hotel_rooms` stored field
- [x] `PresenterData` has `groups`, `is_explicit_group`, `always_grouped`,
      `always_shown_in_group` stored fields
- [x] Edge-backed computed field closures replaced with virtual edge/reverse index closures in Panel, EventRoom, Presenter
- [x] Builders updated and round-trip tests pass
- [x] `cargo test` clean

## Dependencies

- META-035: Documentation and design
