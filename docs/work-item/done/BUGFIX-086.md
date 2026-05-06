# BUGFIX-086: Room filter chips show no text; hotel room missing

## Summary

Room filter chips are blank and hotel room context is absent because the new
export format uses camelCase field names that the widget doesn't handle.

## Status

Completed

## Priority

High

## Blocked By (optional)

N/A

## Description

The cosam-convert JSON export uses camelCase room fields (`shortName`,
`longName`, `hotelRoom`, `sortKey`, `isBreak`) while the widget expects
snake_case (`short_name`, `long_name`, `hotel_room`, `sort_key`, `is_break`).
As a result, room filter chips render with no visible text.

Additionally, the hotel room name (e.g. "Salon F/G") is not shown alongside
the room short/long name in the filter chips, making it hard for attendees to
identify rooms.

## How Found

Manual testing of the widget loaded with cosam-convert output.

## Reproduction

1. Load the widget with a v0 JSON export from cosam-convert.
2. Open the Filters panel.
3. Observe the Room section: chips are present but completely empty.

**Expected:** Room chips show name and hotel room (e.g. "Main (Salon F/G)")

**Actual:** Room chips are blank

## Steps to Fix

Normalize room objects in `_normalizeDataModel` to camelCase, handling both
old snake_case and new camelCase input. Update all downstream room field
references in the widget to use the normalized camelCase fields. Add hotel
room in parentheses to filter chips when it differs from the display name.

## Testing

Load widget with v0 export; verify room chips show name and hotel room.
Verify room filter still correctly hides/shows events.
