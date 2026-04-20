# Builder Pattern

## Summary

Implement entity builders for constructing entity data with UUID assignment.

## Status

Completed

## Related Work Items

- FEATURE-046: `FieldSet::write_multiple` — underlying batch-write primitive

## Priority

Medium

## Blocked By

- FEATURE-014: PanelType entity (proof of concept)

## Description

The old proc-macro generated per-entity builders with `with_*` setters and
`build()` methods. Without the macro, builders need explicit implementation.

### Options

1. **Generic builder** — A single `EntityBuilder<E>` that accepts field name/value
   pairs and constructs `E::Data`. Leverages `FieldSet<E>` for validation.
2. **Per-entity builders** — Hand-written `PanelBuilder`, `PresenterBuilder`, etc.
   with typed setter methods. More ergonomic but more boilerplate.
3. **Macro-assisted** — `macro_rules!` generates builder from a field list.

### UUID assignment

Builders accept a `UuidPreference` parameter (see FEATURE-012):

- `GenerateNew` *(default)* — v7 UUID; for new entities
- `FromV5 { name }` — v5 from natural key; for spreadsheet imports so the
  same source row always maps to the same UUID across re-imports
- `Exact(uuid)` — for round-tripping serialized data

### Evaluate after FEATURE-014

Decide which builder approach to use after seeing the PanelType proof of concept.

## Acceptance Criteria

- [x] Can construct any entity data struct through the builder
- [x] Builder validates required fields before build
- [x] UUID assignment follows v7/v5 rules
- [x] Unit tests for builder construction and validation

## Resolution

Chose the **macro-assisted** approach (option 3). `define_entity_builder!` in
`field_macros.rs` generates a typed builder with `with_*` setters per field,
delegating to `build_entity` (in `builder.rs`) which seeds default data,
applies batched writes via `FieldSet::write_multiple`, runs verification, and
rolls back on failure.

Builders instantiated:

- `PanelTypeBuilder` (with comprehensive unit tests)
- `PanelBuilder`
- `PresenterBuilder`
- `EventRoomBuilder`
- `HotelRoomBuilder`

Follow-up: improve rustdoc for generated `with_*` setters so the rendered docs
show details about the underlying field (deferred; tracked separately).
