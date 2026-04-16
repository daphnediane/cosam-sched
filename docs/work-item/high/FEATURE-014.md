# PanelType Entity (Proof of Concept)

## Summary

Implement the PanelType entity as the first proof of concept for the no-proc-macro field system.

## Status

Open

## Priority

High

## Blocked By

- FEATURE-013: FieldSet registry

## Description

PanelType is the simplest entity (~13 stored fields, 1 edge computed) and serves as
the proof of concept for the FieldDescriptor approach.

### Three structs

**`PanelTypeCommonData`** (`pub`) — hand-written, serializable, user-facing fields
from the **PanelTypes** sheet:

- `prefix: String` — two-letter Uniq ID prefix (required, indexed)
- `panel_kind: String` — human-readable kind name (required, indexed)
- `hidden: bool`
- `is_workshop: bool`
- `is_break: bool`
- `is_cafe: bool`
- `is_room_hours: bool`
- `is_timeline: bool`
- `is_private: bool`
- `color: Option<String>` — CSS color (e.g. `"#db2777"`)
- `bw: Option<String>` — alternate monochrome color

**`PanelTypeInternalData`** (`pub(crate)`) — `EntityType::InternalData`; the field system operates on this:

- `data: PanelTypeCommonData`
- `code: PanelTypeId`

**`PanelTypeData`** (`pub`) — export/API view, produced by `export(&Schedule)`:

- `data: PanelTypeCommonData`
- `code: String` — stringified `PanelTypeId`
- `panels: Vec<PanelId>` — assembled from edge maps

### Field descriptors

~13 static `FieldDescriptor<PanelTypeEntityType>` values; closures access
`internal.data.*` for `CommonData` fields and `internal.code` for the ID.
One edge-backed computed field `panels` (read/write, deferred to FEATURE-018).
Computed field `display_name` derives from `data.panel_kind` / `data.prefix`.

### FieldSet

Assembled manually in `LazyLock`, returned by `PanelTypeEntityType::field_set()`.

### Evaluation point

After implementing, evaluate whether a `macro_rules!` helper for field
descriptor declarations would reduce boilerplate enough to warrant adding.

## Acceptance Criteria

- PanelTypeData compiles with serde serialization
- All field descriptors read/write correctly
- FieldSet lookup by name and alias works
- Computed `display_name` field reads correctly
- Unit tests for field read, write, FieldSet lookup
- Serialization round-trip test
