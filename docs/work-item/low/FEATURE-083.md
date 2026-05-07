# FEATURE-083: Separate Hotel Room sheet in XLSX import/export

## Summary

Add a dedicated `Hotels` sheet to the XLSX format for richer hotel-room metadata.

## Status

Open

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

Implementation notes:

- Import: teach `read/rooms.rs` to look for a Hotels sheet and create `HotelRoomEntityType`
  entities from it; the `Hotel Room` column in the Rooms sheet would still be accepted as a
  fallback for files without the separate sheet.
- Export: add `write_hotel_rooms_sheet()` in `xlsx/write/export.rs` alongside the existing
  `write_rooms_sheet()`; suppress the `Hotel Room` column from the Rooms sheet when the
  separate sheet is written.
- The `EDGE_HOTEL_ROOMS` relationship in `event_room.rs` and `columns::room_map::HOTEL_ROOM`
  are the key integration points.
