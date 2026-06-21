# FEATURE-154: Migrate widget time strings from naive ISO 8601 to epoch seconds

## Summary

Replace naive ISO 8601 datetime strings (e.g., `"2026-06-26T14:00:00"`) with Unix epoch
seconds (seconds since 1970-01-01 UTC) across the widget JSON and widget-html formats
to eliminate timezone-related parsing complexity and ambiguity.

## Status

In Progress

## Phasing

Migration is staged to keep the build green at every step:

1. **Phase 1 (done):** Add epoch-seconds fields (`startEpoch`/`endEpoch`) alongside
   the naive ISO strings in the DTO, export, and widget HTML `data-*`; bump format
   version to 2. Purely additive — all consumers keep reading the ISO strings.
2. **Phase 2 (done):** Migrate the JS widget off the ISO strings. `_normalizeDataModel`
   now derives wall-clock display times from `startEpoch`/`endEpoch` interpreted in
   `meta.timezone` (via `Intl`), falling back to the naive ISO strings on pre-v2 data.
   The widget-html embed loader parses the `data-*-epoch` attributes.
3. **Phase 3:** Migrate the Rust consumers (`schedule-layout`, `cosam-viewer`) to
   compute wall-clock from epoch + `meta.timezone`, **and** remove the ISO string
   fields. Because REFACTOR-153 unified the DTO, these consumers share the wire
   struct, so removing the field and removing the dependency on it are the same step.
4. **Phase 4:** Rename the epoch fields back to the canonical `startTime`/`endTime`
   keys (now integers).

## Priority

Medium

## Description

The current widget formats use naive ISO 8601 datetime strings for all time fields
(`startTime`, `endTime` in panels/timeline, `startTime`/`endTime` in meta). These
strings are wall-clock values in the `timezone` specified in meta (e.g.,
`"America/New_York"`). This approach has several problems:

- **Ambiguity**: Without timezone context, the string is meaningless
- **Parsing complexity**: Consumers must correctly combine naive string + timezone field
- **Library differences**: Different JS/other language libraries handle naive datetimes inconsistently
- **Error-prone**: Easy to forget to apply timezone when parsing/serializing

Since the format already records a `timezone` field in meta (IANA name like
`"America/New_York"`), we can use epoch seconds as the canonical time representation
and convert to local time for display using the timezone field.

## Plan

### Format Changes

**Widget JSON Format (`widget-json-format.md`):**

- Change `meta.startTime` and `meta.endTime` from string to integer (epoch seconds)
- Change `panels[].startTime` and `panels[].endTime` from string to integer
- Change `timeline[].startTime` from string to integer
- Keep `meta.timezone` and `meta.vtimezone` unchanged (used for display formatting and iCalendar generation)
- Update format version from 1 to 2

**Widget HTML Format (`widget-html-format.md`):**

- Change `data-start-time` and `data-end-time` attributes from string to integer
- Keep the `datetime` attribute on `<time>` elements as a local ISO string for SEO/HTML5 compliance
- Update format version in meta from 1 to 2

### Code Changes

**Rust (`cosam-convert`):**

- Update `DisplayPanel`, `TimelineEntry`, and `Meta` types to use `i64` for time fields
- Update serialization logic to emit epoch seconds instead of naive ISO strings
- Update `static_html.rs` to write integer attributes and keep `datetime` as local ISO for HTML5
- Bump widget format version to 2

**JavaScript (`cosam-calendar.js`):**

- Update `_normalizeDataModel()` to handle integer time fields
- Update time parsing logic to convert epoch seconds to local time using `meta.timezone`
- Update time formatting/display logic to work with epoch seconds
- Add backward compatibility for format version 1 (parse naive ISO strings if version < 2)

### Migration Strategy

- Bump format version to 2 to allow consumers to detect the change
- Include backward compatibility in the widget JavaScript to handle version 1 files
- Update all documentation to reflect the new format
- No data migration needed for existing schedules (regenerate exports with new `cosam-convert`)

## Acceptance Criteria

- [x] `cosam-convert` emits epoch seconds for all time fields in both JSON and HTML formats (Phase 1)
- [x] Widget JavaScript correctly parses epoch seconds and converts to local time using `meta.timezone` (Phase 2)
- [x] Widget JavaScript maintains backward compatibility with format version 1 (naive ISO strings) (Phase 2)
- [x] `widget-json-format.md` documents the new integer time fields and format version 2 (Phase 1)
- [x] `widget-html-format.md` documents the new integer `data-*` attributes and format version 2 (Phase 1)
- [x] Time display in the widget (day tabs, time ranges, grid axis) works correctly with epoch seconds (Phase 2)
- [ ] iCalendar generation (`.ics` download) continues to work correctly using `timezone` + `vtimezone`
- [ ] All existing test exports are regenerated with the new format

## Notes

- Epoch seconds are universally understood across platforms and languages
- The `timezone` field remains essential for converting epoch seconds to human-readable local time
- The `datetime` attribute on HTML `<time>` elements should remain as an ISO string for HTML5/SEO compliance
- This change does not affect the internal CRDT-based storage format (`field-system.md`)
