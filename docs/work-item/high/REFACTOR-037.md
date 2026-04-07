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

Replace with standard `uuid::Uuid` v4 for:

* All entity IDs (panels, presenters, rooms, etc.)
* Schedule IDs
* Edge IDs (relationships between entities)

Since UUIDs are standard and self-describing, they can be made public without an opaque wrapper. UUIDs are 128-bit (16 bytes) vs 64-bit (8 bytes) for u64, but this tradeoff is acceptable for the benefits:

* Standard RFC 4122 format
* Built-in collision resistance
* Can be serialized/deserialized reliably
* Enables cross-schedule entity tracking
* No need for opaque wrapper

## Open Questions

### Storage Architecture

Should we change how entities/edges are stored?

* **Current:** Separate storage per entity type (EntityStorage, edge-specific storages)
* **Option A:** Keep current per-type storage with UUID keys
* **Option B:** Unified storage with UUID lookup that returns type information
* **Option C:** Hybrid - per-type storage for performance, with central UUID registry for type resolution

### Type Resolution

Should Schedule provide a "what is this UUID?" lookup?

* **Option A:** Add `Schedule::resolve_uuid(uuid: Uuid) -> Option<EntityType>` that returns entity type and data
* **Option B:** User tracks type knowledge themselves (current pattern)
* **Option C:** Typed wrappers like `PanelId(Uuid)`, `RoomId(Uuid)` for compile-time type safety

### Panel vs Timeline Semantics

Currently panels and timelines may be treated as same/different types in different contexts. With UUIDs:

* Should they share the same ID namespace or have separate ones?
* If separate, how do we handle the "same thing in different contexts" use case?

## Implementation Details

### Phase 1: Update type definitions

* Replace `EntityId` type alias with `uuid::Uuid` in schedule-data
* Remove `InternalId` struct (no longer needed)
* Update `InternalData` trait to use `uuid::Uuid`
* Update `EdgeId` to use `uuid::Uuid`

### Phase 2: Update schedule-macro

* Change generated `entity_id` field to `uuid::Uuid`
* Update `InternalData` implementation
* Update field value conversions
* Generate UUID on entity creation

### Phase 3: Update storage

* Change all HashMap keys from u64 to uuid::Uuid
* Update ID allocators to generate UUIDs instead of sequential u64
* Update serialization/deserialization

### Phase 4: Update public APIs

* Replace all `EntityId` parameters with `uuid::Uuid`
* Replace all `InternalId` parameters with `uuid::Uuid`
* Update query APIs
* Update edge APIs

### Phase 5: Add type resolution (if Option A or C chosen)

* Implement central UUID registry in Schedule
* Add `resolve_uuid()` method
* Or add typed ID wrapper structs

### Phase 6: Update tests

* Update all test fixtures to use UUIDs
* Update test assertions

### Phase 7: Update external formats

* Update JSON schema to use UUID strings for IDs
* Update XLSX import/export to handle UUIDs
* Update converter CLI

## Acceptance Criteria

* All entity, schedule, and edge IDs use `uuid::Uuid`
* No `InternalId` or `EntityId` remain in the codebase
* Public API uses `uuid::Uuid` directly
* Tests pass
* JSON format updated to use UUID strings
* XLSX import/export handles UUIDs correctly
* Cross-schedule entity sharing is possible (same UUID can exist in multiple schedules)

## Notes

* UUID v4 is random and collision-resistant for practical purposes
* If stable IDs are needed (e.g., for external systems), consider UUID v5 (name-based) or assigning UUIDs deterministically from external identifiers
* Memory impact: 16 bytes per ID vs 8 bytes - acceptable for typical schedule sizes (hundreds to thousands of entities)
