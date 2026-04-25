# REFACTOR-064: Update Schedule edge APIs for FieldNodeId

## Summary

Adapt `schedule.rs`, `edge_crdt.rs`, and `edge_cache.rs` to use the new FieldNodeId-based
`RawEdgeMap`, replacing type-parameter-based edge lookups with field-based lookups.

## Status

Open

## Priority

High

## Blocked By

- REFACTOR-063: FieldNodeId-based RawEdgeMap

## Description

This is Phase 4 of the FieldNodeId edge system refactor.

- Replace `edges_from::<L, R>` / `edges_to::<L, R>` with field-aware variants:
  `edges_for_field(uuid, field_id)` and typed wrapper
  `edges_from_field::<E, R>(id, &FIELD_X) -> Vec<EntityId<R>>`.
- Update `edge_add`, `edge_remove`, `edge_set`, `edge_set_to` to dispatch via EdgeDescriptor
  `owner_field` / `target_field`.
- Remove `edge_get_bool`, `edge_set_bool` (no more per-edge metadata).
- Update CRDT mirror ops in `edge_crdt.rs`: use `owner_field.name()` for CRDT field name;
  iterate via `all_edge_descriptors()` instead of `ALL_EDGE_DESCRIPTORS`.
- Update `HomoEdgeCache` (`edge_cache.rs`): key by `(FieldId, NonNilUuid)`; trigger rebuild on
  mutations to `is_transitive` edge fields.
- Remove all homo-specific branches from schedule traversal methods.
