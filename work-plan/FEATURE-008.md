# Allow room-wide events with subpanel overlaps

## Summary

Enable room-wide events like Market Expo to overlap with subpanels in the same room without triggering false conflict warnings.

## Status

Open

## Priority

High

## Description

Currently the converter flags conflicts when room-wide events (like Market Expo) overlap with scheduled subpanels (like Learn to solder workshops) in the same room. These overlaps are intentional - the room-wide event marks the overall operating hours while subpanels are specific activities within that timeframe.

The 2025 schedule shows this pattern:

- ME100 "Market Expo" (13:00-18:00) in room 15
- FD001S1 "Learn to solder" (14:00-16:00) in room 15
- ME101 "Market Expo" (10:00-19:00) in room 15  
- FD001S2 "Learn to solder" (10:00-12:00) in room 15
- FD001S3 "Learn to solder" (14:00-16:00) in room 15

## Expected behavior

- Room-wide events should not conflict with subpanels in the same room
- Subpanels should not conflict with each other (unless they actually overlap)
- Room-wide events should still conflict with other room-wide events in the same room
- Need a way to distinguish room-wide events from regular events

## Implementation Details

### Panel Type Enhancement

Use the new `Is Room Hours` field in PanelTypes sheet to identify room-wide events:

1. **Update PanelType structure**:
   - Add `is_room_hours` boolean field (mapped from "Is Room Hours" column)
   - Set for Market Expo and similar room-hour events
   - Future: RH* prefix events will have "Is Room Hours" = non-blank

2. **Update conflict detection logic** in `Convert::Events::detect_conflicts()`:
   - Skip conflicts where one event has `is_room_hours` = true and the other doesn't
   - Still detect conflicts between two room-hours events in same room
   - Still detect conflicts between two non-room-hours events in same room

3. **Panel Type configuration**:
   - Add "Is Room Hours" flag to Market Expo panel types in PanelTypes sheet
   - Consider other room-hours events like "Registration", "Dealer Hall", etc.
   - Future: RH* prefix will be standard for room-hours events

### Alternative: ID Prefix Pattern (fallback)

If PanelTypes sheet changes are complex, use ID prefix patterns (fallback):

- Future: RH* prefix will be standard for room-hours events
- Skip conflicts between room-hours events and non-room-hours events in same room
- Keep conflicts between room-hours events and other room-hours events

### Testing

- Verify Market Expo + Learn to solder combinations no longer warn
- Verify two Learn to solder sessions at same time still warn
- Verify two Market Expo events at same time still warn
- Test with both room-wide flag and prefix pattern approaches

## Acceptance Criteria

- No warnings for Market Expo overlapping with subpanels in same room
- Still warnings for actual scheduling conflicts
- Backward compatibility with existing schedules
- Clear documentation of what constitutes room-wide vs subpanel events
