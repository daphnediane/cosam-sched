# FEATURE-065: Split credited_presenters into separate CRDT edge fields

## Summary

Convert `credited_presenters` and `uncredited_presenters` on Panel from computed/derived fields
into actual edge storage fields, eliminating the `credited` per-edge boolean and its CRDT
`presenters_meta` map.

## Status

Open

## Priority

High

## Blocked By

- REFACTOR-064: Updated Schedule edge APIs

## Description

This is Phase 5 of the FieldNodeId edge system refactor.

Currently Panel stores one CRDT list (`presenters`) plus a parallel `presenters_meta` map with a
`credited` boolean per entry. `credited_presenters` and `uncredited_presenters` are computed
fields that filter by that boolean.

With FieldNodeId edges, both can be first-class CRDT lists:

- `FIELD_CREDITED_PRESENTERS` on Panel → edge storage; target `FIELD_PANELS` on Presenter.
- `FIELD_UNCREDITED_PRESENTERS` on Panel → edge storage; target `FIELD_PANELS` on Presenter.
- `FIELD_PANELS` on Presenter aggregates entries from both (union via field map accumulation).
- Remove `FIELD_PRESENTERS` (old undivided list) and `EDGE_PRESENTERS`.
- Remove `EdgeFieldSpec`, `EdgeFieldDefault` (no per-edge metadata needed).
- CRDT schema change: replace `presenters` + `presenters_meta` with `credited_presenters` and
  `uncredited_presenters` lists (pre-alpha breaking change is acceptable).
- `FIELD_PANELS` on Presenter becomes read-only
  - `FIELD_ADD_CREDITED_PANELS` -- adds to `credited_presenters`, removes from `uncredited_presenters`
  - `FIELD_ADD_UNCREDITED_PANELS` -- adds to `uncredited_presenters`, removes from `credited_presenters`
  - `FIELD_REMOVE_PANELS` -- removes from `credited_presenters` and `uncredited_presenters`
- Update all tests referencing `credited` per-edge metadata.
