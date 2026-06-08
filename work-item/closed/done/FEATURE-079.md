# FEATURE-079: UUID conflict detection with expanded UuidPreference variants

## Summary

Add UUID conflict detection to entity creation and expand UuidPreference with "prefer" variants that allow fallback to alternate UUIDs.

## Status

Completed

## Priority

High

## Description

Currently, `UuidPreference::Exact` and `UuidPreference::FromV5` silently replace existing entities if the UUID is already in use. This is unsafe and can lead to data loss. We need to:

1. Detect UUID conflicts before insertion
2. Expand UuidPreference with "prefer" variants that allow graceful fallback
3. Make "exact" variants error on conflict

### New UuidPreference variants

- **GenerateNew** (existing) - Generate a new v7 UUID
- **ExactFromV5 { name }** (new) - Derive deterministic v5 UUID from natural key; error if UUID already exists
- **PreferFromV5 { name }** (new) - Derive deterministic v5 UUID from natural key; if conflict, fall back to GenerateNew
- **Exact(uuid)** (existing, changed behavior) - Use exact UUID; error if already exists (currently replaces)
- **Prefer(uuid)** (new) - Prefer exact UUID; if conflict, fall back to GenerateNew

### Implementation Details

1. Add `contains_entity<E>(id: EntityId<E>) -> bool` method to `Schedule`
2. Add `is_entity_deleted<E>(id: EntityId<E>) -> bool` method to `Schedule` for tombstone support
3. Add `UuidConflict(NonNilUuid, &'static str)` variant to `BuildError`
4. Update `build_entity` to check for conflicts before insertion:
   - For `Exact` and `ExactFromV5`: return `BuildError::UuidConflict` if exists and NOT tombstoned (tombstoned entities can be recreated)
   - For `Prefer` and `PreferFromV5`: if conflict and NOT tombstoned, fall back to `GenerateNew` (tombstoned entities use preferred UUID)
5. Update `EntityId::from_preference` to handle the new variants
6. Update CRDT rehydration to use `Exact` (conflict should indicate data corruption)
7. Add comprehensive tests for conflict scenarios
8. Add tests for recreating tombstoned entities

## Acceptance Criteria

- [x] Work item created
- [x] `UuidPreference` enum expanded with 5 variants
- [x] `Schedule::contains_entity` implemented
- [x] `Schedule::is_entity_deleted` implemented for tombstone support
- [x] `BuildError::UuidConflict` variant added
- [x] `build_entity` checks conflicts and handles fallback with tombstone support
- [x] Tests cover all conflict scenarios including tombstone recreation
- [x] All existing tests pass
- [x] Documentation updated

## Notes

- CRDT rehydration should use `Exact` - a conflict indicates data corruption and should error
- Import workflows can use `PreferFromV5` for graceful handling of duplicate natural keys
- Builder macro requires no changes - errors propagate automatically via `BuildError`
- Tombstoned (soft-deleted) entities can be recreated with `Exact` or `ExactFromV5` - this is intentional for undo/redo support
- The `is_entity_deleted` method was added to check if an entity is tombstoned
