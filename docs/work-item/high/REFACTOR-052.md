# Edge to Edge-Entity Migration

## Summary

Convert edge types from separate edge storage to first-class entities with NonNilUuid, stored in EntityStorage alongside other entities.

## Status

Not Started

## Priority

High

## Description

Migrate all edge types (PanelToPresenter, PanelToEventRoom, EventRoomToHotelRoom, PanelToPanelType) from GenericEdgeStorage to EntityStorage as first-class entities with UUIDs. This enables edge metadata, simplifies transaction handling, and prepares for CRDT integration.

## Implementation Details

### 1. Convert PanelToPresenterEdge to Entity

**File: `edge/panel_to_presenter.rs`**

- Add `#[derive(EntityFields)]` to PanelToPresenter
- Add `uuid: NonNilUuid` field
- Convert to `PanelToPresenterEntity` naming
- Add metadata fields: `is_primary: bool`, `confirmed: bool`
- Implement EntityType trait

### 2. Convert PanelToEventRoomEdge to Entity

**File: `edge/panel_to_event_room.rs`**

- Same pattern as PanelToPresenter
- Add metadata: `is_primary_room: bool` (for panels spanning multiple rooms)

### 3. Convert EventRoomToHotelRoomEdge to Entity

**File: `edge/event_room_to_hotel_room.rs`**

- Same pattern
- Add metadata as needed

### 4. Convert PanelToPanelTypeEdge to Entity

**File: `edge/panel_to_panel_type.rs`**

- Same pattern
- Add metadata as needed

### 5. Migrate Storage

**File: `schedule/storage.rs`**

- Remove GenericEdgeStorage usage for these edge types
- Add TypedStorage implementations for each edge-entity type
- Update Schedule struct to hold EntityStorage<EdgeType>

### 6. Update Queries and Indexes

**Files: Various query modules**

- Convert edge lookups to entity lookups
- Update relationship traversal methods
- Maintain existing query APIs for backward compatibility during migration

## Acceptance Criteria

- [ ] PanelToPresenterEntity implements EntityType with NonNilUuid
- [ ] PanelToEventRoomEntity implements EntityType with NonNilUuid
- [ ] EventRoomToHotelRoomEntity implements EntityType with NonNilUuid
- [ ] PanelToPanelTypeEntity implements EntityType with NonNilUuid
- [ ] All edge-entities stored in EntityStorage
- [ ] Edge metadata fields (is_primary, confirmed) accessible
- [ ] All existing tests pass
- [ ] REFACTOR-029 marked as superseded/complete

## Dependencies

- None (this is phase 1 of entity redesign)

## Notes

This phase supersedes REFACTOR-029 (PanelToEventRoom specialized storage) since we're replacing the entire edge storage approach.

When complete, update REFACTOR-051.md to mark REFACTOR-052 as complete.
