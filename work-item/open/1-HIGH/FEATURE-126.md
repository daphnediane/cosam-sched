# FEATURE-126: Widget JSON update-mode import with data preservation

## Summary

Add update-mode (upsert + soft-delete) semantics to widget JSON import,
analogous to what FEATURE-122 did for XLSX, with extra care to preserve
schedule data that the lossy widget JSON format does not carry.

## Status

Open

## Priority

High

## Blocked By (optional)

- REFACTOR-125: Common import infrastructure extraction (see Description)

## Description

`import_from_widget_json` currently creates a fresh `Schedule` from
widget JSON. It cannot be used to update an existing schedule because:

- it always allocates new UUIDs (losing CRDT history)
- it does not match or merge entities by natural key
- it silently drops fields that widget JSON does not carry

This feature brings widget JSON import up to the same standard as the
XLSX update-mode added in FEATURE-122:

### Update-mode semantics

- Match entities by natural key (panel code, room name, presenter name,
  panel type prefix) rather than always creating new ones.
- Upsert: update matched entities in place; create genuinely new ones.
- Soft-delete entities present before the import but absent from the
  widget JSON (rooms, panel types, panels, presenters).
- Use deterministic v5 UUIDs (same derivation as XLSX import) so that
  a widget-JSON round-trip does not fragment CRDT history.
- Expose `update_schedule_from_widget_json(schedule, widget)` mirroring
  the signature of `update_schedule_from_xlsx`.

### Data preservation

Widget JSON is a display format and omits many fields that the schedule
may carry. During an update-mode import, fields **absent** from widget
JSON must be left as-is on the existing entity rather than overwritten
with a default. Fields that widget JSON *does* carry are applied
authoritatively.

Known fields to preserve (not present in widget JSON):

- `hide_panelist`, `sewing_machines`, `pre_reg_max`, `capacity`
- `notes_non_printing`, `workshop_notes`, `power_needs`, `av_notes`
- `ticket_url`, `difficulty`, `prereq`
- `for_kids`, `additional_cost`
- Formula-cell sidecar data
- `is_explicit_group`, `subsumes_members`, `show_individually` on presenters
- Hotel room metadata beyond the room name string
- Credited vs uncredited presenter distinction (widget JSON flattens
  all to credited; do not demote existing uncredited presenters)

### Common import infrastructure

This feature should be implemented on top of the common import machinery
extracted as part of the REFACTOR-125 direction:

- `collect_entity_uuids` / `soft_delete_unseen`
- `PresenterImportCache` (deferred name + rank merge)
- Before-snapshot + seen-accumulator + `finalize` lifecycle

The widget JSON importer should embed or delegate to the same common
session rather than duplicating the seen/before/soft-delete pattern.

### Presenter cache

Apply the same `PresenterImportCache` logic as XLSX:

- Widget JSON carries presenter names and implicit ranks from group
  membership context. Treat these as implicit unless the widget format
  provides an unambiguous rank signal.
- Preserve ranks already stored that exceed what widget JSON implies.

## Acceptance Criteria

- Re-importing the same widget JSON over an existing schedule is
  idempotent (no spurious CRDT entries, no data loss on preserved fields).
- Fields absent from widget JSON are not touched on existing entities.
- Presenters, panels, rooms absent from widget JSON are soft-deleted.
- UUIDs are stable across repeated imports of the same widget JSON.
