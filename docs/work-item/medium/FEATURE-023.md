# CRDT-backed Entity Storage

## Summary

Replace direct `HashMap` entity storage with CRDT-backed storage using automerge.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-022: CRDT abstraction layer design

## Description

Implement the CRDT abstraction layer (FEATURE-022) with automerge as the
concrete backend, replacing in-memory `HashMap<NonNilUuid, Data>` collections
with CRDT-backed equivalents.

Write-through: field writes propagate to automerge document based on
`CrdtFieldType`. Materialization: on load, entities are reconstructed from
CRDT state using `crdt_fields` metadata on each `FieldSet`.

## Acceptance Criteria

- Entity storage backed by automerge document
- Field writes propagate to CRDT based on CrdtFieldType
- Entities can be materialized from CRDT state
- Unit tests for round-trip through CRDT storage
