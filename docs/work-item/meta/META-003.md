# Phase 2 — Core Data Model (schedule-core)

## Summary

Phase tracker for the entity/field system and core schedule data model in schedule-core.

## Status

Open

## Priority

High

## Blocked By

- META-002: Phase 1 — Foundation

## Description

Build the `schedule-core` crate containing the complete entity/field system.
Entity `Data` struct declarations are hand-written and visible — macros must not
obscure them. Proc-macros and `macro_rules!` may be used for boilerplate (trait
impls, field accessor singletons, builders). `CrdtFieldType` annotations are
baked in from the start.

## Work Items

- FEATURE-010: FieldValue, error types, CrdtFieldType
- FEATURE-011: Field traits + FieldDescriptor
- FEATURE-012: EntityType, EntityId, EntityKind
- FEATURE-013: FieldSet registry
- FEATURE-014: PanelType entity (proof of concept)
- FEATURE-015: TimeRange + Panel entity
- FEATURE-016: Presenter + EventRoom + HotelRoom entities
- FEATURE-017: Builder pattern
- FEATURE-018: Relationship storage (EdgeMap / reverse indexes)
- FEATURE-019: Schedule container + EntityStorage
- FEATURE-020: Query system
- FEATURE-043: Field verification callbacks (verify_fn)
- FEATURE-046: Bulk field updates (write_multiple)
- FEATURE-021: Edit command system with undo/redo
