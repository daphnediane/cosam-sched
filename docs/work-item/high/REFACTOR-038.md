# Migrate EdgeId from u64 to uuid::Uuid

## Summary

Replace the `EdgeId(u64)` type with `EdgeId(uuid::Uuid)` and add an edge UUID registry to `Schedule` for cross-edge lookups.

## Status

Open

## Priority

High

## Description

Currently `EdgeId` is a `(u64)` newtype generated sequentially in each edge storage. This is an internal counter with no cross-storage identity guarantees. Migrating to UUID v7 enables:

* Stable edge references across sessions and serialization round-trips
* Unified `Schedule::lookup_edge_uuid` registry alongside the entity UUID registry
* Consistent identity model for all objects in the schedule

Previously blocked on entity UUID migration (REFACTOR-037 phases 1–6). That work is now complete — entity IDs and all storage use `NonNilUuid`. Edge UUIDs are the natural next step.

## Implementation Details

* `EdgeId(u64)` → `EdgeId(uuid::Uuid)` in `edge/traits.rs`
* All per-storage `next_id: u64` counters replaced with `uuid::Uuid::now_v7()` at allocation time
* `Schedule`: add `edge_registry: HashMap<Uuid, EdgeKind>` where `EdgeKind` is an enum of edge types
* Add `Schedule::lookup_edge_uuid(uuid: Uuid) -> Option<EdgeRef<'_>>` returning borrowed edge data
* Update `GenericEdgeStorage` and `PresenterToGroupStorage` to store `EdgeId(Uuid)` keys
* Update serialization/deserialization for edge IDs

## Acceptance Criteria

* `EdgeId` uses `uuid::Uuid` internally
* No sequential `next_id` counters remain in edge storage
* `Schedule::lookup_edge_uuid` returns an edge reference for any known edge UUID
* All existing edge tests pass with UUID-based IDs

## Notes

* `Uuid: Hash + Eq + Ord` so `HashMap` and `BTreeSet` usage is unaffected
* See parent: REFACTOR-037
