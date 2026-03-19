# XLSX Export Support

## Summary

Add the ability to export schedule data to XLSX spreadsheets.

## Status

Open

## Priority

High

## Description

Implement writing schedule data to XLSX files using the `rust_xlsxwriter` crate. This allows round-tripping data back to spreadsheet format for sharing with non-technical staff.

## Implementation Details

- Use `rust_xlsxwriter` to generate XLSX output
- Write Schedule, Rooms, and PanelTypes as separate sheets
- Preserve formatting conventions (header rows, column ordering)
- Support "Save As" dialog for choosing output path
- Validate data before export (warn on missing required fields)
