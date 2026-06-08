# IDEA-080: Update Schedule from Spreadsheet (Merge Import)

## Summary

Design for merging a new XLSX import into an existing CRDT-tracked schedule
rather than always starting from a clean slate.

## Status

Open

## Priority

Low

## Description

The current `import_xlsx` implementation always creates a fresh `Schedule` from
scratch. The convention workflow involves iterative edits to a live spreadsheet,
and it would be useful to re-import without losing manual edits made inside the
editor (e.g., notes, tags, or structural changes applied after the last import).

A merge-based import would:

- Treat the XLSX as the authoritative source for spreadsheet-resident fields
  (name, times, rooms, panelists, costs, etc.)
- Preserve fields set only in the editor that have no spreadsheet column
- Use the existing CRDT merge infrastructure to converge the two states

This is intentionally deferred because:

- It requires careful field-ownership semantics (which fields "belong" to the
  spreadsheet vs. the editor)
- The CRDT merge model needs to be well-established first (FEATURE-022/023)
- A clean-slate import is sufficient for the current workflow

## Acceptance Criteria

- Re-import from XLSX produces correct merge with editor-only changes preserved
- Fields sourced from XLSX overwrite stale editor values when the spreadsheet
  is authoritative
- New panels/presenters added in XLSX appear; panels removed (soft-deleted) are
  marked deleted in the schedule
