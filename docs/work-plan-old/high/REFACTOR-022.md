# Semantic Datetime and Duration Representation

## Summary

Refactor Panel time fields to use a unified TimeRange enum that eliminates inconsistency between duration and end_time fields and centralizes all timing logic.

## Status

✅ **Completed** - TimeRange fully implemented and tested

## Priority

High

## Description

Refactor the Panel struct to use a unified `TimeRange` enum instead of separate `start_time`, `end_time`, and `duration` fields, eliminating potential inconsistency and providing clearer business logic. All timing logic is now centralized in the TimeRange enum in `time.rs`.

## Implementation Details

### Core Changes

- **Panel struct fields:**
  - Removed: `start_time`, `end_time`, `duration` fields
  - Added: `timing: TimeRange` field
  - All timing methods now delegate to `panel.timing.*()`

### New TimeRange Enum

```rust
pub enum TimeRange {
    Unspecified,                           // No timing info - unscheduled
    UnspecifiedWithDuration(Duration),     // Duration only, no start time
    UnspecifiedWithStart(NaiveDateTime),  // Start time only, no duration
    Scheduled {                            // Complete timing info
        start_time: NaiveDateTime,
        duration: Duration,
    },
}
```

### Key Features Implemented

- **Validation**: Prevents negative durations and end times before start times
- **String Methods**: `start_time_str()`, `end_time_str()`, `duration_minutes_str()`
- **Parsing Methods**: `set_*_from_str()` for all time components
- **Overlap Detection**: `overlaps_with()` method for time range comparison
- **DateTime Containment**: `contains_datetime()` method
- **Robust Error Handling**: Graceful degradation to appropriate states for invalid inputs

### Files Modified

- `crates/schedule-core/src/data/time.rs` - TimeRange enum and implementation
- `crates/schedule-core/src/data/panel.rs` - Updated to use TimeRange
- `crates/schedule-core/src/data/schedule.rs` - Updated to use TimeRange methods
- `crates/schedule-core/src/data/post_process.rs` - Updated timing access
- `crates/schedule-core/src/edit/command.rs` - Updated edit commands
- `crates/schedule-core/src/xlsx/*.rs` - Updated XLSL read/write
- `crates/schedule-core/src/data/display_export.rs` - Updated export logic

### Testing

- ✅ 10 comprehensive TimeRange tests covering all functionality
- ✅ 9 Panel integration tests
- ✅ All tests passing

### Updated Semantic Methods

- `effective_duration()` → Simple pattern match on `TimeRange`
- `effective_end_time()` → Simple pattern match on `TimeRange`
- `effective_duration_minutes()` → Convenience method for backward compatibility

### Flexible Edit Support

- `set_duration()` → Sets duration in TimeRange, transitions between states as needed
- `set_end_time()` → Sets end time, preserves duration when available
- `set_start_time()` → Sets start time, transitions between states appropriately
- String convenience methods maintained for backward compatibility

### Architecture Benefits

- **Eliminates Inconsistency**: Cannot have conflicting duration/end_time
- **Centralized Logic**: All timing operations in one TimeRange enum
- **Clear Business Logic**: Enum explicitly models timing specification method
- **Simpler Validation**: Type system ensures only one timing method is active
- **Better Reusability**: TimeRange can be used independently of Panel
- **Robust Error Handling**: Graceful degradation for invalid inputs

### Infrastructure Updates

- ✅ Updated serialization to handle `TimeRange` enum
- ✅ Updated `SessionScheduleState` and edit commands
- ✅ Updated XLSX I/O to work with new timing model
- ✅ Removed redundant `normalize_event_times()` function
- ✅ Centralized string formatting/parsing in TimeRange

## Acceptance Criteria

- [x] Panel struct uses semantic chrono types
- [x] TimeRange enum designed and implemented
- [x] Panel struct updated to use TimeRange instead of separate fields
- [x] All setters and getters updated to work with TimeRange
- [x] Custom serialization handles TimeRange enum
- [x] Edit commands updated to maintain timing consistency
- [x] XLSX I/O updated to handle TimeRange
- [x] All tests updated and passing (19/19 tests passing)
- [x] All compilation errors resolved

## Notes

- ✅ TimeRange enum eliminates the root cause of duration/end_time inconsistency
- ✅ Setters automatically switch timing modes, preventing invalid states
- ✅ Backward compatibility maintained through Panel convenience methods
- ✅ Business logic becomes much clearer through explicit timing modes
- ✅ String methods centralized in TimeRange for better reusability
- ✅ Comprehensive test coverage ensures reliability
- ✅ Option A implemented: invalid end times transition to Unspecified
