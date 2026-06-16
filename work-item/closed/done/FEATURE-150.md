# FEATURE-150: Timezone information in schedules

## Summary

Record a timezone (and overridable start/end bounds) in schedule metadata and
thread it through every export so .ics calendar files anchor correctly.

## Status

Completed

## Priority

Medium

## Description

Commit c9dee9a added client-side .ics export, but it had to assume a timezone:
schedule times are naive wall-clock and the widget emitted floating DATE-TIME
values, so events landed at the viewer device's local time. This adds a timezone
to the canonical metadata, threads it through .cosam / widget JSON / widget HTML /
a new XLSX `Meta` sheet, and uses it to emit TZID + VTIMEZONE anchored .ics.

The rule going forward: any timestamp without a zone is interpreted as being in
the meta timezone.

## Implementation Details

- Add `timezone`, `start_time`, `end_time` to `ScheduleMetadata` (all optional).
- New `value/timezone.rs`: `parse_tz` (chrono-tz + daylight-abbrev alias table),
  `local_tz_name` (iana-time-zone), `resolve_timezone`, `build_vtimezone`.
- `WidgetMeta` gains `timezone` + `vtimezone`; widget HTML inherits via meta clone.
- Widget `buildIcs` emits `DTSTART;TZID=...` + embedded VTIMEZONE.
- XLSX `Meta` sheet: written on export, read on import (authoritative round-trip).
- cosam-convert: `--default-timezone` / `--default-start-time` / `--default-end-time`.
- cosam-modify: `--set-timezone` / `--set-start-time` / `--set-end-time`.

Resolution precedence per field: Meta sheet → CLI default → file hint → local
(tz) / computed panel bounds (start/end). Start/end are a window that extends to
cover any panel scheduled outside it.

## Acceptance Criteria

- ScheduleMetadata round-trips the new fields through .cosam.
- Widget JSON/HTML meta carry `timezone` + `vtimezone`.
- Single-event .ics contains a VTIMEZONE block and TZID-qualified DTSTART/DTEND.
- XLSX export/import round-trips the Meta sheet.
- cosam-convert defaults and cosam-modify sets both work; no source → local tz.

## Notes

Out of scope: natural-language time resolution ("Friday 8 pm"), full POSIX TZ
string parsing.
