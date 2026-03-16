# XLSX Import Support

## Summary

Add the ability to import schedule data from XLSX spreadsheets.

## Status

Open

## Priority

High

## Description

Implement reading XLSX files using the `calamine` crate, parsing the Schedule, Rooms, and PanelTypes sheets into the existing data model. This enables direct editing of spreadsheet-sourced data without going through the Perl converter first.

## Implementation Details

- Enable the `xlsx` feature flag in Cargo.toml
- Parse the "Schedule", "Rooms", and "PanelTypes" sheets per `docs/spreadsheet-format.md`
- Map spreadsheet columns to existing Rust data structs
- Handle missing/malformed cells gracefully with error reporting
- Add file dialog support for opening `.xlsx` files
- Unit tests with sample spreadsheet fixtures
