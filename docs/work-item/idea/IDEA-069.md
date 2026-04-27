# IDEA-069: Add EdgeOwner/EdgeTarget variants to CrdtFieldType

## Summary

Encode CRDT edge ownership direction directly in `CrdtFieldType` instead of
relying solely on `EdgeDescriptor` and `canonical_owner()`.

## Status

Open

## Priority

Low

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

### Fields that would change

- `rw` / `rw_to` / `ro` mode fields (e.g. `FIELD_PRESENTERS` would become
  `EdgeOwner`, `FIELD_PANELS` would become `EdgeTarget`)
- Write-only and computed fields (`add_panels`, `remove_panels`,
  `inclusive_groups`) stay `Derived`

### Open questions

- Should `EdgeOwner` carry a reference to the target field (or vice versa)
  to make the pair self-describing?
- Does this overlap with or subsume the `EdgeDescriptor` registry entirely?
- Impact on the field macro — would `rw` / `rw_to` modes auto-select the
  variant, or should it be explicit?
