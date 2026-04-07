# Update GenericEdgeStorage to index by Uuid

## Summary

Replace `HashMap<EntityId, Vec<EdgeId>>` outgoing/incoming indexes in `GenericEdgeStorage` with `HashMap<uuid::Uuid, Vec<EdgeId>>`.

## Status

Completed

## Priority

High

## Description

Part of REFACTOR-037. `GenericEdgeStorage<E>` in `edge/generic.rs` maintains two index maps keyed by `EntityId` (u64) for fast edge lookup by entity. After the entity ID migration, these keys become `uuid::Uuid`.

Changes to `crates/schedule-data/src/edge/generic.rs`:

* `outgoing_index: HashMap<EntityId, Vec<EdgeId>>` → `HashMap<uuid::Uuid, Vec<EdgeId>>`
* `incoming_index: HashMap<EntityId, Vec<EdgeId>>` → `HashMap<uuid::Uuid, Vec<EdgeId>>`
* Index population: currently calls `edge.from_id().entity_id` → call `edge.from_uuid()` (returning `Option<Uuid>`)
* `find_outgoing(&self, from_id: InternalId)` → `find_outgoing(&self, from_uuid: uuid::Uuid)`
* `find_incoming(&self, to_id: InternalId)` → `find_incoming(&self, to_uuid: uuid::Uuid)`
* Remove any remaining `next_id: u64` counter if present (EdgeId allocation happens at edge construction, not in storage)
* Remove import of `crate::entity::{EntityId, InternalId}`; add `use uuid::Uuid`

Note: `EdgeId(u64)` itself is **unchanged** in this phase.

## Acceptance Criteria

* `GenericEdgeStorage` compiles with `HashMap<Uuid, Vec<EdgeId>>` indexes
* `find_outgoing(uuid: Uuid)` and `find_incoming(uuid: Uuid)` work correctly
* No remaining `EntityId` or `InternalId` references in `generic.rs`

## Notes

* Concrete edge types updated in REFACTOR-045 to implement `from_uuid()`/`to_uuid()`
* See parent: REFACTOR-037
