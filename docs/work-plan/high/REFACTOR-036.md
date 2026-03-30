# Schedule-Data Rewrite

## Summary

Feature parity for `schedule-data` focused on `schedule-core/src/data` and
`schedule-core/src/edit`.

## Status

🚧 In progress

## Priority

High

## Description

This plan migrates schedule-core data/edit behavior into schedule-data using internal monotonic IDs, spreadsheet-canonical field coverage, soft-delete lifecycle, derived scheduling state, and batched undoable edits delivered phase-by-phase with commit checkpoints.

## Phase Status

- Phase 1: Completed
- Phase 2: In Progress
- Phase 3: Not started
- Phase 4: Not started
- Phase 5: Not started
- Phase 6: Not started

## Scope (Phase 1)

Unify `schedule-data` entity identity around a single internal monotonic `u64` ID model while preserving external soft indices for lookup and I/O.

- Remove per-entity `EntityType::Id` usage from core schedule/query/storage APIs.
- Normalize lifecycle state to soft-delete only (`active` / `inactive`).
- Add schedule-level monotonic ID allocators.
- Store entities canonically by internal ID with an external-string index map.
- Keep edge IDs monotonic via schedule allocator.

## Implemented

- `EntityType` now uses unified internal identity semantics (`EntityId = u64`) and optional `external_id()` hook.
- `EntityState::Deleted` removed; lifecycle now `Active`/`Inactive`.
- `Schedule` now owns `IdAllocators` and allocates entity/edge IDs centrally.
- `Schedule` add/find/update signatures switched to internal IDs.
- `EntityStorage` refactored to internal-ID canonical map + external string index map.
- Query `Finder`/`Updater` IDs switched to internal `u64`.
- Plan updated to include future `FieldSet` index scoring (`NotMatch`/`WeakMatch`/`StrongMatch`/`ExactMatch`).

## Notes

- The crate currently has substantial pre-existing compile failures from mixed field APIs (`FieldDescriptor` vs `NamedField`/`FieldSet`).
- This phase intentionally focuses on ID/state foundations first; additional compatibility and field-system cleanup continues in subsequent work.
