# IDEA-044: Reconsider `required` flag on FieldDescriptor

## Summary

The `required: bool` field on `FieldDescriptor` may conflict with design goals around soft deletion and flexible data structures.

## Status

Open

## Priority

Low

## Description

### Current State

`FieldDescriptor` has a `required: bool` field, and `FieldSet` tracks `required_fields()` — fields that must have values. Current tests enforce that `PanelType` fields like `prefix` and `panel_kind` are required.

### The Conflict

The design philosophy is:

- Data can be in temporarily invalid states
- No required fields — a panel without code or name is just soft-deleted/unscheduled
- Edges are typed; text is free-form
- Spreadsheet columns are required (must exist in XLSX), but field values are not

### Potential Problems

The `required` flag may cause issues with:

1. **Builder pattern for recreating structures**: If a builder validates `required` fields at build time, it prevents creating partial/invalid entities that are later completed
2. **`EntityType::import()` method** (opposite of `export()`): Importing data from external sources should allow partial records that get filled in later
3. **Soft deletion**: An entity with all "required" fields cleared is effectively deleted — but `required` validation would prevent this state

### Options

| Approach                | Description                                                                     |
| ----------------------- | ------------------------------------------------------------------------------- |
| **Remove entirely**     | Drop `required: bool` from `FieldDescriptor`; all fields optional by definition |
| **Repurpose for CRDT**  | Use to indicate "must sync" vs "optional sync" fields                           |
| **UI hint only**        | Keep as display guidance, but never enforce programmatically                    |
| **Contextual required** | Required only in certain contexts (e.g., export to widget JSON)                 |

### Open Questions

- Is `required` currently enforced anywhere beyond tests?
- Do XLSX import/export workflows depend on required field validation?
- Would removing it break any existing assumptions in `cosam-convert`?
- Should `EntityType::validate()` check required fields, or is that too strict?

### Related

- FEATURE-043 (verify callback) — both touch on validation philosophy
- Entity builder design (future work)
- `EntityType::import()` method (inverse of `export()`) — not yet implemented
