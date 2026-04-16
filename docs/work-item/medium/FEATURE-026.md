# Multi-Year Schedule Archive Support

## Summary

Support multiple convention years in a single schedule file for historical
reference and jump-starting new conventions.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-025: Internal schedule file format

## Description

A schedule archive contains multiple years of convention data in one file,
enabling:

- **Jump-start**: Copy entities from a prior year to pre-populate the next
  convention (recurring panels, returning presenters, same rooms)
- **Historical reference**: View past schedules alongside the current one

## Acceptance Criteria

- Can store multiple years in one file
- Jump-start creates a new year from a prior year correctly
- Historical data is read-only
