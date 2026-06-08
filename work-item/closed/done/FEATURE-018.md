# Relationship Storage (EdgeMap / Reverse Indexes)

## Summary

Implement typed relationship storage for entity-to-entity relationships.

## Status

Completed

## Priority

High

## Blocked By

- FEATURE-016: Presenter + EventRoom + HotelRoom entities

## Description

Relationships between entities are stored as typed `Vec<EntityId>` fields on the
owning entity (virtual edges, not edge entities). Reverse indexes maintain
bidirectional lookup.

### Relationship types

- **Panel → Presenter**: which presenters are on which panels
- **Presenter → Group**: presenter group membership
- **Panel → PanelType**: which category a panel belongs to
- **Panel → EventRoom**: which room a panel is assigned to
- **EventRoom → HotelRoom**: physical room mapping

### EdgeMap

`EdgeMap<L, R>` — bidirectional index mapping `EntityId<L> → Vec<EntityId<R>>`
and the reverse. The left entity owns the forward edge (stored in its data struct);
the reverse index is maintained automatically.

### Lifecycle hooks

Entity types implement `on_insert` and `on_soft_delete` to seed/clear EdgeMap
entries when entities are added/removed.

### Computed field descriptors

Relationship-backed computed fields (e.g., `Panel::presenters`) use
`ComputedFieldDescriptor` with fn pointers that take `(&Data, &EntityStorage)`
for reads and `(&mut Data, FieldValue, &mut EntityStorage)` for writes.

## Acceptance Criteria

- EdgeMap correctly maintains forward and reverse indexes
- Lifecycle hooks seed/clear edges on entity add/remove
- Computed field read/write for relationships works correctly
- Unit tests for CRUD on relationships
