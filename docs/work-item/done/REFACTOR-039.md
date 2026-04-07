# Replace EntityId and InternalId with Uuid in core entity types

## Summary

Remove `EntityId = u64` type alias and `InternalId` struct from `entity/mod.rs`; add `EntityKind` and `PublicEntityRef` enums; re-export `uuid::Uuid`.

## Status

Completed

## Priority

High

## Description

Part of REFACTOR-037. This phase updates the foundation types that the rest of the UUID migration depends on.

Changes to `crates/schedule-data/src/entity/mod.rs`:

* Remove `pub type EntityId = u64`
* Remove `InternalId` struct (no longer needed once typed wrappers exist)
* Update `InternalData` trait: `entity_id() -> EntityId` → `uuid() -> uuid::Uuid` and `set_entity_id` → `set_uuid`
* Add `pub use uuid::Uuid` re-export for crate-wide convenience
* Add `pub enum EntityKind { Panel, Presenter, EventRoom, HotelRoom, PanelType }` for registry dispatch
* Add `pub enum PublicEntityRef { Panel(Panel), Presenter(Presenter), EventRoom(EventRoom), HotelRoom(HotelRoom), PanelType(PanelType) }` as the return type for `Schedule::fetch_uuid`

`EntityKind` and `PublicEntityRef` require the concrete entity structs to be in scope, so the enums are defined at the bottom of `entity/mod.rs` after all the `pub use` re-exports.

## Acceptance Criteria

* `EntityId` type alias gone from codebase
* `InternalId` struct gone from codebase
* `InternalData` trait methods use `Uuid`
* `EntityKind` and `PublicEntityRef` enums compile without errors
* `cargo test` passes (downstream compile errors from dependent code are expected and resolved in later phases)

## Notes

* This phase intentionally causes downstream compile errors in edge and schedule code; those are resolved in REFACTOR-041 through REFACTOR-047
* See parent: REFACTOR-037
