# v5 private JSON export

## Summary

Implement serialization of the v5 full/private JSON format from the
`Schedule` struct.

## Status

Completed

## Priority

Medium

## Description

Update `Schedule::save_json_with_mode(Full)` to serialize the `panels` hash
per `docs/json-private-v5.md`. The output includes all fields at every level,
including private internal-use fields.

## Implementation Details

### `save_json_with_mode(Full)`

- Set `meta.version = 5` and `meta.variant = "full"`.
- Serialize `panels` as a hash keyed by base ID.
- `IndexMap` preserves insertion order; keys are sorted alphabetically by
  base ID for deterministic output.
- All private fields serialized; no filtering applied.
- `rooms`, `panelTypes`, `timeTypes`, `timeline`, `presenters`, `conflicts`
  serialized identically to v4.

### `JsonExportMode`

Update `JsonExportMode` enum to distinguish `Full` (private) from `Public`
(see FEATURE-016).

## Acceptance Criteria

- Output round-trips: loading the saved file produces an identical `Schedule`.
- All private fields present in output.
- `meta.version` is `5` and `meta.variant` is `"full"`.
- Output matches the schema in `docs/json-private-v5.md`.

## References

- [json-private-v5.md](../json-private-v5.md)
- [FEATURE-012.md](FEATURE-012.md) — v5 Rust structs (prerequisite)
- [FEATURE-014.md](FEATURE-014.md) — xlsx_import v5 (prerequisite)
