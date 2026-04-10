# EntityFields Derive Macro

## Summary

Implement the `#[derive(EntityFields)]` proc-macro in the `schedule-macro` crate.

## Status

Completed

## Priority

High

## Description

Port and refine the `EntityFields` derive macro from the `feature/schedule-data`
experiment. The macro generates boilerplate for the entity/field system so that
entity structs remain clean and declarative.

### Generated Items

For a struct annotated with `#[derive(EntityFields)]`:

- Per-field unit structs implementing `NamedField`, `SimpleReadableField`,
  `SimpleWritableField`
- An internal `<Name>Data` storage struct with `entity_uuid: NonNilUuid`
- A `<Name>EntityType` struct implementing the `EntityType` trait
- A static `FieldSet` with name map, aliases, required fields, and indexable fields
- Field constants in a `fields` module

### Supported Attributes

- `#[field(display = "...", description = "...")]` — display metadata
- `#[alias("a", "b")]` — extra lookup names
- `#[required]` — required-field validation
- `#[indexable(priority = N)]` — match-index participation
- `#[computed_field(...)]` with `#[read(...)]` / `#[write(...)]` — schedule-aware fields
- `#[entity_kind(Kind)]` — links struct to `EntityKind` variant

## Acceptance Criteria

- Macro compiles and generates correct code for a simple test entity
- Generated code passes `cargo test` and `cargo clippy`
- Computed fields with schedule access work correctly
