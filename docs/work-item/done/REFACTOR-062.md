# REFACTOR-062: Redesign EdgeDescriptor with FieldDescriptorAny and inventory

## Summary

Replace string-based `EdgeDescriptor` fields with `&'static dyn FieldDescriptorAny` references
and move EdgeDescriptor registration to `inventory`.

## Status

Completed

## Priority

High

## Blocked By

- REFACTOR-061: FieldDescriptorAny / FieldId / FieldNodeId foundation

## Description

This is Phase 2 of the FieldNodeId edge system refactor. Changes to `EdgeDescriptor`:

- Remove: `owner_type`, `target_type`, `is_homogeneous`, `field_name`, `fields: &[EdgeFieldSpec]`
- Remove: `EdgeFieldSpec`, `EdgeFieldDefault` types
- Add: `owner_field: &'static dyn FieldDescriptorAny`, `target_field: &'static dyn FieldDescriptorAny`
- Add: `is_transitive: bool` (replaces `is_homogeneous` for HomoEdgeCache purposes)
- CRDT field name derived from `owner_field.name()`; owner/target type from trait methods

Replace `ALL_EDGE_DESCRIPTORS` static slice with inventory:

- `CollectedEdge(&'static EdgeDescriptor)` wrapper
- `inventory::collect!(CollectedEdge)` on each owner entity module
- `all_edge_descriptors()` helper returning `impl Iterator<Item = &'static EdgeDescriptor>`

Update all `const EDGE_*` declarations in `panel.rs`, `presenter.rs`, `event_room.rs`.
Split `EDGE_PRESENTERS` into `EDGE_CREDITED_PRESENTERS` + `EDGE_UNCREDITED_PRESENTERS`
(both with `target_field = &PresenterEntityType::FIELD_PANELS`).
