# FEATURE-118: cosam-viewer — grid view (rooms × time slots)

## Summary

Add a CSS-grid schedule view to cosam-viewer mirroring the JS widget's grid mode.

## Status

Open

## Priority

High

## Description

Implement a grid view in `apps/cosam-viewer` where columns are rooms and rows are
time slots, with panels spanning multiple rows based on duration. Mirrors the
`grid` view mode of the JS widget.

## Implementation Details

- Add `ViewMode::Grid` variant to `state.rs`
- Add grid toggle button to the toolbar (List | Grid)
- Render `ui/grid.rs`: CSS Grid with sticky room-name header row and time column
- Panels span `grid-row` based on (duration / slot-size); breaks span all room columns
- Gridlines via CSS border (1px, subtle)

## Acceptance Criteria

- Grid and List toggle buttons in toolbar switch view mode
- Room names in sticky header; time slots in left column
- Panels positioned correctly by start time and room
- Panels spanning correct number of rows for their duration
