# Update entity storage to use Uuid keys

## Summary

Replace `HashMap<u64, StoredEntity>` and `u64`-keyed internals in `schedule/storage.rs` with `HashMap<uuid::Uuid, StoredEntity>`.

## Status

Completed

## Priority

High

## Description

Part of REFACTOR-037. `EntityStorage` in `schedule/storage.rs` stores entities serialized as JSON strings, keyed by a `u64` internal ID. After the entity ID migration these keys become `uuid::Uuid`.

Changes to `crates/schedule-data/src/schedule/storage.rs`:

* `EntityTypeStorage::by_internal_id: HashMap<u64, ...>` → `HashMap<uuid::Uuid, ...>`
* `StoredEntity` struct: `internal_id: u64` field → `internal_uuid: uuid::Uuid`
* `EntityStorage::add_with_id(id: EntityId, ...)` → `add_with_uuid(uuid: Uuid, ...)`
* `EntityStorage::get(id: EntityId)` → `get(uuid: Uuid)`
* `EntityStorage::contains_id(id: EntityId)` → `contains_uuid(uuid: Uuid)`
* Update all internal `HashMap::get`, `HashMap::insert`, `HashMap::contains_key` calls to use `Uuid`
* Remove import of `EntityId`; add `use uuid::Uuid`

The `deserialize` function stub is kept as-is (it returns `None`); only the key type changes.

## Acceptance Criteria

* `EntityStorage` compiles with `HashMap<Uuid, StoredEntity>` keys
* Method signatures updated from `EntityId` to `Uuid`
* No remaining `u64` ID references in `storage.rs` (only `EdgeId(u64)` uses u64 elsewhere)

## Notes

* `EntityStorage::deserialize` stub is not fixed in this phase
* See parent: REFACTOR-037
