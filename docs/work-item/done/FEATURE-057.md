# FEATURE-057: Inclusive Edge Cache

## Summary

Implement a transitive edge relationship cache to efficiently compute inclusive members, groups, panels, and other hierarchical relationships.

## Status

Completed

## Priority

Medium

## Blocked By (optional)

None

## Description

Currently, computing transitive edge relationships (e.g., all members of a group including nested groups, all panels a presenter belongs to including via group membership) requires walking the edge graph each time. This is inefficient for repeated queries.

This feature implements a cache similar to the v10-try1 relationship.rs implementation that stores both direct and transitive (inclusive) edge relationships. The cache invalidates automatically whenever the edge map is updated, ensuring correctness while providing O(1) lookup for common queries.

## Implementation Details

1. Create a new `edge_cache.rs` module in `schedule-core/src/`
2. Implement `EdgeCache` struct with:
   - Direct edge maps (forward and reverse for each entity type pair)
   - Inclusive (transitive) edge maps
   - Cache version counter for invalidation
3. Add `EdgeCache` field to `Schedule` struct
4. Invalidate cache in all edge modification methods:
   - `edge_add`
   - `edge_remove`
   - `edge_set`
   - `remove_entity` (via `clear_all`)
5. Implement lazy cache rebuilding (only rebuild when queried after invalidation)
6. Add public methods to `Schedule` for querying inclusive relationships:
   - `inclusive_edges_from<L, R>(id)` - all R entities reachable from L via transitive edges
   - `inclusive_edges_to<L, R>(id)` - all L entities that transitively point to R
7. Add comprehensive tests for cache invalidation and transitive closure computation

## Acceptance Criteria

- Cache stores both direct and transitive edge relationships
- Cache invalidates on any edge modification
- Lazy rebuilding ensures cache is only rebuilt when needed
- Public API provides O(1) lookup for inclusive relationships
- Tests cover:
  - Basic transitive closure (A→B→C means A→C)
  - Cycle handling (A→B, B→A should not infinite loop)
  - Cache invalidation on edge add/remove
  - Mixed het and homo edges
  - Multiple entity types

## Notes

Reference implementation: v10-try1/crates/schedule-core/src/data/relationship.rs

The main workspace uses a different edge storage model (RawEdgeMap with UUIDs and type tags) compared to v10-try1 (string-based names), so the cache must be adapted to work with the current architecture.

## Followup

The implementation now uses a single `inclusive_edges<Near, Far>(near: EntityId<Near>, edge: FullEdge)` method instead of the originally planned separate `inclusive_edges_from` and `inclusive_edges_to` methods. The `FullEdge` parameter encodes both the near and far field descriptors and traversal direction, making the API more explicit and avoiding near/far confusion for homogeneous edges.
