# Rename Edge trait methods from_id/to_id to from_uuid/to_uuid

## Summary

Rename `Edge::from_id()` and `Edge::to_id()` to `from_uuid()` and `to_uuid()` returning `Option<uuid::Uuid>`; update `RelationshipStorage` and `RelationshipEdge` trait signatures to use `Uuid`.

## Status

Open

## Priority

High

## Description

Part of REFACTOR-037. The `Edge` trait in `edge/traits.rs` currently returns `Option<InternalId>` from `from_id()` and `to_id()`. Since `InternalId` is removed in REFACTOR-039, these methods must change to return `Option<uuid::Uuid>` — the raw UUID of the referenced entity.

Note: `EdgeId(u64)` is **not** changed in this phase (that is REFACTOR-038).

Changes to `crates/schedule-data/src/edge/traits.rs`:

* `fn from_id(&self) -> Option<InternalId>` → `fn from_uuid(&self) -> Option<uuid::Uuid>`
* `fn to_id(&self) -> Option<InternalId>` → `fn to_uuid(&self) -> Option<uuid::Uuid>`
* `RelationshipStorage` trait methods: replace `EntityId` parameter type with `uuid::Uuid`
* `RelationshipEdge` trait methods: replace `EntityId` parameter/return types with `uuid::Uuid`
* Remove `use crate::entity::{EntityId, EntityType, InternalId}` import; add `use uuid::Uuid`

The `EdgeStorage` trait's `find_outgoing` and `find_incoming` signatures also take `InternalId` — replace with `Uuid`.

## Acceptance Criteria

* `Edge::from_uuid()` and `Edge::to_uuid()` exist and return `Option<Uuid>`
* `from_id()` and `to_id()` are gone from the trait
* `RelationshipStorage` uses `Uuid` throughout
* `EdgeId(u64)` unchanged

## Notes

* Concrete edge implementations are updated in REFACTOR-045
* `GenericEdgeStorage` index key type updated in REFACTOR-044
* See parent: REFACTOR-037
