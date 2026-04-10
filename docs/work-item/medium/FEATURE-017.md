# XLSX Spreadsheet Import

## Summary

Import schedule data from the existing XLSX spreadsheet format.

## Status

Open

## Priority

Medium

## Description

The primary data source is an Excel spreadsheet maintained by the convention
organizers. Import must handle the existing column layout documented in
`docs/spreadsheet-format.md`.

### Import Process

1. Read XLSX file (using `umya-spreadsheet` or similar)
2. Detect sheet types (panels, presenters, rooms, panel types, etc.)
3. Map spreadsheet columns to entity fields using canonical header names
4. Create entities with appropriate UUIDs
5. Build edges from relationship columns (presenter lists, room assignments,
   panel type prefixes)
6. Handle multi-part/multi-session panels (base UID, part num, session num)
7. Import presenter grouping relationships
8. Preserve extra/unknown columns as metadata

### Import Options

- Merge mode: update existing entities vs. replace all
- Column mapping overrides
- Sheet selection (which sheets to import)

### Error Handling

- Report unmapped columns
- Report validation errors (missing required fields)
- Continue on non-fatal errors with a summary report

## Acceptance Criteria

- Successfully imports the standard convention spreadsheet
- All entity types are created with correct field values
- Relationships are correctly established
- Extra columns are preserved as metadata
- Import report lists any warnings or errors
- Unit tests with fixture spreadsheets
