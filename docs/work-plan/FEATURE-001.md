# Interactive event calendar with spreadsheet-to-JSON converter

## Summary

Implement a two-part system for Cosplay America schedule management.

## Status

Completed

## Priority

High

## Description

Added Perl converter tool and embeddable vanilla JS calendar widget with grid/list views, filtering, bookmarks, and print support.

## Implementation Details

- Created Perl converter (converter/schedule_to_json) that reads XLSX spreadsheets
- Added converter library modules under converter/lib/Convert/
- Created embeddable calendar widget (widget/cosam-calendar.js) as a self-contained IIFE
- Added responsive CSS with print styles and theming support
- Implemented day tabs, filters, bookmarks, and event detail modal
- Added sample data and embed.html for testing

## Completed

2026-03-15 - Initial implementation completed
