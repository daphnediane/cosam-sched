# FEATURE-105: Widget @media print CSS for grid view

## Summary

Improve the widget's browser print output so the grid view prints cleanly with proper column layout, hidden chrome, and expanded descriptions.

## Status

Completed

## Priority

High

## Description

The current `_handlePrint()` always produces a list-view output regardless of the active view mode. When the user is in grid view, printing should render the grid cleanly.

## Implementation Details

- Add `@media print` CSS rules in `widget/cosam-calendar.css` scoped under `.cosam-calendar`:
  - Hide toolbar, filters, day tabs, modals, star buttons
  - Grid: freeze column widths, `break-inside: avoid` on event cells
  - Expand all `.cosam-event-desc` elements
  - Day headers trigger `break-before: page`
  - Force `print-color-adjust: exact` for panel type colors
- Modify `_handlePrint()` in `widget/cosam-calendar.js`: if current view is `grid`, build grid print content (all days) instead of list

## Acceptance Criteria

- Grid view prints with room columns and time rows visible
- No toolbar/filter/modal chrome in print output
- Descriptions shown expanded in print
- List view printing behavior unchanged
