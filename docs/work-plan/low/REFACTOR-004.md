# Mutation API and Side-Effect Semantics

## Summary

Implement per-entity mutation families with deterministic side effects for add, update, restore, and find-or-add operations.

## Status

Not Started

## Priority

High

## Description

Port mutation behavior from `schedule-core/src/edit/command.rs` into `schedule-data`, providing typed mutation families that handle side effects (e.g., creating dependent presenters/rooms/edges from panel edits) deterministically.

## Implementation Details

- Implement per-entity mutation families:
  - `add_<thing>` — strict create with ID allocation
  - `restore_<thing>` — soft-undelete (inactive → active)
  - `find_or_add_<thing>` — find existing / restore inactive / add new
  - `update_<thing>` — upsert + patch field updates
- Support collection-style virtual update fields (`add_presenters`, `remove_presenters`, `add_rooms`, `remove_rooms`) via edge operations
- Port `update_or_create_presenter`-style parsing and relationship behavior from schedule-core
- Ensure side effects (creating dependent entities/edges from panel edits) are deterministic and documented
- Validation hooks: run entity `validate()` before committing mutations

## Acceptance Criteria

- All entity types have complete mutation families on `Schedule`
- Side effects are deterministic and match schedule-core behavior
- `find_or_add` correctly searches by indexed fields, restores inactive matches, or creates new
- Collection-style update fields operate on edges correctly
- Mutation contract documentation complete with side-effect matrix
