# Phase 3 — CRDT Integration

## Summary

Phase tracker for making an automerge CRDT document the authoritative storage
underneath `Schedule`.

## Status

Open

## Priority

Medium

## Description

Make the automerge CRDT document the single source of truth for all entity
and edge data in `Schedule`. The in-memory `HashMap` entity store and
`RawEdgeMap` become pure derived caches that are rebuilt from the document
on load/merge and kept in sync on every write.

CRDT support is **not optional** — there is no feature flag, no
`Option<Box<dyn CrdtStorage>>` sidecar. Every `Schedule` owns an
`automerge::AutoCommit` directly, and every field write flows through it.

Edges are stored as relationship-list fields on a canonical owner entity,
following a panels-outward ownership rule:

- Panel owns `presenter_ids`, `event_room_ids`, `panel_type_id`
- EventRoom owns `hotel_room_ids`
- Presenter (member) owns `group_ids`

This gives automerge-native OR-set-ish add-wins semantics on concurrent
relationship edits without a separate edge-entity layer.

See `docs/crdt-design.md` for the settled design and path layout.

## Work Items

- FEATURE-022: Automerge-backed Schedule storage (single source of truth)
- FEATURE-023: CRDT-backed edges via relationship lists
- FEATURE-024: Change tracking, merge, and conflict surfacing
