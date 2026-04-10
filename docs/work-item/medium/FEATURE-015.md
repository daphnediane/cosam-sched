# Multi-Year Schedule Archive Support

## Summary

Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.

## Status

Open

## Priority

Medium

## Description

A schedule archive contains multiple years of convention data in one file,
enabling:

- **Jump-start**: Copy entities from a prior year to pre-populate the next
  convention (e.g., recurring panels, returning presenters, same rooms)
- **Historical reference**: View past schedules alongside the current one
- **Widget display**: Optionally serve multi-year data to the calendar widget

### Data Model

- Each year's schedule is a self-contained `Schedule` with its own entities,
  edges, and metadata
- An `Archive` wrapper contains a map of year → Schedule
- The "active" year is marked for editing; other years are read-only views
- Entity UUIDs are globally unique across years (v7 UUIDs guarantee this)

### Operations

- `create_new_year(source_year)` — clone selected entities from a prior year
  into a new schedule, generating fresh UUIDs
- `import_year(file)` — add a year from an external file
- `export_year(year)` — extract a single year as a standalone file
- Configurable which entity types to carry forward (rooms and panel types
  are likely; panels and presenters may be selective)

### Widget Integration

- Widget JSON export (FEATURE-016) can include multiple years or a single year
- Year selector in the widget UI

## Acceptance Criteria

- Archive can hold multiple years
- Jump-start creates a new year with copied entities and fresh UUIDs
- Single-year export produces a valid standalone schedule
- Widget export supports year filtering
- Unit tests for archive CRUD and jump-start
