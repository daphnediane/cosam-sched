# XLSX Import/Export for Schedule-Data

## Summary

Implement XLSX reading and writing against schedule-data entities, replacing schedule-core's xlsx module.

## Status

Not Started

## Priority

High

## Description

Port XLSX import/export from `schedule-core/src/xlsx/` into schedule-data, reading spreadsheet rows into entity Data structs via the field system and writing them back. The ScheduleFile abstraction should be the single entry point for all file I/O (from old CLEANUP-028).

## Implementation Details

- Port column definitions from `schedule-core/src/xlsx/columns.rs` to align with schedule-data field aliases
- Read XLSX sheets into schedule-data entities using field-system-aware row mapping
- Write schedule-data entities back to XLSX with proper column ordering
- Unified `ScheduleFile` entry point for load/save with format detection (XLSX vs JSON)
- Handle `metadata`/extras: preserve non-standard columns through round-trip (from old FEATURE-019)
- Duration/end time conflict detection during import (from old FEATURE-040)
- Support panel splitting (SPLIT events) during import
- Maintain backward compatibility with existing spreadsheet layouts

## Acceptance Criteria

- XLSX import populates schedule-data `Schedule` with all entity types and edges
- XLSX export produces spreadsheet matching original layout
- Round-trip: XLSX → Schedule → XLSX preserves all data
- Non-standard columns preserved in entity metadata
- Duration/end time conflicts detected and recorded during import
- `ScheduleFile` is the single public entry point for file I/O
