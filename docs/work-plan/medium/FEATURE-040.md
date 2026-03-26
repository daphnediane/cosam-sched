# Duration/End Time Conflict Detection

## Summary

Implement conflict detection and recording when both duration and end time are specified in XLSX data but result in different effective end times.

## Status

📋 **Ready** - Design specified, implementation pending

## Priority

Medium

## Description

When reading XLSX schedule data, conflicts can occur when both duration and end time are specified but don't align. The current logic prioritizes duration over end time but doesn't record the conflict for user visibility.

## Implementation Details

### Current State

- XLSX reading prioritizes duration over end time
- Conflict detection logic stubbed with TODO comments
- No user-facing visibility of timing conflicts

### Required Changes

#### 1. Conflict Data Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConflict {
    pub panel_id: String,
    pub conflict_type: TimingConflictType,
    pub specified_end_time: NaiveDateTime,
    pub calculated_end_time: NaiveDateTime,
    pub duration_used: Duration,
    pub severity: ConflictSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimingConflictType {
    DurationEndMismatch,
    InvalidDuration,
    InvalidEndTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Warning,   // Minor discrepancy
    Error,     // Major inconsistency
}
```

#### 2. Conflict Collection

- Add `conflicts: Vec<TimingConflict>` field to Panel or Schedule
- Update XLSX reading to populate conflicts array
- Implement conflict detection in `xlsx/read/schedule.rs`

#### 3. Conflict Reporting

- Display conflicts in editor UI
- Include conflicts in JSON export (per-event conflicts array)
- Add conflict summary in schedule validation

#### 4. User Resolution Options

- Show conflicting values side-by-side
- Allow user to choose which value to trust
- Auto-correct options for common patterns

### Files to Modify

- `crates/schedule-core/src/data/time.rs` - Add conflict structures
- `crates/schedule-core/src/data/panel.rs` - Add conflicts field
- `crates/schedule-core/src/xlsx/read/schedule.rs` - Implement conflict detection
- `crates/schedule-core/src/data/schedule.rs` - Conflict aggregation
- `crates/schedule-core/src/data/display_export.rs` - Export conflicts
- Editor UI components - Conflict display and resolution

### Acceptance Criteria

- [ ] TimingConflict data structure defined
- [ ] Conflict detection implemented in XLSX reading
- [ ] Conflicts stored in Panel/Schedule structures
- [ ] Conflicts exported in JSON format
- [ ] Editor UI displays timing conflicts
- [ ] User can resolve conflicts through UI
- [ ] Conflict severity classification implemented
- [ ] Tests cover conflict scenarios

### Examples

#### Conflict Scenario 1

```text
Start: 10:00 AM
Duration: 60 minutes
End: 11:15 AM
```

**Result**:

- Uses 60min duration
- Records conflict: "Specified end 11:15 AM ≠ calculated end 11:00 AM"
- Severity: Warning

#### Conflict Scenario 2

```text
Start: 10:00 AM
Duration: -30 minutes (invalid)
End: 11:00 AM
```

**Result**:

- Falls back to end time (60min session)
- Records conflict: "Invalid duration -30min, used end time instead"
- Severity: Error

### Notes

- This complements the existing room conflict detection system
- Conflicts should be preserved through schedule editing
- Consider adding conflict resolution suggestions
- Integration with existing JSON conflicts array format
