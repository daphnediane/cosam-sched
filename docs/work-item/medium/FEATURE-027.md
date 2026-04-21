# Widget Display JSON Export

## Summary

Implement export of schedule data to the JSON format consumed by the calendar display widget.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-019: Schedule container + EntityStorage
- FEATURE-056: Synthesized Data Fields for Export

## Description

The calendar widget renders schedule data from a JSON file. This work item
implements the export functionality that converts from the internal CRDT/field-system
format to the widget JSON display format (documented in `docs/widget-json-format.md`).

The export should use the public data structures (PanelTypeData, HotelRoomData, EventRoomData, etc.)
rather than InternalData, as these already contain synthesized fields like `inclusive_presenters`.
If public versions don't have data in the required format, computed fields should be added to the public data structure.

All items should use Uuid for identification. For break synthesis, Uuid v5 should be generated.
References between items should use Uuid instead of names or other IDs. Panels should have references
to both hotel and event rooms as separate records.

## Acceptance Criteria

- Export produces valid JSON matching widget schema
- All scheduled panels with times and rooms are included
- Presenter names are correctly formatted
- Credit handling and break synthesis implemented as per v9/v10 patterns
- Export function added to Schedule or appropriate module
- Export uses public data structures (PanelTypeData, HotelRoomData, EventRoomData, etc.)
- All items use Uuid for identification
- Break synthesis generates Uuid v5 for synthesized panels
- Panels have references to both hotel and event rooms
