# Update and extend tests for UUID migration

## Summary

Update the four existing integration test files to use `Uuid` instead of `EntityId`/`InternalId`, and add new tests for `fetch_uuid` and `lookup_uuid`.

## Status

In Progress

## Priority

High

## Description

Part of REFACTOR-037. After all implementation phases are complete, the test suite needs to be updated to reflect the new UUID-based API and extended to cover the new registry methods.

Files to update in `crates/schedule-data/tests/`:

* `entity_fields_integration.rs` — replace `EntityId` references with `Uuid`; update entity construction to use generated `*Data::new(...)` constructors
* `direct_indexable_test.rs` — same EntityId → Uuid updates
* `indexable_fields_test.rs` — same updates
* `simple_indexable_test.rs` — same updates

**All four files above are updated and passing as of the working branch.**

New tests to add (can be in `entity_fields_integration.rs` or a new `uuid_registry_test.rs`):

* `test_schedule_metadata_has_uuid` — verify `ScheduleMetadata::new()` generates a non-nil `schedule_id`
* `test_fetch_uuid_panel` — add a panel to a schedule, call `fetch_uuid(panel.uuid())`, verify returned `PublicEntityRef::Panel` matches
* `test_fetch_uuid_unknown_returns_none` — call `fetch_uuid` with a random UUID, verify `None`
* `test_lookup_uuid_returns_borrowed_data` — verify `lookup_uuid` returns `EntityRef::Panel(&PanelData)` for a known panel
* `test_type_of_uuid` — verify `type_of_uuid` returns `Some(EntityKind::Panel)` for a known panel UUID and `None` for an unknown UUID
* `test_entity_data_new_generates_unique_uuids` — create two `PanelData::new(...)` instances, verify UUIDs differ
* `test_to_public_roundtrip` — create `PanelData`, call `to_public()`, verify all stored fields match

**None of the seven new tests above are written yet. They are now unblocked** — `EntityStorage` stores real data (no longer a stub), so data round-trip tests will pass. Routing/dispatch tests (`fetch_uuid_routes_through_identify`, `identify_kind_matches_entity_kind`, etc.) exist in `schedule/mod.rs` inline tests.

## Acceptance Criteria

* ✅ All four existing test files updated and passing
* 🔲 All seven new tests added and passing
* 🔲 `cargo test` at workspace root passes with no failures after new tests added

## Notes

* If adding a new test file `uuid_registry_test.rs`, include the standard copyright header
* See parent: REFACTOR-037
