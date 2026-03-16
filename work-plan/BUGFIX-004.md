# Hide staff only / private events from converted JSON

## Summary

Filter out internal staff events from the public schedule JSON using the "Hidden" field in PanelTypes sheet and add `--staff` option to include private events.

## Status

Completed

## Priority

High

## Description

Staff-only events are being included in the public JSON output. These should be filtered out during conversion to maintain privacy and reduce clutter. The PanelTypes sheet already has a "Hidden" column for this purpose.

## Implementation Details

### PanelTypes Hidden Field

1. **Use existing "Hidden" column** in PanelTypes sheet:
   - Non-blank "Hidden" field indicates private/staff-only events
   - Filter these events out by default in public JSON output
   - Examples: Staff meetings, setup events, private functions

2. **Update Events.pm filtering**:
   - Check `panel_type->{ is_hidden }` during event processing
   - Skip hidden events when building public events list
   - Log hidden events that were filtered (optional)

### Staff Mode Option

1. **Add `--staff` command line flag**:

   ```bash
   schedule_to_json --staff --input schedule.xlsx --output staff_schedule.json
   ```

   When `--staff` is specified, include hidden events. Useful for internal staff schedules and planning. Default behavior (no `--staff`) excludes hidden events.

2. **Update schedule_to_json script**:

   - Add `staff` parameter to GetOptions
   - Pass staff flag to Events processing
   - Update output documentation to reflect staff mode

3. **Add validation checks**:

   - Verify no hidden events in public JSON output
   - Optional: Count and report filtered hidden events
   - Test both public and staff modes

## Acceptance Criteria

- Public JSON excludes events with "Hidden" = non-blank in PanelTypes
- `--staff` flag includes hidden events for internal use
- No staff data leaks in default public output
- Clear documentation of hidden vs public event distinction
