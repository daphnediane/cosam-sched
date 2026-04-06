# JSON Format V11 — Schedule-Data Native Format

## Summary

Define and implement a new JSON format version aligned with schedule-data's internal model.

## Status

Not Started

## Priority

High

## Description

The current JSON format (V10) was designed around schedule-core's data model. Schedule-data uses a fundamentally different internal model (monotonic IDs, entity/edge split, field system). A new format version is needed to serialize/deserialize schedule-data's `Schedule` struct directly.

## Implementation Details

- Design V11 schema that maps naturally to schedule-data entities and edges
- Internal IDs as primary keys; external UIDs preserved as indexed fields
- Entities serialized by type with all stored fields from `XxxData` structs
- Edges serialized separately by relationship type
- Metadata section with version, generator, and ID allocator state for round-trip fidelity
- Standardize field case conventions (resolve V9 camelCase/snake_case inconsistency from old CLEANUP-041)
- Support `metadata` / extras field on entities for non-standard spreadsheet columns (from old FEATURE-019)
- Maintain backward-compatible reading of V10 format with migration path
- Add `Serialize`/`Deserialize` derives to `XxxData` structs via macro
- Document format in `docs/json-schedule/` and `docs/json-v11-full.md`

## Acceptance Criteria

- V11 schema documented with per-entity and per-edge specifications
- Round-trip: load V11 → in-memory Schedule → save V11 produces identical output
- V10 files can be loaded and migrated to V11 in memory
- Case conventions are consistent and documented
- Metadata/extras fields preserved through round-trip
