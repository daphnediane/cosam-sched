# IDEA-081: Import Provenance / SourceInfo Sidecar

## Summary

Track where each entity came from (file, sheet, row) in a UUID-indexed sidecar
structure separate from the CRDT schedule document.

## Status

Open

## Priority

Low

## Description

During XLSX import every entity has an origin: which file it was read from, which
sheet, and which row. This "source info" is useful for:

- Displaying provenance in the editor ("imported from 2026.xlsx row 42")
- Round-trip update workflows (knowing which entities were xlsx-imported vs.
  created in the editor)
- Future merge-import (IDEA-080): knowing a row's origin helps decide authority

**Why not in the CRDT entity?**

SourceInfo is import-specific and changes every re-import, so storing it as CRDT
fields creates unnecessary history and awkward merge semantics (two replicas that
import the same xlsx agree on source info, but a replica that created an entity
programmatically has no source info, causing spurious conflicts).

**Proposed design: UUID-indexed sidecar:**

A `HashMap<NonNilUuid, SourceInfo>` stored alongside the schedule but outside the
automerge doc. Possibilities:

- In-memory only (lost on save/load — acceptable if only used for import→export
  within one session)
- Serialized into the native file envelope (an extra JSON chunk after the automerge
  blob, indexed by UUID)
- A separate `.provenance` file alongside the `.cosam` file

The sidecar should also cover non-xlsx sources (e.g., "created in editor at time T")
so it generalizes beyond just xlsx.

**Open questions:**

- Does the sidecar need to survive save/load for the current use cases?
- Should SourceInfo be shared with the extra-metadata sidecar (IDEA-082)?
- What format: flat JSON map, or a structured envelope with version/type?

## Acceptance Criteria

- SourceInfo is tracked per imported entity
- SourceInfo survives a save/load round-trip (if applicable)
- Exporting to XLSX can use SourceInfo to preserve the original row order
