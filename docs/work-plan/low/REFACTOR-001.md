# Edge Trait System and Relationship Storage

## Summary

Replace current generic Edge entity with a trait-based edge system with dedicated storage per relationship type.

## Status

Not Started

## Priority

High

## Description

The current `Edge` entity is a generic catch-all. Replace it with typed edge structs implementing common edge traits, each with dedicated storage, bidirectional index lookups, and relationship-specific behavior (e.g., transitive closure for presenter groups).

## Implementation Details

- Create edge trait hierarchy: `Edge` base trait, `RelationshipEdge` for presenter-group transitive closure, `SimpleEdge` for basic relationships
- Create `EdgeStorage<T: Edge>` generic storage with bidirectional index lookups
- Implement specific edge structs:
  - `PresenterToGroupEdge` with `always_grouped` / `always_shown_in_group` flags, cycle detection, transitive closure
  - `PanelToPresenterEdge`
  - `PanelToEventRoomEdge`
  - `EventRoomToHotelRoomEdge`
  - `PanelToPanelTypeEdge`
- Create `EdgeRegistry` for type-safe edge management
- Integrate edge storage into `Schedule` struct with lifecycle management
- Add computed entity field access via edges (e.g., `Panel.presenters`, `EventRoom.hotel_room`)
- Port relationship traversal semantics from `schedule-core/src/data/relationship.rs`

## Acceptance Criteria

- All relationship types have dedicated typed edge structs
- Bidirectional lookups work for all edge types
- Presenter group transitive closure matches `schedule-core` behavior
- Computed entity fields resolve via edge lookups
- Old generic `Edge` entity removed
