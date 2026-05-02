# BUGFIX-076: Implement multi-source detection for edge_field_properties add_fn

## Summary

The edge_field_properties macro currently sets add_fn to AddEdge for all target edges without checking if the edge has multiple source fields. This should return None for target edges with multiple sources since add_edge doesn't support multi-source edges yet.

## Status

Open

## Priority

Low

## Blocked By (optional)

None

## Description

In the edge_field_properties macro (crates/schedule-macro/src/edge_output.rs), the add_fn generation logic currently returns AddEdge for all target edges regardless of the number of source fields. However, the add_edge function in schedule-core only supports single-source target edges (it returns an error for multiple sources). This inconsistency means the macro-generated code claims to support add operations that will fail at runtime.

## How Found

Code review during edge descriptor refactoring. The TODO comment at line 75 in edge_output.rs explicitly notes this limitation.

## Reproduction

N/A - This is a code generation issue, not a runtime bug.

**Expected:** The macro should check the source_fields array length and return None for add_fn when there are multiple sources, matching the runtime behavior of add_edge.

**Actual:** The macro returns AddEdge for all target edges, even those with multiple sources, leading to runtime errors when add is called.

## Steps to Fix

1. Parse the source_fields array in the edge_field_properties macro to determine its length
2. Update the add_fn generation logic to:
   - Return AddEdge if source_fields has exactly one element
   - Return None if source_fields has multiple elements
3. Remove or update the TODO comment

## Testing

- Verify that target edges with single source fields still get AddFn::AddEdge
- Verify that target edges with multiple source fields get add_fn: None
- Ensure existing tests still pass
- Add a test case for a multi-source target edge to verify add_fn is None
