# Replace EntityId with uuid::Uuid for all IDs

## Summary

Migrate from internal u64-based entity IDs to standard UUID v4 for entities, schedules, and edges to enable cross-schedule ID sharing and simplify the public API.

## Status

In Progress

## Priority

High

## Description

Currently the codebase uses `EntityId` (a crate-private u64 type alias) for internal entity identifiers and `InternalId` as a public wrapper. This design has limitations:

* Entity IDs cannot be shared across different schedules (e.g., a guest reinvited in a future year)
* Requires opaque wrapper to hide internal implementation
* Public API exposure of internal types

Replace with standard `uuid::NonNilUuid` (wrapping `uuid::Uuid` v7) for:

* All entity IDs (panels, presenters, rooms, etc.)
* Schedule IDs
* Edge IDs (relationships between entities)

Since UUIDs are standard and self-describing, they can be made public without an opaque wrapper.

## Implementation Details

### Phase 1: Update type definitions ✅ Done

* All entity ID structs (`PanelId`, `PresenterId`, `EventRoomId`, `HotelRoomId`, `PanelTypeId`) now wrap `NonNilUuid`
* `InternalData` trait uses `NonNilUuid` for `uuid()`/`set_uuid()`
* All `From<Uuid>` → `From<NonNilUuid>` and added `try_from_raw_uuid(Uuid) -> Option<Self>` at boundaries

### Phase 2: Update schedule-macro ✅ Done

* Generated `entity_uuid` field is `uuid::NonNilUuid`
* `InternalData` impl uses `NonNilUuid`
* UUID generated via `uuid::Uuid::now_v7()` + `NonNilUuid::new_unchecked` on entity creation

### Phase 3: Update storage ✅ Done

* `EntityStorage` replaced with concrete typed `HashMap<NonNilUuid, XxxData>` per entity type
* `TypedStorage` trait on `EntityType` marker structs provides typed map access
* All edge storages (`GenericEdgeStorage`, `PanelToPanelTypeStorage`, `PanelToPresenterStorage`,
  `PresenterToGroupStorage`, `EventRoomToHotelRoomStorage`) use `NonNilUuid` keys

### Phase 4: Update public APIs ✅ Done

* All `Schedule` methods (`get_entity`, `get_entity_by_uuid`, `find_entities`, `get_entities`,
  `add_entity`, `update_entity`, etc.) typed by `TypedStorage` bound
* All edge storage `find_outgoing`/`find_incoming`/`edge_exists` use `NonNilUuid`

### Phase 5: Add type resolution ✅ Done

* `TypedId` trait: typed ID wrappers with `EntityType` association and `non_nil_uuid()`
* `EntityUUID` enum: `NonNilUuid` tagged by entity kind
* `Schedule::identify(uuid) -> Option<EntityUUID>` using central `entity_registry`
* `Schedule::fetch_entity(id: impl TypedId)` — zero-dispatch borrow of internal data
* `Schedule::fetch_typed(id)` / `Schedule::lookup_typed(id)` — owned public / borrowed enum values
* `Schedule::fetch_uuid(uuid)` / `Schedule::lookup_uuid(uuid)` — via `identify()` dispatch
* `Finder<T>` and `Updater<T>` now bounded by `TypedStorage` throughout

### Phase 6: Update tests ✅ Done

* All four integration test files updated to `NonNilUuid`-based construction
* Unit tests in entity modules updated (`try_from_nil_uuid_returns_none` tests added)
* Edge storage tests updated to use `NonNilUuid::new_unchecked` helpers
* See REFACTOR-049 for remaining new test coverage

### Phase 7: Update external formats 🔲 Remaining

* Update JSON schema to use UUID strings for IDs
* Update XLSX import/export to handle `NonNilUuid`-based entity IDs
* Update converter CLI (`cosam-convert`, `cosam-modify`)

## Acceptance Criteria

* ✅ All entity and edge storage uses `NonNilUuid`
* ✅ No `EntityId` / `InternalId` remain in the codebase
* ✅ Public API uses typed ID wrappers backed by `NonNilUuid`
* ✅ Tests pass (101 passing)
* 🔲 JSON format updated to use UUID strings
* 🔲 XLSX import/export handles `NonNilUuid`-based IDs correctly
* ✅ Cross-schedule entity sharing is possible (same UUID can exist in multiple schedules)

## Notes

* **UUID v7 was chosen** — time-ordered (monotonic) UUIDs generated via `uuid::Uuid::now_v7()`, wrapped in `NonNilUuid` to guarantee non-nil invariant
* UUID v4 (random) was the original plan and remains collision-resistant, but v7 provides better index locality in sorted/range-query scenarios
* If stable IDs are needed (e.g., for external systems or deterministic re-import), consider UUID v5 (name-based, SHA-1 hash of a namespace + name) to assign UUIDs deterministically from external identifiers
* Memory impact: 16 bytes per ID vs 8 bytes - acceptable for typical schedule sizes (hundreds to thousands of entities)
