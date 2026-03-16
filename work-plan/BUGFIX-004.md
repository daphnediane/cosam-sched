# Hide staff only / private events from converted JSON

## Summary

Filter out internal staff events from the public schedule JSON.

## Status

Open

## Priority

High

## Description

Staff-only events are being included in the public JSON output. These should be filtered out during conversion to maintain privacy and reduce clutter.

## Implementation Details

- Add a flag or marker in spreadsheet for staff-only events
- Update Events.pm to skip these events during conversion
- Ensure no staff data leaks into public JSON
- Add validation to verify no private events are exported
