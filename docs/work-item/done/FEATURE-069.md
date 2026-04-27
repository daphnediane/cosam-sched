# FEATURE-069: Add EdgeOwner/EdgeTarget variants to CrdtFieldType

## Summary

Encode CRDT edge ownership direction directly in `CrdtFieldType` instead of
relying solely on `EdgeDescriptor` and `canonical_owner()`.

## Status

Completed

## Priority

High

## Description

Currently all edge-field descriptors use `CrdtFieldType::Derived`, which the
CRDT mirror layer skips entirely during `mirror_entity_fields`.  Ownership
direction lives only in `EdgeDescriptor`, and mirror functions must call
`canonical_owner()` to resolve it at runtime.

Adding `EdgeOwner` / `EdgeTarget` variants to `CrdtFieldType` would:

- Encode CRDT ownership direction directly in the field descriptor
- Enable mirror functions to derive canonical ownership from field descriptors
  without the `canonical_owner()` lookup
- Potentially allow `mirror_entity_fields` to handle edge list mirroring
  automatically during hydration, eliminating the separate
  `ensure_all_owner_lists_for_type` setup pass

### Design decisions

- `EdgeOwner(&'static EdgeDescriptor)` — the owner field carries the full descriptor
- `EdgeTarget` — plain variant, no payload (a field may be the target of multiple edges)
- `add`/`remove`/write-only fields stay `Derived`
- Optional `edge: &EDGE_X` parameter in `edge_field!` macro selects `EdgeOwner` vs `EdgeTarget`
- `rw_to` mode merged into `rw` (generated code was identical; `source:`/`source_field:` was cosmetic)
- `mirror_entity_fields` iterates `EdgeOwner` fields to call `ensure_owner_list`,
  eliminating the separate `ensure_all_owner_lists_for_type` setup pass
