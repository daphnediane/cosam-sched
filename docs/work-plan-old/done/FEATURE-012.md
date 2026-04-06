# v5 Rust structs in schedule-core

## Summary

Define the Rust data structures for the v5 JSON format in `crates/schedule-core`.

## Status

Completed

## Priority

High

## Description

Implement the Rust types that correspond to the v5 private JSON format
specified in `docs/json-private-v5.md`. This is the foundational step required
by all subsequent v5 work items.

## Implementation Details

### New types in `crates/schedule-core/src/data/`

- `panel.rs` — `Panel`, `PanelPart`, `PanelSession` structs with full serde
  support (camelCase serialization, `skip_serializing_if` for optional private
  fields).
- `panel_id.rs` — `PanelId` utility that parses a Uniq ID string into
  `{ base_id, part_num, session_num }` components. Regex: `^([A-Z]+\d+)(?:P(\d+))?(?:S(\d+))?$`.

### Update `Schedule` struct (`schedule.rs`)

Add `panels: IndexMap<String, Panel>` field (requires `indexmap` crate for
deterministic ordering). The existing `events: Vec<Event>` field is **removed**;
all data is stored in `panels`.

### `Cargo.toml` changes

Add `indexmap` dependency to `crates/schedule-core/Cargo.toml`.

## Acceptance Criteria

- All v5 fields from `docs/json-private-v5.md` are represented in the structs.
- `PanelId` correctly parses `"GP002"`, `"GW097P1"`, `"GW097P2S3"`.
- Structs round-trip correctly through `serde_json`.
- Unit tests cover parsing and serialization.

## References

- [json-private-v5.md](../json-private-v5.md)
- [json-public-v5.md](../json-public-v5.md)
