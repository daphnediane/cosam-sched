# FEATURE-084: XLSX Spreadsheet Update (In-Place Save)

## Summary

Implement `update_xlsx` to write schedule changes back into an existing XLSX
file, preserving formatting, formulas, extra columns, and non-standard content.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-029: XLSX spreadsheet export ✓ resolved

## Description

`export_xlsx` (FEATURE-029) always writes a fresh workbook from scratch.
`update_xlsx` would instead open the original file and patch only the rows that
changed, preserving:

- Cell formatting (colors, fonts, borders)
- Formula cells the user has added (e.g., conditional-format helpers)
- Extra non-standard columns (custom per-convention data)
- Timestamp and Grid sheets
- Non-imported sheets that we never touch

This is the workflow convention staff actually uses: import once to seed the
schedule database, then save back repeatedly as edits accumulate.

### What v10-try1 implemented

The `v10-try1` worktree has a working reference in
`crates/schedule-core/src/xlsx/write/update.rs` (~1,300 lines). Key behaviors:

- Reads the existing workbook with `umya_spreadsheet::reader::xlsx::read`.
- Dispatches by sheet: Schedule, Rooms, PanelTypes, People are each updated
  separately; the sheet name is recovered from `SourceInfo.sheet_name` stored
  at import time.
- Change-state routing: each entity carries a `ChangeState` enum
  (Added / Modified / Replaced / Deleted / Unchanged); the update function
  only touches rows whose state is not `Unchanged`.
- Schedule sheet: **never deletes rows** — deleted panels are marked in place
  by prefixing their Uniq ID with `*`; the old ID is written to the
  `Old Uniq Id` column.
- Rooms / PanelTypes sheets: deleted rows are physically removed (reverse
  order to avoid index drift), and new rows are appended.
- Presenter columns: writes to existing columns only; never creates new
  presenter columns for newly-significant presenters.
- ExtraFields: preserves arbitrary extra columns that were imported but are not
  modeled, by writing them back from a per-entity `metadata: IndexMap` populated
  at import time.
- Atomic write: writes to `.xlsx.tmp` then renames into place.
- Safety checks: detects Office lock files (`~$filename`) and external
  modification (mtime comparison before/after processing).
- After a successful save, `post_save_cleanup` removes deleted entities and
  resets all change states to `Unchanged`.

### Limitations encountered in v10-try1 that must be addressed

1. **Row index drift after deletion.** When rows are removed from the Rooms or
   PanelTypes sheet, `SourceInfo.row_index` values for entities below the
   deletion point become stale. v10-try1 mitigates this by deleting from the
   bottom up, but does not update the stored indices — a second round of
   changes after a deletion could target the wrong rows.

2. **No new presenter columns on update.** The update path never adds presenter
   columns to the Schedule sheet. A newly-significant presenter (≥ 3 panels)
   who had no named column in the original file will only appear in the
   existing `Other` column until the user does a full re-export. This is a
   deliberate conservative choice in v10-try1 but should be documented clearly
   as a known limitation of the update path.

3. **Change state tracking must be derived from CRDT.** The main branch uses
   Automerge CRDT for all mutation tracking — there is no `ChangeState` enum
   on entities. Before `update_xlsx` can be implemented, we need a mechanism
   to classify entities as Added / Modified / Deleted relative to the last
   saved state. Candidate approaches:
   - Snapshot the CRDT heads after each successful save and diff on the next
     save.
   - Maintain a sidecar map of entity UUID → last-saved field digest (see
     IDEA-081 for a related UUID-indexed sidecar concept).
   - Use automerge's change log directly if it exposes per-entity diffs cleanly.

4. **SourceInfo (sheet name + row index) is absent from the main data model.**
   Without knowing which sheet row corresponds to each entity, we cannot locate
   the row to update. This is a prerequisite — likely via an extension to the
   IDEA-081 sidecar or a dedicated lightweight import-provenance store.

5. **ExtraFields / custom-column preservation is absent.** Main entities have
   no `metadata: IndexMap` carrying unmodeled import columns. Supporting this
   would require the importer to capture unknown columns and the entity model
   to carry them through the CRDT — a non-trivial change.

6. **Table area resize fragility.** `update_table_areas` in v10-try1 assumes
   one table per sheet starting at row 1. This breaks if a user has added a
   second table or pivot table to the sheet. A more robust approach would
   locate each table by its named range and resize only that.

### Suggested approach for main

Given the CRDT architecture, the minimum viable `update_xlsx` in main requires:

1. A mechanism to classify which entities changed since the last save.
2. A `SourceInfo` sidecar mapping entity UUIDs to `(sheet_name, row_index)`.
3. The update logic itself (adapt v10-try1's `update.rs` to the main entity
   model and edge API).

Items 1 and 2 are design work adjacent to IDEA-081 and should be resolved as
prerequisites before this feature is scheduled.

## Acceptance Criteria

- `update_xlsx(schedule, path)` opens an existing XLSX file and patches only
  changed rows, leaving unchanged rows, formatting, and non-standard columns
  intact.
- Deleted panels are soft-deleted (Uniq ID prefixed with `*`); deleted rooms
  and panel types have their rows removed.
- New entities are appended to the appropriate sheet.
- Atomic write (temp file + rename) and Office lock-file detection are
  implemented.
- Integration tests verify: import → modify → update → re-import shows the
  expected changes with no spurious modifications to unchanged rows.
