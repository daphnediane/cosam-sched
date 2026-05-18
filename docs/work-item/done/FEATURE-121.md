# FEATURE-121: cosam-viewer multi-format schedule loading

## Summary

Expand cosam-viewer to open XLSX, binary `.cosam`, and CSV directory schedules, plus
fetch widget JSON from a webpage URL.

## Status

Completed

## Priority

Medium

## Description

cosam-viewer currently only opens widget JSON files. This adds:

- **XLSX** (`.xlsx`) — import via schedule-core XLSX reader, export to WidgetExport
- **Binary** (`.cosam` and unknown extensions) — load via `Schedule::load_from_file`,
  export to WidgetExport
- **CSV directory** — load via `import_csv`, export to WidgetExport
- **URL input** — fetch widget JSON embedded in a webpage via `load_from_url`

## Implementation Details

- `data/display.rs`: Add `ScheduleDoc::from_path()` (auto-detects format) and
  `ScheduleDoc::from_url()` methods
- `ui/app.rs`: Expand file dialog filters; add "Open Folder" button for CSV dirs;
  add URL text input in the empty state; use `tokio::task::spawn_blocking` for blocking
  I/O (XLSX, binary, CSV, URL fetch)
- `Cargo.toml`: Add `tokio` dependency for `spawn_blocking`

## Acceptance Criteria

- File dialog accepts `.xlsx`, `.cosam`, and directory (CSV) files in addition to `.json`
- "Open Folder" button opens CSV directory schedules
- URL text input fetches widget JSON from webpage URLs
- All formats display correctly in the viewer after conversion to widget JSON format
- Blocking I/O operations don't block the UI thread
