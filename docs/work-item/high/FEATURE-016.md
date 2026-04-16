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

**`PresenterCommonData`** (`pub`):

- `name: String` — full display name (required, indexed)
- `rank: PresenterRank` — `Guest`, `Judge`, `Staff`, `InvitedGuest(Option<String>)`, `Panelist`, `FanPanelist`
- `bio: Option<String>`
- `is_explicit_group: bool`
- `always_grouped: bool` — always shown under group name, never individually
- `always_shown_in_group: bool` — group name always shown even with partial attendance
- `sort_rank: Option<PresenterSortRank>` — import ordering key (column, row, member index)

Future fields (no spreadsheet source yet): `pronouns: Option<String>`, `website: Option<String>`

**`PresenterInternalData`** (`pub(crate)`) — `EntityType::InternalData`:

- `data: PresenterCommonData`
- `code: PresenterId`

**`PresenterData`** (`pub`) — export/API view:

- `data: PresenterCommonData`
- `code: String`
- `group_ids: Vec<PresenterId>` — groups this presenter belongs to (from edge maps)
- `panels: Vec<PanelId>` — panels this presenter is on (from edge maps)

Computed edge-backed fields (stubs until FEATURE-018):

- `groups`, `is_group`, `members`, `inclusive_groups`, `inclusive_members`
- `panels`, `add_panels`, `remove_panels`, `inclusive_panels`

### EventRoom

**`EventRoomCommonData`** (`pub`). Maps to **Rooms** sheet:

- `room_name: String` — must match Room column in Schedule sheet (required, indexed)
- `long_name: Option<String>` — display name shown in widget (indexed)
- `sort_key: Option<i64>` — values ≥ 100 are hidden from public schedule

**`EventRoomInternalData`** (`pub(crate)`) — `EntityType::InternalData`:

- `data: EventRoomCommonData`
- `code: EventRoomId`

**`EventRoomData`** (`pub`) — export/API view:

- `data: EventRoomCommonData`
- `code: String`
- `hotel_room_ids: Vec<HotelRoomId>` — from edge maps
- `panels: Vec<PanelId>` — from edge maps

Computed edge-backed fields (stubs until FEATURE-018): `hotel_rooms`, `panels`

### HotelRoom

**`HotelRoomCommonData`** (`pub`). Sourced from **Hotel Room** column of Rooms sheet:

- `hotel_room_name: String` (required, indexed)

**`HotelRoomInternalData`** (`pub(crate)`) — `EntityType::InternalData`:

- `data: HotelRoomCommonData`
- `code: HotelRoomId`

**`HotelRoomData`** (`pub`) — export/API view:

- `data: HotelRoomCommonData`
- `code: String`
- `event_rooms: Vec<EventRoomId>` — from edge maps (reverse lookup)

Computed edge-backed field (stub until FEATURE-018): `event_rooms`

## Acceptance Criteria

- All entity data structs compile with serde
- Field descriptors read/write correctly
- PresenterRank serializes/deserializes
- Unit tests for each entity's field read/write
- Serialization round-trip tests
