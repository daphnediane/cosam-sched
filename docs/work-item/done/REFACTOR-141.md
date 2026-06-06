# REFACTOR-141: Remove presenter sort_index and XLSX sort-key infrastructure

## Summary

Remove the presenter `sort_index` field and the XLSX `xlsx_sort_key` sidecar
infrastructure that fed it, so presenters order deterministically by rank then
name with nothing carried over from spreadsheet column/row position.

## Status

Completed

## Priority

Low

## Description

REFACTOR-140 added `PresenterCommonData::cmp_for_display` with a
rank → `sort_index` → name ordering, where `sort_index` was assigned after import
from each presenter's source position in the spreadsheet (tracked in the sidecar
as `xlsx_sort_key`). In practice this let the spreadsheet's column/row layout
silently reorder the exported presenter lists (widget JSON, People-sheet export,
panel credits), which surprised users.

Rather than keep the machinery dormant, remove it entirely. Any future ordering
scheme is expected to differ from import-position ordering, so the `sort_index`
field, its CRDT storage, and the whole `xlsx_sort_key` →
`normalize_presenter_sort_indices` pipeline are dead weight. Ordering within a
rank is now purely alphabetical by name; the `priority()` rank-tier ordering
(guest > judge > staff > invited > panelist > fan panelist) is unchanged.

## Implementation Details

- `tables/presenter.rs`: remove the `sort_index` field from
  `PresenterCommonData`, the `FIELD_SORT_INDEX` descriptor and its inventory
  registration, and the `sort_index` tie-breaker from `cmp_for_display`
  (now rank → name). Drop the `sort_index` serde tests and update the field-set
  count (13 → 12).
- `sidecar/mod.rs`: remove the `XlsxSortKey` type alias and the
  `xlsx_sort_key` field from `EntitySidecar`.
- `xlsx/read/mod.rs`: remove `normalize_presenter_sort_indices` and its call in
  `finalize`.
- `xlsx/read/people.rs` and `xlsx/read/schedule.rs`: drop the `(col, row,
  sub_col)` key plumbing — `record_presenter` / `import_member` / `import_group`
  no longer take or store a sort key.
- `widget_json/import.rs`: stop writing the widget `sortKey` into a presenter
  field on import. The widget JSON still emits `sortKey` on export, derived from
  the rank-then-name display order (it is no longer round-tripped to an internal
  field).
- Docs: update architecture, crdt-design, field-comparison, and widget-json
  formats to drop `sort_index` / `xlsx_sort_key`.

## Acceptance Criteria

- `cargo build --workspace` and `cargo test -p schedule-core` are green; clippy
  clean.
- Within a rank, exported presenters sort alphabetically by name.
- No `sort_index` / `xlsx_sort_key` / `XlsxSortKey` references remain in code.
- Importing the same XLSX twice still yields identical presenter entities.
