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

PanelType is the simplest entity (~10 stored fields, 1 computed) and serves as
the proof of concept for the FieldDescriptor approach.

### Data struct

`PanelTypeData` — hand-written, visible, with serde:

- `entity_id: EntityId<PanelTypeEntityType>`
- `prefix: Option<String>`
- `panel_kind: Option<String>`
- `hidden: bool`
- `is_workshop: bool`
- `is_talk: bool`
- `is_gaming: bool`
- `is_video: bool`
- `is_performance: bool`
- `is_photoshoot: bool`
- Additional fields as needed

### Field descriptors

~12 static `FieldDescriptor<PanelTypeEntityType>` values, one per field.
Computed field `display_name` derives from `panel_kind` / `prefix`.

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
