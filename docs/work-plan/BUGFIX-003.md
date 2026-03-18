# Do not list events as free

## Summary

Remove "free" labeling from events as all events require registration.

## Status

Completed

## Priority

High

## Description

Currently, some events are marked as "free" which misleads attendees. All events require convention registration, only paid workshops have additional costs.

## Implementation Details

- ~~Remove any "free" indicators from event display~~
- ~~Update cost filtering to show "Included with registration" instead of "Free"~~
- ~~Only highlight events with additional costs (workshops)~~
- ~~Update any documentation or help text regarding event costs~~

## Resolution

Removed the "Free" badge from both event cards and the detail modal in
cosam-calendar.js. Renamed the cost filter chip from "Free" to "Included"
(filter value `'free'` → `'included'`) and "Paid" to "Additional Cost".
The CSS class `.cosam-badge-free` is now unused but retained for
backward compatibility.
