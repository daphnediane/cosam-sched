# FEATURE-082: Extended Entity Metadata (Unknown XLSX Columns)

## Summary

Preserve unknown XLSX columns across import/export without encoding them as
first-class entity fields, and decide how this interacts with CRDT merge.

## Status

Open

## Priority

Medium

## Description

When importing an XLSX spreadsheet, columns that are not recognized by the
importer (e.g., custom convention-specific fields, computed legacy columns like
`Lstart`/`Lend`, or future columns not yet in the schema) are currently silently
dropped. For round-trip fidelity (import → edit → export → same spreadsheet) they
should be preserved.

### Design options

1. **JSON blob field per entity** — Store `extra_data: Option<serde_json::Value>` in
   each entity's `InternalData` with a single FieldDescriptor.  CRDT: last-write-wins
   on the whole blob.  Problem: merging two replicas that each added different extra
   columns overwrites one side.

2. **Per-key CRDT entries** — For each extra column, write a named entry in a CRDT
   Map nested under the entity.  Each key has independent LWW semantics.  Rich merge
   behavior but requires extending the field system to support dynamic-keyed maps,
   which automerge natively supports (`AutoCommit::put` on nested maps).

3. **Sidecar store** (alongside SourceInfo from IDEA-081) — A UUID-indexed
   `HashMap<NonNilUuid, HashMap<String, ExtraValue>>` outside the CRDT doc.  Simple
   to implement; no CRDT complexity; never merged.  Re-import overwrites extra data.
   Acceptable if the workflow is always "import wins" for spreadsheet-resident data.

4. **Formula preservation** — Extra columns that contain Excel formulas
   (`ExtraValue::Formula { formula, value }`) need special handling: the formula
   string must round-trip through export so the spreadsheet recalculates correctly.

### Recommendation

Start with option 3 (sidecar alongside IDEA-081's SourceInfo) as it is simplest
and sufficient for the current round-trip use case.  Upgrade to option 2 if
merge-import (IDEA-080) becomes a priority, since cross-replica merging of extra
fields would require per-key CRDT entries.

**Also relates to**: `PresenterSortRank` persistence — `sort_rank` on
`PresenterCommonData` currently has no FieldDescriptor, so it is not mirrored to
the CRDT doc and is lost on save/load.  Adding a FieldDescriptor that serializes
`PresenterSortRank` as a compact string (e.g. `"col,row,member"`) would fix this.

## Acceptance Criteria

- Unknown XLSX columns survive an import → export round-trip
- Formula values (formula string + evaluated result) are preserved
- Merge behavior is defined (even if "last import wins")
- PresenterSortRank has a FieldDescriptor so it persists across save/load
