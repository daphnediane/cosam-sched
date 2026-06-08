# REFACTOR-059: Replace CanonicalOwner/OWNER_EDGE_FIELDS with EdgeDescriptor

## Summary

Introduce `EdgeDescriptor` as a first-class type that co-locates edge definition,
CRDT ownership, and relationship semantics on the canonical owner entity type,
replacing the split `canonical_owner()` match table and `OWNER_EDGE_FIELDS` constant.

## Status

Completed

## Priority

Medium

## Description

The current system has a split-brain problem: edge relationships are defined via
`edge_field!` macros on entity types, but their CRDT ownership is encoded in a
separate hardcoded `canonical_owner()` match table and `OWNER_EDGE_FIELDS` constant
in `edge_crdt.rs`. Adding any new edge type requires editing both files with no
compiler enforcement that they stay in sync.

This refactor introduces `EdgeDescriptor` — a static constant on the canonical
owner entity type that encodes the relationship name, owner/target types,
is_homo flag, CRDT field name, and (future) edge-specific fields. A single
`ALL_EDGE_DESCRIPTORS` registry replaces both `canonical_owner()` and
`OWNER_EDGE_FIELDS` as the authoritative source of truth.

## Implementation Details

- Add `EdgeDescriptor` struct to a new `edge_descriptor.rs` module
- Add `ALL_EDGE_DESCRIPTORS: &[&EdgeDescriptor]` registry
- Add `EDGE_PRESENTERS`, `EDGE_EVENT_ROOMS`, `EDGE_PANEL_TYPE` consts to `PanelEntityType`
- Add `EDGE_HOTEL_ROOMS` const to `EventRoomEntityType`
- Add `EDGE_GROUPS` const to `PresenterEntityType`
- Replace `canonical_owner()` with lookup over `ALL_EDGE_DESCRIPTORS`
- Replace `OWNER_EDGE_FIELDS` with iteration over `ALL_EDGE_DESCRIPTORS`
- Keep `RawEdgeMap`, `TransitiveEdgeCache`, and `Schedule` generic edge API unchanged

## Acceptance Criteria

- `cargo test` passes
- `cargo clippy` clean, no dead-code warnings on removed items
- No changes to `Schedule` public edge API
