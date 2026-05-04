# BUGFIX-078: callback_field_properties generates Scalar instead of Derived

## Summary

The `callback_field_properties!` macro generates `CrdtFieldType::Scalar` for all fields, but it should generate `Derived` for fields with custom read/write callbacks that project from internal state (like Panel's time_slot projections).

## Status

Open

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

## Steps to Fix

1. Modify `callback_output.rs` to unconditionally generate `CrdtFieldType::Derived` instead of using the marker trait's `CRDT_TYPE`
2. Remove the `crdt_type` generation logic that depends on cardinality and item type
3. Update the macro documentation to clarify that callback fields are always Derived
4. Consider adding a separate macro for Scalar fields with custom callbacks if needed

## Testing

- Verify that all existing fields using `callback_field_properties!` now have `crdt_type: Derived`
- Add a test that checks the generated `crdt_type` for a callback field
- Ensure existing save/load round-trip tests still pass (they should work correctly with Derived fields not being stored)
- Add a regression test to ensure callback fields are never stored in the CRDT document
