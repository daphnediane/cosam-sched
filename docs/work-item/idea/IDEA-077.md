# IDEA-077: Consider list cardinality support for accessor_field_properties

## Summary

Evaluate whether accessor_field_properties should support add/remove operations for list cardinality fields, and document the work required to implement it.

## Status

Open

## Priority

Low

## Description

The accessor_field_properties macro currently sets add_fn and remove_fn to None for all fields, with a TODO comment to revisit if list cardinality support is implemented. This idea explores whether accessor fields (computed fields that read/write to underlying storage) should support add/remove operations for list fields.

Currently, add/remove operations are only supported for edge fields through the AddEdge/RemoveEdge variants. Supporting add/remove for accessor list fields would require:

1. **Determine use cases**: Identify which accessor fields with list cardinality should support add/remove operations (e.g., adding to a list field vs. replacing the entire list)

2. **Add new AddFn/RemoveFn variants**: Create new callback variants for accessor field add/remove operations, possibly:
   - AddFn::BareList - for bare function add operations on lists
   - AddFn::ScheduleList - for schedule-aware add operations on lists
   - RemoveFn::BareList - for bare function remove operations on lists
   - RemoveFn::ScheduleList - for schedule-aware remove operations on lists

3. **Implement AddableField/RemovableField for FieldDescriptor**: The FieldDescriptor already implements these traits, but they would need to handle the new list-specific variants

4. **Update conversion support**: The conversion layer (field_value_to_runtime_entity_ids and similar functions) may need updates to handle list add/remove operations for non-edge types. Currently these conversions are primarily designed for entity IDs in edge contexts.

5. **Update accessor_field_properties macro**: Add logic to generate appropriate add_fn/remove_fn based on:
   - Field cardinality (Single vs. List)
   - Whether add/remove operations are desired for the field
   - The type of callback needed (bare vs. schedule)

6. **Update stored_output.rs**: Modify the macro to conditionally generate add_fn/remove_fn instead of always setting them to None

7. **Testing**: Add comprehensive tests for add/remove operations on accessor list fields

## Alternatives Considered

- **Keep current behavior**: Continue to require full list replacement via write operations. This is simpler and may be sufficient for most use cases.
- **Only support for specific field types**: Limit add/remove support to specific field types (e.g., only for certain entity types or field patterns)
- **Use edge fields instead**: For relationships that need add/remove semantics, use edge fields rather than accessor list fields

## Open Questions

- Are there actual use cases where add/remove on accessor list fields is needed, or is full list replacement sufficient?
- Should add/remove support be opt-in or opt-out for accessor fields?
- What should the behavior be when adding/removing from a single cardinality field (should it error or be a no-op)?
- Should the new list-specific AddFn/RemoveFn variants share the same enum as the edge variants, or be separate?
