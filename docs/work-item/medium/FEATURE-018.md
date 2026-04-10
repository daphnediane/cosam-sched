# XLSX Spreadsheet Export

## Summary

Export schedule data back to the XLSX spreadsheet format.

## Status

Open

## Priority

Medium

## Description

Export the schedule to an Excel spreadsheet matching the convention's expected
column layout, enabling round-trip with the import (FEATURE-017).

### Export Modes

- **Fresh export**: Create a new XLSX with all data
- **Update in-place**: Modify an existing XLSX, preserving formatting, formulas,
  and extra columns where possible

### Export Process

1. Map entity fields back to spreadsheet columns
2. Write panels with denormalized presenter names, room names, panel type prefixes
3. Write presenter sheets with grouping information
4. Write room and panel type reference sheets
5. Preserve extra/metadata columns in their original positions
6. Apply table formatting for Excel table features

### Considerations

- Maintain compatibility with the convention's existing spreadsheet workflow
- Handle multi-part/multi-session panel numbering
- Presenter sort order (by rank, then name)
- Formula preservation in update mode

## Acceptance Criteria

- Exported XLSX opens correctly in Excel
- Round-trip (import → export) preserves all standard columns
- Update mode preserves formatting and formulas
- Extra columns survive round-trip
- Unit tests for export and round-trip scenarios
