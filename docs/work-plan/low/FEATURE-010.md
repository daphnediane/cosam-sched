# Conflict Detection System

## Summary

Implement room and presenter conflict detection with support for room-wide event exemptions.

## Status

Not Started

## Priority

High

## Description

Port and improve conflict detection from schedule-core and the Perl converter into schedule-data. Detect room conflicts and presenter double-bookings, with proper handling of room-wide events (from old FEATURE-035).

## Implementation Details

- Detect room conflicts: two non-room-hours panels in the same event room with overlapping time ranges
- Detect presenter conflicts: same presenter assigned to overlapping panels
- Room-wide event exemptions: panels with `is_room_hours` panel type do not conflict with non-room-hours panels in the same room
- Room-hours vs room-hours conflicts still detected
- Duration/end time mismatch detection during import (from old FEATURE-040)
- Store conflicts per-panel and as schedule-level summary
- Conflict data included in both full JSON and display JSON exports
- Support conflict severity levels (warning vs error)

## Acceptance Criteria

- Room conflicts correctly detected between overlapping non-room-hours panels
- Presenter conflicts detected for double-booked presenters
- Room-wide events (is_room_hours) exempt from conflicts with subpanels
- Conflicts stored on both individual panels and schedule-level
- Conflict data round-trips through JSON serialization
- Duration/end time mismatches recorded as timing conflicts
