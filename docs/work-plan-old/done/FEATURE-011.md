# JSON v5 format specification

## Summary

Define the v5 JSON format for the schedule data, introducing a
base→part→session hierarchy, public/private split, and multi-room sessions.

## Status

Completed

## Priority

High

## Description

The existing v4 format (see `docs/json-format-v4.md`) stores schedule data as a
flat array of events. Version 5 redesigns the structure to better match the
spreadsheet data model:

- Related panels (sharing a base Uniq ID) are organized under a single base
  panel record with nested parts and sessions.
- Sessions support multiple rooms via a `roomIds` array.
- Private internal fields (workshop notes, power needs, sewing machines, etc.)
  are separated from public-facing fields.
- Two output variants: `full` (private, all fields) and `public` (widget-ready,
  public fields only, flattened).

## Acceptance Criteria

- `docs/json-private-v5.md` documents the full/internal format.
- `docs/json-public-v5.md` documents the public/widget format.
- `docs/json-format.md` updated to index both new specs and archive v4.
- All future work items reference the v5 spec documents.

## References

- [json-private-v5.md](../json-private-v5.md)
- [json-public-v5.md](../json-public-v5.md)
- [json-format-v4.md](../json-format-v4.md) (archived v4 spec)
