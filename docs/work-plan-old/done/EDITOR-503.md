# Conflict Detection

## Summary

Detect and highlight scheduling conflicts between events.

## Status

Completed

## Priority

Medium

## Description

Automatically identify events that overlap in the same room or involve the same presenter at the same time. Display conflicts visually and provide a summary view.

## Implementation Details

- Room conflicts: two events in the same room with overlapping times
- Presenter conflicts: same presenter assigned to overlapping events
- Visual indicators on conflicting event cards (warning icon, border color)
- Conflict summary panel listing all detected issues
- Real-time detection as events are edited

## Implementation

- Added conflict detection for panel-based schedule structure
- Detects room conflicts by finding overlapping sessions in same room
- Detects presenter conflicts by finding overlapping sessions with same presenter
- Supports both individual presenters and group presenters
- Adds conflicts to both top-level schedule conflicts and per-session conflicts
- Integrates with existing post-processing pipeline

## Notes

Conflict detection now works for both legacy events structure and new panel-based structure. Conflicts are stored in PanelSession.conflicts and Schedule.conflicts.
