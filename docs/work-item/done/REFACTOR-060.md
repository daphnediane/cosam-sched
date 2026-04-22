# REFACTOR-060: Edge metadata infrastructure — `EdgeDescriptor.fields` + `credited` flag

## Summary

Add per-edge data infrastructure to `EdgeDescriptor` and implement `credited: bool`
on the Panel ↔ Presenter relationship so individual presenters can be excluded
from credits without hiding all credits for the panel.

## Status

Completed

## Priority

Medium

## Description

The `EdgeDescriptor` struct (introduced in REFACTOR-059) reserves no slot for
per-edge fields. All five existing descriptors pass `fields: &[]`. This refactor:

1. Adds `EdgeFieldSpec` and `EdgeFieldDefault` types to `edge_descriptor.rs`.
2. Adds `fields: &'static [EdgeFieldSpec]` to `EdgeDescriptor`; all existing
   descriptors keep `fields: &[]` except `EDGE_PRESENTERS` which gains a
   `credited` boolean field defaulting to `true`.
3. Adds `read_edge_meta_bool` / `write_edge_meta_bool` helpers to `edge_crdt.rs`
   that read/write a parallel `{field_name}_meta` automerge `ObjType::Map` keyed
   by target UUID string, with each value being a nested map of per-edge scalars.
4. Adds `edge_get_bool<L,R>` / `edge_set_bool<L,R>` to `Schedule`.
5. Adds `FIELD_SET_PRESENTER_CREDITED` write field on `PanelEntityType`
   (two-item list: `[EntityIdentifier, Boolean]`) and updates `FIELD_CREDITS`
   to call `edge_get_bool` and skip uncredited presenters.

### CRDT Storage Layout

```text
entities/panel/{uuid}/
  presenters        ObjType::List   ← existing membership list
  presenters_meta   ObjType::Map    ← NEW
    "{presenter_uuid}": ObjType::Map
      "credited": bool               ← LWW scalar; absent == default (true)
```

Removing a presenter leaves the meta entry as a harmless tombstone.

### `FIELD_SET_PRESENTER_CREDITED` Encoding

Write value is a two-item `FieldValue::List`:

- `[0]` — `FieldValueItem::EntityIdentifier(presenter_id)`
- `[1]` — `FieldValueItem::Boolean(credited_value)`

## Acceptance Criteria

- [x] `EdgeDescriptor` has `fields: &'static [EdgeFieldSpec]`; all five existing
  descriptors compile with `fields: &[]`
- [x] `EDGE_PRESENTERS.fields` contains `{ name: "credited", default: Boolean(true) }`
- [x] `edge_get_bool` returns `true` (default) when no meta entry exists
- [x] `edge_set_bool` / `edge_get_bool` round-trip correctly
- [x] Automerge save/load round-trip preserves per-edge data
- [x] `FIELD_CREDITS` excludes presenters where `credited == false`
- [x] `credited_presenters` / `uncredited_presenters` read/write fields implemented
- [x] `add_credited_presenters` / `add_uncredited_presenters` write fields implemented
- [x] `cargo test` passes (460 tests), `cargo clippy` clean

## Notes

- `RawEdgeMap`, `HomoEdgeCache`, and the generic `edge_add`/`edge_remove`/`edge_set`
  API are unchanged (Option A from edge-descriptors-orig.md).
- `FIELD_SET_PRESENTER_CREDITED` (two-item list encoding) was replaced by the
  cleaner `credited_presenters` / `uncredited_presenters` partition model.
- REFACTOR-058 (credits display update) is unblocked; it can now narrow its scope
  to documenting the integration and any UI changes needed.
