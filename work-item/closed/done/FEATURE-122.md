# FEATURE-122: Import Update — Merge Imports Into Existing Schedule

## Summary

Replace all "import → new schedule" functions with "update existing schedule"
variants; the old functions become thin wrappers that create a blank schedule
and delegate.

## Status

Completed

## Priority

High

## Description

Implements the import-side complement to FEATURE-084. Resolves IDEA-080.

Key behaviors:

- **Upsert semantics**: entities matched by natural key (v5 UUID from code/name)
  are updated in place; new entities are created. Existing UUIDs are preserved.
- **Soft-delete**: after processing each sheet, entities of that type that were
  not seen in the import are soft-deleted (`remove_entity`).
- **Widget JSON special care**: when updating from widget JSON (a display-only
  format that lacks uncredited-presenter data), uncredited presenter edges on
  existing panels are preserved. Presenter soft-delete is skipped because widget
  JSON does not carry uncredited-only presenters.
- **Convenience wrappers**: `import_xlsx`, `import_from_widget_json`, and
  `import_csv` create a blank schedule then call the update variant.

## Implementation Details

- Add `upsert_entity<E>` to `edit/builder.rs`: compute v5 UUID, update if
  exists, create if not.
- Change each `read_X_into` sub-function signature to return a
  `HashSet<NonNilUuid>` of seen UUIDs in addition to current return value.
- Use `edge_set` (replace) instead of `edge_add` for edges on updated entities.
- In `update_schedule_from_xlsx`: collect entity UUIDs before import, compare
  after, soft-delete the difference (skipped sheets are exempt).
- In `update_schedule_from_widget_json`: similar, but preserve uncredited
  presenter edges per panel and skip presenter soft-delete.

## Acceptance Criteria

- Re-importing the same XLSX twice leaves the schedule unchanged (idempotent).
- Panels removed from XLSX (or soft-deleted with `*` prefix) are soft-deleted
  in the schedule.
- Panels present in both old and new XLSX retain their original UUID.
- Widget JSON update preserves uncredited presenters for existing panels.
