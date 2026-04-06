# Integration Testing and Schedule-Core Parity Validation

## Summary

Comprehensive integration tests validating schedule-data against schedule-core behavior with real schedule data.

## Status

Not Started

## Priority

High

## Description

Before decommissioning schedule-core, validate that schedule-data produces equivalent results for all supported operations using representative real-world schedule data (e.g., 2025 convention schedule).

## Implementation Details

- Round-trip tests: XLSX → schedule-data → JSON matches XLSX → schedule-core → JSON
- Entity parity: all panels, presenters, rooms, panel types, edges match between systems
- Conflict detection parity: same conflicts detected by both systems
- Credit resolution parity: same presenter credits generated
- Query parity: find/lookup operations return equivalent results
- Edit parity: same edit operations produce same mutations
- Performance benchmarking: schedule-data vs schedule-core for load, query, export
- Edge case coverage: empty schedules, missing fields, malformed data

## Acceptance Criteria

- All entity counts match between schedule-core and schedule-data for test data
- JSON export diff is empty or contains only expected format differences
- Conflict detection produces identical results
- Performance is comparable or better
- Edge cases handled gracefully
