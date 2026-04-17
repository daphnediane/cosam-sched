# META-048: FieldValue / FieldType / Converter Overhaul

## Summary

Restructure `FieldValue` with proper cardinality, add `FieldTypeItem`/`FieldType`
enums, wire `FieldType` into `FieldDescriptor`, and implement the generic
`FieldValueConverter` system from IDEA-038.

## Status

Open

## Priority

High

## Description

The current `FieldValue` enum conflates scalar values, lists, and absence into a
single flat enum. This overhaul splits it into `FieldValueItem` (scalars) and
`FieldValue` (`Single`/`Optional`/`List` wrappers), adds a matching `FieldTypeItem` /
`FieldType` pair for type-level declarations, wires `FieldType` into field descriptors,
and finally adds the type-safe `FieldValueConverter` system for import pipelines.

The `EntityIdentifier` ad-hoc enum is also removed; entity references are unified
under `FieldValueItem::EntityId(RuntimeEntityId)`.

## Work Items

- REFACTOR-049: Restructure FieldValue → FieldValueItem + cardinality
- FEATURE-050: Add FieldTypeItem and FieldType enums
- FEATURE-051: Add field\_type to FieldDescriptor
- FEATURE-038: FieldValueConverter system

## Notes

IDEA-038 has been promoted to FEATURE-038 for the converter phase.
IDEA-037 (read-only entity resolution) is captured by the `lookup_next` /
`resolve_next` split in FEATURE-038.
