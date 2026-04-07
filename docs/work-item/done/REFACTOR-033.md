# EventRoom Entity Field Alignment

## Summary

Align EventRoom entity fields with schedule-core canonical column definitions.

## Status

Completed

## Priority

High

## Description

Add canonical aliases to EventRoom entity fields, including computed fields, for proper field resolution while respecting the edge-based architecture.

## Implementation Details

### Add Aliases for Computed Fields

- Add canonical aliases to the computed `sort_key_computed` field: "Sort_Key", "SortKey", "Sort", "Order"
- Add canonical aliases to the computed `hotel_room` field: "Hotel_Room", "HotelRoom", "Hotel", "Building"
- Keep these as computed fields (via edges) - do not add as direct stored fields

### Verify and Update Field Aliases

- `short_name`: Add canonical "Room_Name" to aliases
- `long_name`: Add canonical "Long_Name" to aliases

## Acceptance Criteria

- Computed fields have canonical aliases for field resolution
- Direct field aliases include canonical forms from schedule-core
- EventRoom entity compiles and passes tests
- Edge-based architecture respected (no direct fields duplicating edge data)
