# XLSX Spreadsheet Import

## Summary

Import schedule data from the existing XLSX spreadsheet format.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-020: Query system

## Description

The primary data source is an Excel spreadsheet maintained by the convention
organizers. Import must handle the existing column layout.

## Acceptance Criteria

- Import from XLSX produces a valid schedule with all entities
- Presenter tag-string parsing creates correct entities and relationships
- Import is idempotent (re-importing same data produces same result)
- Integration tests with fixture XLSX files
