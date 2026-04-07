# Update edge implementation files to typed UUID IDs

## Summary

Update all five concrete edge implementation files to use typed `*Id(Uuid)` constructors and implement `from_uuid()`/`to_uuid()` from the `Edge` trait.

## Status

Open

## Priority

High

## Description

Part of REFACTOR-037. After the `Edge` trait is updated (REFACTOR-042) and typed ID wrappers exist (REFACTOR-041), the five concrete edge files need their constructors and `Edge` impl updated.

Files to update:

* `edge/panel_to_presenter.rs`
  * `new(panel_id: EntityId, presenter_id: EntityId)` → `new(panel_id: PanelId, presenter_id: PresenterId)`
  * `from_id: InternalId` → `from_id: PanelId`, `to_id: InternalId` → `to_id: PresenterId`
  * `impl Edge`: `from_uuid() -> Option<Uuid> { Some(self.from_id.0) }`, `to_uuid()` same pattern

* `edge/panel_to_panel_type.rs`
  * `new(panel_id: EntityId, panel_type_id: EntityId)` → `new(panel_id: PanelId, panel_type_id: PanelTypeId)`
  * Same `from_uuid`/`to_uuid` pattern

* `edge/panel_to_event_room.rs`
  * `new(panel_id: PanelId, room_id: EventRoomId)`
  * Same pattern

* `edge/event_room_to_hotel_room.rs`
  * `new(event_room_id: EventRoomId, hotel_room_id: HotelRoomId)`
  * Same pattern

* `edge/presenter_to_group.rs`
  * `PresenterToGroupEdge` enum variants use `PresenterId` instead of `InternalId`/`EntityId`
  * `PresenterToGroupStorage` inner `HashMap<EntityId, Vec<EntityId>>` maps → `HashMap<Uuid, Vec<Uuid>>`
  * `member_to_groups`, `group_to_members`, `groups`, `always_grouped` caches all use `Uuid` keys/values
  * `RelationshipStorage` impl: `get_inclusive_members(group_id: Uuid)`, `get_inclusive_groups(member_id: Uuid)`, `is_group(uuid: Uuid)`
  * Construction and lookup methods updated throughout

## Acceptance Criteria

* All five edge files compile using typed ID wrappers
* `from_uuid()` and `to_uuid()` implemented on each edge type
* `PresenterToGroupStorage` uses `Uuid` keys in all maps
* `EdgeId(u64)` is unchanged throughout

## Notes

* `PresenterToGroupStorage` is the largest and most complex file (~28 KB); allow extra time
* `BTreeSet<PresenterToGroupEdge>` still works since `Uuid: Ord` and typed wrappers can derive `Ord`
* See parent: REFACTOR-037
