# Synthesized Data Fields for Export

## Summary

Add computed/synthesized fields to public data structures to support widget JSON export.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-019: Schedule container + EntityStorage

## Description

The widget JSON export requires certain data that is not directly stored in the internal
entity structures but can be computed from existing fields. This work item adds computed
fields to the public data structures (PanelData, HotelRoomData, EventRoomData, etc.) to
make this data available for export.

Specific synthesized fields needed:

**PanelData:**

- `credits`: Formatted credit strings for display (hidePanelist, altPanelist, group resolution)
- `hotel_rooms`: Computed field that traverses event_rooms => hotel room edges (similar to inclusive_presenters traversal)

**Existing fields (no changes needed):**

- `inclusive_presenters`: Already exists as computed field (BFS over direct presenters + groups/members)
- `event_rooms`: Already exists as edge field to EventRoomEntityType

**PresenterData:**

- Verify existing fields meet export needs
- May need additional computed fields for bidirectional group membership

## Acceptance Criteria

- PanelData has computed field for credits with proper formatting
- PanelData has computed field for hotel_rooms that traverses event_rooms => hotel room edges
- Credit formatting implements hidePanelist, altPanelist, and group resolution logic
- Computed fields are calculated from existing internal data
- Computed fields are included in the export view (Data struct)
- Tests verify computed field correctness

## Implementation Details

- Add credits as a computed field using the field system's computed field mechanism
- Add hotel_rooms as a computed field that traverses event_rooms edges to hotel room edges
  - Similar to inclusive_presenters traversal pattern
  - For each event_room in event_rooms, follow edges to hotel rooms
- Credit formatting logic from v9/v10 should be adapted:
  - hidePanelist: Presenters who should not appear in credits
  - altPanelist: Alternative presenter names for credits
  - Group resolution: Expand groups to individual presenters
- Computed fields should be read-only in the public API

## Notes

This work item is a dependency for FEATURE-027 (Widget Display JSON Export).
The export functionality will use these computed fields instead of implementing
the logic directly in the export code.
