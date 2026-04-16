# Presenter + EventRoom + HotelRoom Entities

## Summary

Implement the remaining core entity data structs and field descriptors.

## Status

Open

## Priority

High

## Blocked By

- FEATURE-014: PanelType entity (proof of concept)

## Description

### Presenter

- `PresenterData` with name, rank, badge_number, group membership backing
  (`group_ids: Vec<EntityId<PresenterEntityType>>`), boolean flags
  (`is_explicit_group`, `always_grouped`, `always_shown_in_group`)
- `PresenterRank` enum: Guest, InvitedGuest, Judge, Staff, Panelist, FanPanelist
- Computed fields: `panels`, `groups`, `inclusive_panels`, `inclusive_members`,
  `inclusive_groups` (stubs until FEATURE-018)

### EventRoom

- `EventRoomData` with name, location, capacity, hotel_room_id backing
- Computed field: `hotel_room`

### HotelRoom

- `HotelRoomData` with name, floor, tower/building
- Computed field: `event_rooms` (reverse lookup)

## Acceptance Criteria

- All entity data structs compile with serde
- Field descriptors read/write correctly
- PresenterRank serializes/deserializes
- Unit tests for each entity's field read/write
- Serialization round-trip tests
