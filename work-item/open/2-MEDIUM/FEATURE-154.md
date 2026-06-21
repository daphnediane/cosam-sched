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
   - Added `date: String` to `WidgetDaySpan` and `day_key: Option<String>` to `WidgetPanel`;
     stamped in Rust via `dayTimeline` range lookup (inclusive, respects `borrowedEndEpoch`
     for overnight break panels). Added `data-day-key` attribute to widget-html panel
     elements; parsed by the embed loader into `panel.dayKey`.
   - Regression tests for overnight-break `day_key` stamping (panel ending at midnight,
     borrowed session after midnight).
   - Replaced the ISO-string slot-key pipeline in `_buildGridView` with epoch-minute keys:
     `epochToSlotEpoch` / `slotEpochToName` helpers; `evenSlotEpochs` /
     `getIntermediateSlotEpochs` replacing four functions that had a hardcoded
     `2026-06-25` reference date; O(1) row-span lookup; day headers from `dayTimeline`
     range lookup; `_buildGridHeader` takes a `dayTimeline` entry directly.
   - List view groups by epoch slot key; day boundary via `evt.dayKey`.
   - Fixed `formatTime`, `formatTimeGrid`, `formatTimeSplit` to parse hours/minutes
     from ISO substring instead of `new Date(naiveIso)` (avoids browser-local drift).
3. **Phase 3:** Migrate the Rust consumers (`schedule-layout`, `cosam-viewer`) to
   compute wall-clock from epoch + `meta.timezone`, **and** remove the ISO string
   fields. Because REFACTOR-153 unified the DTO, these consumers share the wire
   struct, so removing the field and removing the dependency on it are the same step.
4. **Phase 4:** Rename the epoch fields back to the canonical `startTime`/`endTime`
   keys (now integers).

## Implementation Details

### Completed — Even time axis infrastructure (schedule-layout + schedule-core)

The Rust side of the even-time-axis grid (needed by the simple print and the
Typst layout) was plumbed through `schedule-layout/src/timegrid.rs` and
`schedule-core/src/value/timezone.rs`:

- `TimeSlot` gained an `epoch: i64` field (minute-aligned Unix epoch) alongside
  the ISO `key` string.
- `GridLayout::compute_even` added: fills intermediate time slots at the GCD
  of the local minute-of-hour across regular events (clamped 15–60 min), using
  precomputed TZ offsets from `WidgetMeta` so IST (UTC+5:30) and other
  non-integer-hour zones are handled correctly.
- `is_major` is now computed via epoch arithmetic
  `(epoch + tz_offset_secs) % 3600 == 0` using the precomputed TZ offset (and
  DST transition) from `WidgetMeta`, replacing the fragile ISO-suffix check.
- Private helpers added: `slot_epoch`, `is_major_slot`, `gcd`, `grid_unit_secs`.
- `local_iso_to_epoch(iso, tz_name) -> Option<i64>` added to
  `schedule-core/src/value/timezone.rs` (inverse of `epoch_to_local_iso`).

### Completed — Epoch-window refactor in `document.rs`

Section window bounds in `schedule-layout/src/document.rs` are now epoch `i64`
throughout, and the Day/HalfDay splits consume the precomputed `day_timeline` /
`half_day_timeline` (bucketing by `panel.day_key`). `Timeline`/`Custom` splits
and `spanning_into` compare epochs directly. `group_by_day`, `unique_days`, and
`split_halves` were removed.

**Behavior note:** the only render change is the half-day schedule grid
(`schedule-tabloid`): single-half days now read `"Thursday"`/`"Friday"` (the
timeline label) instead of `"Thursday PM"`/`"Friday PM"`, and each half's grid
opens at its first content (span-clamped) rather than forcing an empty
Noon/midnight band. All day-split outputs render byte-identical. This is the
intended effect of consuming the timelines (tighter, cleaner grids).

Original plan (for reference):

**Type changes:**

- `TimedSection<'a>` type alias: change the two `Option<String>` window fields
  to `Option<i64>`.
- `Section.window_start / window_end`: `Option<String>` → `Option<i64>`.
- `GridLayout.window_start / window_end`: `Option<String>` → `Option<i64>`.
- `GridLayout::compute` / `compute_even` parameters: `Option<&str>` →
  `Option<i64>` (the `local_iso_to_epoch` call currently done inside
  `compute_inner` moves to callers).

**`TimeSplit::Day` — use `data.day_timeline`:**

