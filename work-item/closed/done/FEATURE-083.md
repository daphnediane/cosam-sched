# FEATURE-083: Separate Hotel Room sheet in XLSX import/export

## Summary

Add a dedicated `Hotels` sheet to the XLSX format for richer hotel-room metadata.

## Status

Completed

## Priority

Low

## Description

Currently hotel rooms are expressed as a single column (`Hotel Room`) in the Rooms sheet,
limited to one hotel room name per event room. A dedicated `Hotels` sheet would allow richer
metadata (sort key, long name, notes) and cleaner round-trips, mirroring how rooms and panel
types already get their own sheets.

Proposed sheet name: `Hotels` (with `Hotel Rooms` as a fallback alias).

Proposed columns:

- Hotel Room — canonical name (key for linking from the Rooms sheet)
- Sort Key — optional integer for ordering
- Long Name — optional display name

## Implementation notes:

- Sheet name: "Hotel" (singular) - import also accepts "Hotel Rooms" and "HotelMap" as aliases
- Columns: Hotel Room, Sort Key, Long Name (all three proposed columns implemented)
- Import: `read/hotel_rooms.rs` reads the Hotel sheet and creates `HotelRoomEntityType` entities
- Export: `write_hotel_rooms_sheet()` in `xlsx/write/export.rs` writes the Hotel sheet
- The Hotel Room column in the Rooms sheet is still written (not suppressed as proposed)
- `HotelRoomEntityType` entity in `tables/hotel_room.rs` with full field and edge support
- `EDGE_HOTEL_ROOMS` relationship in `event_room.rs` and `HALF_EDGE_HOTEL_ROOMS` in `hotel_room.rs`
- Column definitions in `xlsx/columns.rs::hotel_rooms` module
