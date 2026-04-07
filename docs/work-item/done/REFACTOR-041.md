# Add typed UUID ID wrappers for all entity types

## Summary

Introduce per-entity typed ID newtypes (`PanelId`, `PresenterId`, `EventRoomId`, `HotelRoomId`, `PanelTypeId`) each wrapping `uuid::Uuid`, replacing bare `u64` typed IDs.

## Status

Completed

## Priority

High

## Description

Part of REFACTOR-037. Typed ID wrappers provide compile-time safety: calling `get_panel_presenters(presenter_id)` becomes a compile error since `PresenterId` is not `PanelId`. Each wrapper is a simple newtype with standard trait impls.

Files and changes:

* `entity/panel.rs` — change `PanelId(u64)` → `PanelId(Uuid)`; update `Display`; add `From<Uuid>`, `Into<Uuid>` impls; update `Panel.presenters: Vec<EntityId>` → `Vec<PresenterId>`
* `entity/presenter.rs` — add `PresenterId(Uuid)` with `Display`, `From<Uuid>`, `Into<Uuid>`
* `entity/event_room.rs` — add `EventRoomId(Uuid)` with same impls
* `entity/hotel_room.rs` — add `HotelRoomId(Uuid)` with same impls
* `entity/panel_type.rs` — add `PanelTypeId(Uuid)` with same impls

Each typed ID derives `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord` and implements `Serialize`/`Deserialize` (transparent UUID string format).

Also update the computed field `#[read]` closures in `panel.rs` that reference `entity.entity_id` to use `entity.entity_uuid`, and update presenter list references from `EntityId` to `PresenterId`.

## Acceptance Criteria

* All five entity files have typed `*Id(Uuid)` newtypes
* Each wrapper implements `From<Uuid>`, `Into<Uuid>`, `Display`, `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`, `Serialize`, `Deserialize`
* `Panel.presenters` field type updated to `Vec<PresenterId>`
* Computed field closures in `panel.rs` reference `entity.entity_uuid` correctly

## Notes

* `presenter_rank.rs` does not need an entity ID wrapper — it is a data value, not an entity
* See parent: REFACTOR-037
