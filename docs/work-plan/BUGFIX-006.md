# Detect and warn about scheduling conflicts

## Summary

The converter does not detect or report scheduling conflicts such as a presenter double-booked across overlapping events, or two non-break events in the same room at the same time.

## Status

Completed

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

**✅ COMPLETED:**

1. **Added conflict detection function** to `Convert::Events::detect_conflicts()`:
   - Builds presenter and room indexes
   - Checks for overlapping events (skipping break events)
   - Emits detailed warnings to STDERR

2. **Enhanced JSON output** in `schedule_to_json`:
   - Added `conflicts` array to top-level JSON structure
   - Added `conflicts` array to each event with references to conflicting events
   - Each conflict includes type, details, and conflicting event ID

3. **Warning format** as specified:

   ```text
   WARNING: Presenter "Jane Doe" is double-booked:
     FP032 "Foam Armor 101" (10:00-11:00, room 10)
     GP045 "Guest Q&A" (10:30-11:30, room 1)

   WARNING: Room conflict in room 10:
     FP032 "Foam Armor 101" (10:00-11:00)
     FW019 "Advanced Sewing" (10:00-11:00)
   ```

4. **Break event handling**: Break events are excluded from conflict detection as expected

5. **Multi-way conflict support**: Enhanced to handle 3+ events conflicting in same room or with same presenter:
   - Groups overlapping events and reports all pairwise conflicts
   - Special warning format for multi-way conflicts:

   ```text
   WARNING: Presenter "Name" has N-way booking conflict:
     EVENT1 "Name" (time, room)
     EVENT2 "Name" (time, room)
     EVENT3 "Name" (time, room)
   ```

### Widget (future, optional)

- The JSON now includes conflict data that the widget can use for visual indicators
- Each event has a `conflicts` array with details about what it conflicts with
- Top-level `conflicts` array provides complete conflict overview

## Testing

- Create test data with intentional presenter double-booking → verify warning
- Create test data with room conflict → verify warning
- Create test data with break overlapping real event → verify NO warning
- Verify normal non-conflicting data produces no warnings
