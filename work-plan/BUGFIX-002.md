# Don't show breaks when any filter besides room is selected

## Summary

Break events should only be visible when filtering by room or when no filters are applied.

## Status

Open

## Priority

Medium

## Description

Currently, break events appear regardless of active filters (except room filter). This creates confusion as breaks should only show in the context of room schedules, not when filtering by type, cost, or presenter.

## Implementation Details

- Modify filteredEvents() in cosam-calendar.js
- Add logic to exclude break events when any non-room filter is active
- Ensure room filter still shows breaks appropriately with colspan logic
