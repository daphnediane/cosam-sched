# XLSX Spreadsheet Export

## Summary

Export schedule data back to the XLSX spreadsheet format.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-028: XLSX spreadsheet import

## Description

Export the schedule to an Excel spreadsheet matching the convention's expected
column layout, enabling round-trip with the import (FEATURE-028).

## Acceptance Criteria

- Export to XLSX matches expected column layout
- Round-trip import → export produces equivalent data
- Integration tests with fixture files
