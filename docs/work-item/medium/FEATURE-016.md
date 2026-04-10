# Widget Display JSON Export

## Summary

Implement export of schedule data to the JSON format consumed by the calendar
display widget.

## Status

Open

## Priority

Medium

## Description

The calendar widget (in `widget/`) renders schedule data from a JSON file.
This work item defines and implements the new export format.

### Format Design

- Clean break from v9/v10; new version number
- Public-only data (no internal notes, admin fields, CRDT state)
- Flat event list with denormalized room names, presenter names, panel type info
- Timeline markers for day boundaries
- Room list with sort order
- Panel type list with colors and prefixes
- Presenter list (credited only)
- Schedule metadata (title, dates, generator)

### Multi-Year Support

- Optional multi-year output (see FEATURE-015)
- Year selector metadata for the widget

### Backward Compatibility

- The widget should be updated to consume the new format
- Consider a compatibility shim or version detection in the widget JS

## Acceptance Criteria

- Export produces valid JSON consumed by the widget
- All scheduled events appear with correct times, rooms, and presenters
- Unscheduled events are omitted from display export
- Multi-year export works when archive has multiple years
- Widget renders the exported data correctly
