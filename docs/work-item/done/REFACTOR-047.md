# Update Schedule core: remove allocators, add schedule_id and fetch_uuid

## Summary

Remove `IdAllocators` from `Schedule`, add `schedule_id: Uuid` to `ScheduleMetadata`, add a private entity UUID registry, implement `Schedule::fetch_uuid`, and update all typed entity/edge method signatures to use typed ID wrappers.

## Status

Completed

## Priority

High

## Description

Part of REFACTOR-037. This is the main wiring phase that makes the UUID migration visible in the public `Schedule` API.

Changes to `crates/schedule-data/src/schedule/mod.rs`:

* Remove `IdAllocators` struct and all its uses (UUID generation no longer requires counters)
* Remove `pub type EdgeId = u64` (use `edge::EdgeId` which is unchanged)
* `ScheduleMetadata`: add `pub schedule_id: uuid::Uuid`; generate it in `ScheduleMetadata::new()` via `uuid::Uuid::now_v7()`
* `Schedule`: add private `entity_registry: HashMap<uuid::Uuid, crate::entity::EntityKind>`
* Update `add_entity` to insert `(data.uuid(), EntityKind::Panel)` (etc.) into `entity_registry`
* Implement `pub fn fetch_uuid(&self, uuid: uuid::Uuid) -> Option<crate::entity::PublicEntityRef>`:
  * Match `entity_registry.get(&uuid)` → `EntityKind::Panel` → look up in typed storage → call `data.to_public()` → wrap in `PublicEntityRef::Panel(...)`
  * Repeat for all five entity kinds
* Update all typed accessor/mutator methods to use typed ID parameters:
  * `get_panel_presenters(panel_id: PanelId) -> Vec<PresenterId>`
  * `connect_panel_to_presenter(panel_id: PanelId, presenter_id: PresenterId)`
  * `get_presenter_panels(presenter_id: PresenterId) -> Vec<PanelId>`
  * `connect_panel_to_event_room(panel_id: PanelId, room_id: EventRoomId)`
  * `connect_panel_to_panel_type(panel_id: PanelId, type_id: PanelTypeId)`
  * `connect_event_room_to_hotel_room(event_room_id: EventRoomId, hotel_room_id: HotelRoomId)`
  * (all similar methods throughout)
* `find_related` generic method: return `Vec<uuid::Uuid>` for the untyped path

## Acceptance Criteria

* `Schedule` compiles with no remaining `EntityId`, `InternalId`, or `IdAllocators` references
* `ScheduleMetadata::new()` assigns a new `schedule_id: Uuid`
* `fetch_uuid(uuid)` returns `Some(PublicEntityRef::Panel(panel))` for a known panel UUID
* All typed methods accept/return typed `*Id` wrappers
* `cargo test` passes

## Notes

* `fetch_uuid` dispatches through the private `entity_registry` — O(1) lookup
* REFACTOR-048 adds `type_of_uuid` and `lookup_uuid` using the same registry
* See parent: REFACTOR-037
