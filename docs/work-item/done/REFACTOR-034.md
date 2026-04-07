# HotelRoom Entity Field Alignment

## Summary

Align HotelRoom entity field aliases with schedule-core canonical column definitions.

## Status

Completed

## Priority

High

## Description

Ensure HotelRoom entity field aliases include canonical forms from schedule-core for proper field resolution.

## Implementation Details

### Verify and Update Field Aliases

- `hotel_room`: Add canonical "Hotel_Room" to aliases
- `sort_key`: Add canonical "Sort_Key" to aliases

## Acceptance Criteria

- All field aliases include canonical forms from schedule-core
- HotelRoom entity compiles and passes tests
