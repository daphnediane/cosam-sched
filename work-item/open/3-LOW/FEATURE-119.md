# FEATURE-119: cosam-viewer — My Schedule bookmarking

## Summary

Allow attendees to star/bookmark panels and view a personal schedule, mirroring
the JS widget's named-schedule feature.

## Status

Open

## Priority

Low

## Description

Add panel bookmarking to cosam-viewer so users can build a personal schedule.
On desktop, persist to a local file or app-data directory. On mobile, use
platform storage. Optionally support URL-hash sharing (as in the JS widget).

## Implementation Details

- Add `starred: HashSet<String>` (panel IDs) to `ViewerState`
- Add star button to panel cards and detail modal
- Add "My Schedule" filter toggle to toolbar (show starred only)
- Desktop: persist to `<data_dir>/cosam-viewer/starred.json` via `directories` crate
- Mobile: use platform-appropriate path from `dirs` or equivalent

## Acceptance Criteria

- Star button on each panel card toggles bookmarked state
- "My Schedule" toggle in toolbar filters to starred panels only
- Starred state persists across app restarts (desktop)
