# BUGFIX-078: callback_field_properties generates Scalar instead of Derived

## Summary

The `callback_field_properties!` macro generates `CrdtFieldType::Scalar` for all fields, but it should generate `Derived` for fields with custom read/write callbacks that project from internal state (like Panel's time_slot projections).

## Status

Done

## Priority

High

## Blocked By (optional)

None

## Description

The `callback_field_properties!` macro is designed for fields with custom read/write callbacks that project from internal state (e.g., Panel's `start_time`, `end_time`, `duration` projecting from `time_slot`). These are derived fields that should not be stored in the CRDT document.

However, the macro currently generates `crdt_type` based on the `FieldTypeMapping` trait, which defaults to `CrdtFieldType::Scalar` for all types. This causes derived fields to be incorrectly marked as Scalar, leading to:

- Incorrect CRDT storage behavior (derived data being stored)
- Potential data inconsistency between in-memory state and CRDT document
- Violation of the single-source-of-truth principle

The macro should generate `CrdtFieldType::Derived` for all callback fields, since they by definition project from other state rather than being the primary storage location.

## Cross-References

- BUGFIX-073: Panel time_slot is silently dropped on save/load - this is the high-priority issue that needs fixing. The callback_field_properties bug is a separate issue that affects the macro implementation, but BUGFIX-073 describes the actual data loss problem.
- REFACTOR-074: Completed - introduced HalfEdgeDescriptor and EdgeKind, which included the macro refactor that removed edge_field_properties

## How Found

Review of open bugfix items BUGFIX-073 and BUGFIX-076 during work item cleanup. BUGFIX-073 described time fields as Derived but the macro was generating Scalar, revealing the root cause in the macro itself.

## Reproduction

1. Define a field using `callback_field_properties!` with custom read/write callbacks
2. Check the generated `crdt_type` in the `CommonFieldData`
3. Observe it is `Scalar` instead of `Derived`

**Expected:** Fields defined with `callback_field_properties!` should have `crdt_type: Derived` since they project from internal state.

**Actual:** Fields have `crdt_type: Scalar` based on the item type's `FieldTypeMapping::CRDT_TYPE`.

## Resolution

Fixed by refactoring `crdt_type` handling in the macros:

- Moved `crdt_type` from `CommonFieldData` to `FieldDescriptor<E>` (field/descriptor.rs)
- Updated `callback_field_properties!` to compute a default `crdt_type` based on field type:
  - List cardinality → `CrdtFieldType::List`
  - Single/Optional cardinality → use the marker trait's `CRDT_TYPE` (e.g., DateTime → Auto, String → Scalar, Text → Text)
- Removed `crdt_type` as an input parameter from `callback_field_properties!` macro
- Updated macro to return a 3-tuple `(data, crdt_type, cb)` instead of `(data, cb)`
- Field authors can override the default by using `let (data, _, cb)` (ignoring the macro's crdt_type) and explicitly setting `crdt_type: CrdtFieldType::Derived` in the FieldDescriptor initialization
- Updated all field definitions across panel.rs, presenter.rs, panel_type.rs, event_room.rs, and hotel_room.rs to use the new pattern
- Fields that are truly derived (e.g., `FIELD_PRESENTERS`, `FIELD_INCLUSIVE_PRESENTERS`, `FIELD_CREDITS`) now explicitly set `crdt_type: CrdtFieldType::Derived`
- Fields that should be stored (e.g., `FIELD_START_TIME`, `FIELD_END_TIME`, `FIELD_DURATION`) use the macro's default

This approach provides sensible defaults based on field type while giving field authors explicit control to override when needed.

## Testing

- Verify that all existing fields using `callback_field_properties!` now have `crdt_type: Derived`
- Add a test that checks the generated `crdt_type` for a callback field
- Ensure existing save/load round-trip tests still pass (they should work correctly with Derived fields not being stored)
- Add a regression test to ensure callback fields are never stored in the CRDT document
