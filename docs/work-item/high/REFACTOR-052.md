# Edge to Edge-Entity Migration

## Summary

Convert edge types from separate edge storage to first-class entities with NonNilUuid, stored in EntityStorage alongside other entities.

## Status

In Progress - Query Layer Complete

## Completed

- [x] Entity module files created for all 5 edge types
- [x] EntityKind variants added for all edge types
- [x] EntityUUID variants added for all edge types
- [x] TypedStorage implementations for all edge types
- [x] EntityStorage fields for all edge types
- [x] Schedule.identify() handles all edge types
- [x] Correct metadata preserved (`always_shown_in_group`, `always_grouped` for PresenterToGroup)
- [x] Removed metadata documented in FEATURE-030
- [x] V5 UUID generation for deterministic edge identities (`uuid_v5` module)
- [x] `EdgeEntityQuery` module with secondary indexes for endpoint-based lookups
- [x] `PanelToPresenterIndex` for panel/presenter queries
- [x] `PresenterToGroupIndex` for member/group queries with group detection
- [x] Transitive closure caching for inclusive presenter/panel lookups
- [x] Schedule CRUD: `create_panel_to_presenter_entity`, `create_presenter_to_group_entity`
- [x] Schedule CRUD: `delete_panel_to_presenter_entity`, `delete_presenter_to_group_entity`
- [x] Schedule CRUD: `delete_panel_to_presenter_by_endpoints`, `delete_presenter_to_group_by_endpoints`
- [x] Tests for all new edge-entity CRUD operations

## Priority

High

## Description

Migrate all edge types (PanelToPresenter, PanelToEventRoom, EventRoomToHotelRoom, PanelToPanelType, PresenterToGroup) from GenericEdgeStorage to EntityStorage as first-class entities with UUIDs. This enables edge metadata, simplifies transaction handling, and prepares for CRDT integration.

Note: PresenterToGroup is self-referential (connects Presenter to Presenter) with special semantics for group markers and membership.

## Implementation Details

### Edge UUID Generation Strategy

Edge UUIDs should be **V5 (deterministic)** generated from:

- Namespace: Fixed edge-type namespace (e.g., UUID for "panel-to-presenter")
- Name: Hash of `(from_uuid, to_uuid)` concatenation

This provides:

- **Idempotent edge creation**: Same edge always gets same UUID
- **Natural collision detection**: V5 collision = duplicate edge attempt
- **Transaction compatibility**: Edge identity is immutable function of endpoints

Alternative considered: V7 random UUIDs would require separate uniqueness checks for (from, to) pairs.

### 1. Convert PanelToPresenterEdge to Entity

**File: `edge/panel_to_presenter.rs`**

- Add `#[derive(EntityFields)]` to PanelToPresenter
- Add `uuid: NonNilUuid` field
- Convert to `PanelToPresenterEntity` naming
- Add metadata fields: `is_primary: bool`, `confirmed: bool`
  - These should be removed and documented as something to consider for FEATURE-030
- Implement EntityType trait

### 2. Convert PanelToEventRoomEdge to Entity

**File: `edge/panel_to_event_room.rs`**

- Same pattern as PanelToPresenter
- Add metadata: `is_primary_room: bool` (for panels spanning multiple rooms)
  - This should be removed and documented as something to consider for FEATURE-030
  - Also need to consider how this interacts with HotelRoom and if it belongs there FEATURE-030

### 3. Convert EventRoomToHotelRoomEdge to Entity

**File: `edge/event_room_to_hotel_room.rs`**

- Same pattern
- Add metadata as needed

### 4. Convert PanelToPanelTypeEdge to Entity

**File: `edge/panel_to_panel_type.rs`**

- Same pattern
- Add metadata as needed

### 5. Convert PresenterToGroupEdge to Entity

**File: `edge/presenter_to_group.rs`**

- Self-referential edge (Presenter→Presenter)
- Preserve `is_group_marker` and `is_group_member` flags
  - These new flags should not have been added and need to be removed
- Preserve `always_shown_in_group` and `always_grouped` flags
- Migrate RelationshipCache logic to work with edge-entity storage
- Group marker edges: member=presenter, group=presenter (self-loop)
- Group member edges: member=presenter, group=group_presenter

### 6. Migrate Storage

**File: `schedule/storage.rs`**

- Remove GenericEdgeStorage usage for these edge types
- Add TypedStorage implementations for each edge-entity type
- Update Schedule struct to hold `EntityStorage<EdgeType>`

### 6. Update Queries and Indexes

**Files:** Various query modules

- Convert edge lookups to entity lookups
- Update relationship traversal methods
- Maintain existing query APIs for backward compatibility during migration

## Acceptance Criteria

- [ ] PanelToPresenterEntity implements EntityType with NonNilUuid
- [ ] PanelToEventRoomEntity implements EntityType with NonNilUuid
- [ ] EventRoomToHotelRoomEntity implements EntityType with NonNilUuid
- [ ] PanelToPanelTypeEntity implements EntityType with NonNilUuid
- [ ] PresenterToGroupEntity implements EntityType with NonNilUuid (self-referential)
- [ ] All edge-entities stored in EntityStorage
- [ ] Edge metadata that we had before is still present (always_shown_in_group, always_grouped)
- [ ] Edge metadata fields (always_shown_in_group, always_grouped) accessible
- [ ] Document new metadata in FEATURE-030 for future consideration
- [ ] All existing tests pass
- [ ] REFACTOR-029 marked as superseded/complete

## Dependencies

- None (this is phase 1 of entity redesign)

## Notes

This phase supersedes REFACTOR-029 (PanelToEventRoom specialized storage) since we're replacing the entire edge storage approach.

When complete, update REFACTOR-051.md to mark REFACTOR-052 as complete.
