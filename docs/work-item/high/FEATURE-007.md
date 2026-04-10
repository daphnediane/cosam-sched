# Edge/Relationship System

## Summary

Implement typed edge storage for entity-to-entity relationships.

## Status

Open

## Priority

High

## Description

Relationships between entities are modeled as typed edges with their own storage
and query capabilities. Edge types include:

- **PanelToPresenter** — which presenters are on which panels
- **PresenterToGroup** — presenter group membership (with `always_grouped` and
  `always_shown_in_group` flags)
- **PanelToEventRoom** — which room a panel is assigned to
- **PanelToPanelType** — which category a panel belongs to
- **EventRoomToHotelRoom** — physical room mapping

### Edge Traits

- `Edge` trait with `from_uuid()`, `to_uuid()`, and edge-specific data
- `EdgeStorage` trait for collections of edges with `add_edge`, `remove_edge`,
  `find_outgoing`, `find_incoming`
- `GenericEdgeStorage<E>` for simple edge types
- Specialized storage for presenter-to-group (group detection, transitive closure)

### Edge-Entity Migration

Edges are also stored as entities in `EntityStorage` (with `EntityKind` variants)
to enable UUID-based lookup and CRDT tracking. V5 UUIDs provide deterministic
identity based on endpoint UUIDs.

### Relationship Cache

Cached transitive closure for inclusive presenter/panel lookups and group
membership traversal.

## Acceptance Criteria

- All five edge types have storage implementations
- Edges can be added, removed, and queried by either endpoint
- Presenter group detection and transitive closure work correctly
- V5 UUID generation is deterministic for same endpoints
- Unit tests for CRUD and transitive queries
