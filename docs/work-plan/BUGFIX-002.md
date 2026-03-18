# Don't show breaks when any filter besides room is selected

## Summary

Break events should only be visible when filtering by room or when no filters are applied.

## Status

Completed

## Priority

Medium

## Description

Currently, break events appear regardless of active filters (except room filter). This creates confusion as breaks should only show in the context of room schedules, not when filtering by type, cost, or presenter.

## Implementation Details

- ~~Modify filteredEvents() in cosam-calendar.js~~
- ~~Add logic to exclude break events when any non-room filter is active~~
- ~~Ensure room filter still shows breaks appropriately with colspan logic~~

## Resolution

Removed `this._isBreakEvent(e) ||` pass-through from search, type, cost, and
presenter filter clauses in `filteredEvents()`. Breaks now only pass through
the room filter, so they disappear when any other filter is active.
