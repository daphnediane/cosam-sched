# Migrate PanelToEventRoom to Specialized Storage

## Summary

Replace GenericEdgeStorage usage for PanelToEventRoom with a specialized PanelToEventRoomStorage implementation similar to other edge types, adding any relationship-specific behaviors if needed.

## Status

Not Started

## Priority

Medium

## Description

PanelToEventRoom currently uses GenericEdgeStorage directly in Schedule. This should be migrated to a dedicated PanelToEventRoomStorage to maintain consistency with the edge system refactoring (REFACTOR-001).

## Remaining Work

### 1. Create PanelToEventRoomStorage

**File: `edge/panel_to_event_room.rs`**

Create a specialized storage struct:

- Determine if any specialized behavior is needed (e.g., caching, indexing)
- If simple: leave as GenericEdgeStorage, but declare a type alias for clarity
- If specialized: add relevant caching/indexing fields
- Implement EdgeStorage trait for PanelToEventRoomEdge

### 2. Update Schedule

**File: `schedule/mod.rs`**

Replace panel_to_event_room field from GenericEdgeStorage to PanelToEventRoomStorage.

### 3. Update Re-exports

**File: `edge/mod.rs`**

Add PanelToEventRoomStorage to re-exports.

## Acceptance Criteria

- PanelToEventRoomStorage implements EdgeStorage trait
- Schedule uses PanelToEventRoomStorage instead of GenericEdgeStorage
- No GenericEdgeStorage usage remains in schedule-data crate
- All existing tests pass
