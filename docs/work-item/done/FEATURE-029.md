# XLSX Spreadsheet Export

## Summary

Export schedule data back to the XLSX spreadsheet format.

## Status

Completed

## Priority

Medium

## Blocked By

- FEATURE-028: XLSX spreadsheet import

## Description

Export the schedule to an Excel spreadsheet matching the convention's expected
column layout, enabling round-trip with the import (FEATURE-028).

Implemented in `crates/schedule-core/src/xlsx/write/` with four sheets:
Schedule, Rooms, People, and PanelTypes. Dynamic presenter columns are generated
from panel attendance counts (named column threshold: 3+ panels or always_grouped).
Lstart/Lend formula columns are written with both formula and calculated values
for Excel compatibility.

## Acceptance Criteria

- Export to XLSX matches expected column layout ✓
- Round-trip import → export produces equivalent data ✓
- Integration tests with fixture files ✓
