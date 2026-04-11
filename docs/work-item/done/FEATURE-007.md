# Edge/Relationship System

## Summary

Implement typed edge storage for entity-to-entity relationships.

## Status

Completed

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

### Completed

- All five edge-entity structs defined with `#[derive(EntityFields)]`
- Field-level `#[edge_from(Entity)]` / `#[edge_to(Entity)]` macro attributes
  generate `DirectedEdge` impl and typed named accessor methods on `*Data` structs
- `DirectedEdge` trait in `entity::mod` with `from_id()`, `to_id()`,
  `from_uuid()`, `to_uuid()`, `is_self_loop()` default methods
- Edge endpoint fields are immutable after construction: no builder setters,
  excluded from `apply_to()`
- `UuidPreference::GenerateNew` auto-upgrades to `Edge { from, to }` in
  `build()` for deterministic UUID derivation from endpoints
- All edge types wired into `EntityKind`, `EntityUUID`, `PublicEntityRef`,
  `EntityRef`, and `entity::mod` re-exports

### Remaining

- `EdgeStorage` trait and `GenericEdgeStorage<E>` implementation
  (add/remove/query by endpoint) — deferred to FEATURE-008
- Specialized presenter-to-group storage (group detection, transitive closure,
  relationship cache) — deferred to FEATURE-008

## Acceptance Criteria

- [x] All five edge types defined as first-class entities with UUIDs
- [x] `DirectedEdge` trait with typed `FromId`/`ToId` associated types
- [x] Deterministic V5 UUID generation from endpoint UUIDs
- [x] Edge endpoint fields immutable after construction
- Storage, query, and test coverage deferred to FEATURE-008

## Design Revision

The edge-as-entities approach implemented here was superseded by the virtual
edge design in REFACTOR-036/037/038.  Relationships are now stored as UUID
fields on the owning entity; `DirectedEdge`, `#[edge_from]`/`#[edge_to]`, and
`UuidPreference::Edge` are removed.  The five edge entity files are deleted.
