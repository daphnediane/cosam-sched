# REFACTOR-063: Redesign RawEdgeMap with FieldNodeId storage

## Summary

Replace the two-map `RawEdgeMap` with a nested `HashMap<NonNilUuid, HashMap<FieldId, Vec<FieldNodeId>>>`,
eliminating the `homogeneous_reverse` special case.

## Status

Open

## Priority

High

## Blocked By

- REFACTOR-061: FieldDescriptorAny / FieldId / FieldNodeId foundation
- REFACTOR-062: Redesigned EdgeDescriptor

## Description

This is Phase 3 of the FieldNodeId edge system refactor.

New `RawEdgeMap` structure:

```text
HashMap<NonNilUuid, HashMap<FieldId, Vec<FieldNodeId>>>
```

- Outer key: entity UUID
- Inner key: FieldId (which field on that entity)
- Values: `FieldNodeId` pairs for the other side of each edge

Both directions of every edge are stored symmetrically. Homogeneous and heterogeneous edges are
treated identically — no `homogeneous_reverse` needed.

New public API: `add_edge`, `remove_edge`, `set_field_neighbors`, `neighbors_for_field`,
`clear_all`. Remove: `add_het`, `remove_het`, `add_homo`, `remove_homo`, `set_neighbors`,
`neighbors`, `homo_reverse`.

Full test suite rewrite for all operations and `clear_all` consistency.
