# FEATURE-115: Separate Timeline Sheet in XLSX

## Summary

Separate Timeline Sheet in XLSX

## Status

Completed

## Priority

Medium

## See also

- FEATURE-083: Separate Hotel Room sheet in XLSX import/export

## Description

Add a dedicated Timeline sheet to the XLSX format to separate timeline events from regular panels. This aligns with the new Timeline entity type and simplifies the data model.

## Motivation

Currently timeline events are stored as Panel entities with `is_timeline` panel type flag. The widget export filters these out separately. With the new Timeline entity type, we can:

- Store timelines as their own entity type
- Have a dedicated Timeline sheet in XLSX
- Simplify panel-related code that currently filters for/against timelines
- Make the data model more explicit and type-safe

## Requirements

- XLSX import reads Timeline sheet and creates Timeline entities
- XLSX export writes Timeline entities to a dedicated Timeline sheet
- Timeline sheet includes: code, name, description, note, time, panel_types
- Widget export uses Timeline entities directly instead of filtering panels
- Update panel-related code to leverage timeline separation where applicable

## Implementation Notes

**Actual implementation:**
- `TimelineEntityType` entity in `tables/timeline.rs` with full field and edge support
- Export: `write_timeline_sheet()` in `xlsx/write/export.rs` writes dedicated Timeline sheet
- Import: `xlsx/read/timeline.rs` reads Timeline sheet and creates Timeline entities
- Timeline sheet includes: code, name, description, note, time, panel_types (all requirements met)
- Timeline entities have single time point (no duration field)
- Panel types associated via `HALF_EDGE_PANEL_TYPES` edge
- Timelines also written to Schedule sheet for backward compatibility (lines 402-678 in export.rs)
- Timeline rows in Schedule sheet have blank room, duration, presenter columns, and formulas
- Grid export skips timeline panels (line 1065-1067 in export.rs)
