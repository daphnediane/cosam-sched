# Display JSON Export

## Summary

Implement display/public JSON export for the schedule widget, equivalent to schedule-core's display_export.

## Status

Not Started

## Priority

High

## Description

The schedule widget consumes a display-oriented JSON format (currently V10-display) that differs from the full internal format. Implement export from schedule-data's `Schedule` to this display format, porting logic from `schedule-core/src/file/display_export.rs`.

## Implementation Details

- Port display export logic from `schedule-core/src/file/display_export.rs` (43KB)
- Generate widget-consumable JSON with:
  - Panels with resolved presenter names and credits
  - Panel types with display properties (color, bw_color, hidden, etc.)
  - Rooms with sort keys and filterable flags
  - Timeline entries
  - Conflicts (room and presenter)
  - Room hours extraction (from old UI-037)
- Credit resolution: resolve group membership to public-facing names
- Exclude inactive entities from display output
- Support both minified and pretty-printed output
- Maintain backward compatibility with existing widget JavaScript

## Acceptance Criteria

- Display JSON output matches widget's expected schema
- Credit resolution handles group/member relationships correctly
- Inactive entities excluded from output
- Room hours extracted and formatted separately (from old UI-037)
- Widget renders correctly from schedule-data display export
