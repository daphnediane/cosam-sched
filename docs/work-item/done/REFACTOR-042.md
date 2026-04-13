# Add typed `id()` accessor to entity data types

## Summary

Add a method to entity data structs that returns the typed ID directly, avoiding repeated `XId::from_uuid(entity.uuid())` boilerplate.

## Status

Completed

## Priority

Low

## Description

`InternalData::uuid()` returns `NonNilUuid`.  Computed-field closures and
other callers must write `PanelId::from_uuid(entity.uuid())` every time they
need the entity's own typed ID.  This is verbose and bypasses type-safety
unnecessarily.

Preferred solution: add a default method (or a companion trait) so that any
type implementing `InternalData` can call `.typed_id::<T::Id>()` or a concrete
shortcut like `PanelData::panel_id()` returning `PanelId` directly.

Alternatively, add a blanket `id()` method to `InternalData` that returns
`Self::Id` when the associated type is in scope.

## Notes

- Discovered while fixing REFACTOR-041 call sites — computed-field closures
  always need `PanelId::from_uuid(entity.uuid())` before calling entity-type
  methods that now accept typed IDs.
- Low priority; the current idiom is correct, just verbose.
