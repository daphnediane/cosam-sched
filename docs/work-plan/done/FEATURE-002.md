# Handle SPLIT and BREAK special events

## Summary

Filter out SPLIT page-break markers and display BREAK time slots stretched across rooms.

## Status

Completed

## Priority

High

## Description

Added special event handling in converter and widget, with BREAK events spanning columns and exception handling for overlapping events.

## Implementation Details

- Converter: Skip events where room is "SPLIT"
- Widget: Filter out SPLIT events defensively
- BREAK events span across all room columns via colspan
- Handle exceptions where real events overlap breaks
- Add CSS styling with diagonal stripe patterns for breaks

## Completed

2026-03-15 - Special event handling implemented
