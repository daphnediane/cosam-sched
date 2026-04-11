# Virtual Edge Refactor — Entity Field Changes

## Summary

Add stored relationship fields to Panel, EventRoom, and Presenter; remove
edge-backed computed field closures.

## Status

Open

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

Remove computed field closures backed by edge `EntityType` methods.

### EventRoom (`event_room.rs`)

Add stored field:

- `hotel_rooms: Vec<HotelRoomId>` — replaces `EventRoomToHotelRoom` edges

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

## Acceptance Criteria

- [ ] `PanelData` has `panel_type`, `event_room`, `presenters` stored fields
- [ ] `EventRoomData` has `hotel_rooms` stored field
- [ ] `PresenterData` has `groups`, `is_explicit_group`, `always_grouped`,
      `always_shown_in_group` stored fields
- [ ] Edge-backed computed field closures removed from Panel, EventRoom, Presenter
- [ ] Builders updated and round-trip tests pass
- [ ] `cargo test` clean

## Dependencies

- META-035: Documentation and design
