# Detect and warn about scheduling conflicts

## Summary

The converter does not detect or report scheduling conflicts such as a presenter double-booked across overlapping events, or two non-break events in the same room at the same time.

## Status

Open

## Priority

Medium

## Description

When building the schedule spreadsheet, mistakes happen — a presenter may be
marked as attending two events that overlap in time, or two events may be
accidentally assigned to the same room at the same time.

Currently the converter silently produces JSON with these conflicts, and the
widget displays overlapping events without any indication that something is
wrong. Neither tool provides any warning to the schedule author.

### Expected behavior

- **Presenter conflict**: If the same presenter name appears in two events
  whose time ranges overlap, the converter should emit a warning to STDERR.
  The widget could optionally show a visual indicator.

- **Room conflict**: If two non-break events share the same room and their
  time ranges overlap, the converter should emit a warning.

- **Break overlap is allowed**: Break events are expected to overlap with
  real events (e.g. "Costume Contest Staging" during "Dinner Break"). These
  should NOT be flagged as conflicts.

### Scope

This is primarily a **converter-side warning** to help catch spreadsheet
mistakes. The widget may optionally surface conflicts visually in a future
iteration, but the core deliverable is converter warnings.

## Implementation Details

### Converter (`Events.pm` or `schedule_to_json`)

1. After all events are parsed, build indexes:
   - By presenter: `{ presenter_name => [ list of (start, end, event_id) ] }`
   - By room: `{ room_id => [ list of (start, end, event_id) ] }`

2. For each presenter, sort their events by start time and check for overlaps.
   Skip any event where `is_break` is true.

3. For each room, sort events by start time and check for overlaps.
   Skip any pair where either event has `is_break` set.

4. Emit warnings to STDERR in a clear format:

   ```text
   WARNING: Presenter "Jane Doe" is double-booked:
     FP032 "Foam Armor 101" (Sat 10:00-11:00, Panel Room 1)
     GP045 "Guest Q&A" (Sat 10:30-11:30, Main)

   WARNING: Room conflict in "Panel Room 1":
     FP032 "Foam Armor 101" (Sat 10:00-11:00)
     FW019 "Advanced Sewing" (Sat 10:00-11:00)
   ```

5. Optionally add a `--strict` flag that turns warnings into errors
   (non-zero exit code).

### Widget (future, optional)

- Could add a `conflicts` array to the JSON output that the widget reads
- Visual indicator (e.g. warning icon) on conflicting events
- Tooltip explaining the conflict

## Testing

- Create test data with intentional presenter double-booking → verify warning
- Create test data with room conflict → verify warning
- Create test data with break overlapping real event → verify NO warning
- Verify normal non-conflicting data produces no warnings
