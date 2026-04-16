# Builder Pattern

## Summary

Implement entity builders for constructing entity data with UUID assignment.

## Status

Open

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

- UUID v7 for new entities (time-ordered)
- UUID v5 for deterministic IDs (e.g., edges keyed by endpoint pair)

### Evaluate after FEATURE-014

Decide which builder approach to use after seeing the PanelType proof of concept.

## Acceptance Criteria

- Can construct any entity data struct through the builder
- Builder validates required fields before build
- UUID assignment follows v7/v5 rules
- Unit tests for builder construction and validation
