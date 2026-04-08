# Add type_of_uuid, lookup_uuid, and EntityRef to Schedule

## Summary

Expose `Schedule::type_of_uuid` and `Schedule::lookup_uuid` using the private entity registry added in REFACTOR-047, and add `EntityRef<'a>` as the borrowed-data return type for `lookup_uuid`.

## Status

Completed

## Priority

High

## Description

Part of REFACTOR-037. REFACTOR-047 added a private `entity_registry: HashMap<Uuid, EntityKind>` and `fetch_uuid` (which returns owned public data). This phase adds the remaining two registry methods for internal use:

* `pub fn type_of_uuid(&self, uuid: uuid::Uuid) -> Option<crate::entity::EntityKind>`
  * Simple registry dispatch — returns only the type tag
  * Useful for callers that already know how to use typed methods once they know the type

* `pub fn lookup_uuid(&self, uuid: uuid::Uuid) -> Option<crate::entity::EntityRef<'_>>`
  * Returns borrowed internal `*Data` via `EntityRef<'a>`
  * Useful for internal code that needs to inspect raw entity data without copying

New type `EntityRef<'a>` added to `entity/mod.rs`:

```rust
pub enum EntityRef<'a> {
    Panel(&'a PanelData),
    Presenter(&'a PresenterData),
    EventRoom(&'a EventRoomData),
    HotelRoom(&'a HotelRoomData),
    PanelType(&'a PanelTypeData),
}
```

`lookup_uuid` dispatches through the registry (same as `fetch_uuid`) but borrows the `*Data` struct rather than cloning it.

## Acceptance Criteria

* `EntityRef<'a>` enum defined in `entity/mod.rs` and exported
* `Schedule::type_of_uuid(uuid: Uuid) -> Option<EntityKind>` works for all five entity types
* `Schedule::lookup_uuid(uuid: Uuid) -> Option<EntityRef<'_>>` returns borrowed data
* Both return `None` for unknown UUIDs
* `cargo test` passes

## Notes

* The registry was established in REFACTOR-047; this phase only adds the two public methods and the `EntityRef` type
* `fetch_uuid` (REFACTOR-047) returns owned data; `lookup_uuid` returns borrowed — both use the same registry
* See parent: REFACTOR-037