- Replace `group_by_day(panels, tz)` + ISO date-string grouping with iteration
  over `data.day_timeline` (`Vec<WidgetDaySpan>`).
- Each span's `start_epoch` / `borrowed_end_epoch.unwrap_or(end_epoch)` forms
  the epoch filter window; panels where `p.start_epoch` falls in that range
  belong to the span.
- Day sections have no grid clipping: `win_start = None, win_end = None`.

**`TimeSplit::HalfDay` — use `data.half_day_timeline`:**

- Replace `split_halves` (which builds a `format!("{}T12:00", date)` ISO noon
  string) with iteration over `data.half_day_timeline`.
- AM span: `win_end = Some(span.end_epoch)` (which is already clamped to the
  noon epoch by `compute_day_spans`). `win_start = None`.
- PM span: `win_start = Some(am_span.end_epoch)` from the corresponding AM
  span (the noon epoch), `win_end = None`. When there is no AM span (pure PM
  day), `win_start = None`.
- The `split_halves` helper can be deleted.

**`TimeSplit::Timeline` — use `t.start_epoch` directly:**

- Replace `timeline_start_iso(t, tz)` (which converts `start_epoch` → ISO)
  with `t.start_epoch` in `split_on_timeline`.
- `tl_boundaries` becomes `Vec<(i64, &str)>` (epoch, label).
- `section_label_for` compares `p.start_epoch` against boundary epochs (no
  ISO conversion).
- `section_keys` windows become `Vec<(String, Option<i64>, Option<i64>)>`.
- Sort is by epoch (already guaranteed monotonic from export order).

**`TimeSplit::Custom` — epoch-based slot windows:**

- Parse config ISO strings to epochs once at entry via `local_iso_to_epoch`.
  Expose `naive_to_epoch(dt: NaiveDateTime, tz_name: &str) -> i64` from
  `schedule-core/src/value/timezone.rs` as `pub` so
  `split_on_custom_timeline` can call `parse_loose_datetime` → epoch without
  an intermediate ISO string.
- `all_slots` becomes `Vec<(i64, Option<i64>, &str)>` (start_epoch,
  end_epoch_opt, label).
- `slot_windows` becomes `Vec<(i64, Option<i64>, &str)>`.
- `section_label_for` compares `p.start_epoch` against slot epochs.
- `first_slot_start` becomes an epoch.

**`spanning_into`:**

- Signature: `(all_panels: &[&'a Panel], window_start_epoch: i64) -> Vec<&'a Panel>`.
- Filter: `p.start_epoch < window_start_epoch && p.end_epoch > window_start_epoch`
  (no ISO conversion, no `tz` parameter needed).

**Callers:**

- `build_sections` passes `section.window_start` (now `Option<i64>`) directly to
  `spanning_into` and `GridLayout::compute`.
- `meta_start_iso` / `meta_end_iso` in `model.rs` are no longer needed by
  `document.rs` for the Custom timeline path; callers should use
  `data.meta.start_epoch` / `data.meta.end_epoch` directly.

**Cleanup:**

- `panel_start_iso` / `panel_end_iso` and `timeline_start_iso` in `model.rs`
  become unused by `document.rs` once the refactor is complete (retain for any
  other callers, e.g. `blocks/grid.rs`).
- `group_by_day` / `unique_days` / `split_halves` helpers in `document.rs` can
  be deleted when no longer called.
- `local_iso_to_epoch` in `timegrid.rs` import becomes unused (remove).

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
- [x] Precomputed `day_key` on panels used for day bucketing throughout widget (Phase 2)
- [x] Grid time-slot pipeline uses epoch keys — no ISO string parsing for layout or bucketing (Phase 2)
- [x] Time display functions use ISO substring parsing, not `new Date(naiveIso)` (Phase 2)
- [x] `data-day-key` emitted in widget-html; parsed by embed loader (Phase 2)
- [x] Overnight break day_key regression tests added (Phase 2)
- [ ] iCalendar generation (`.ics` download) continues to work correctly using `timezone` + `vtimezone`
- [ ] All existing test exports are regenerated with the new format

## Notes

- Epoch seconds are universally understood across platforms and languages
- The `timezone` field remains essential for converting epoch seconds to human-readable local time
- The `datetime` attribute on HTML `<time>` elements should remain as an ISO string for HTML5/SEO compliance
- This change does not affect the internal CRDT-based storage format (`field-system.md`)
