# CRDT Abstraction Layer Design

## Summary

Design the abstraction layer between the entity/field system and the CRDT backend.

## Status

Open

## Priority

Medium

## Blocked By

- FEATURE-021: Edit command system with undo/redo

## Description

Before integrating a specific CRDT library, define the abstraction boundary so
the entity system doesn't depend directly on CRDT internals.

Uses the `CrdtFieldType` annotations (Scalar, Text, List, Derived) on field
descriptors to drive write-through and materialization without per-entity tables.

See `docs/crdt-design.md` for the settled design decisions.

## Acceptance Criteria

- Abstraction trait defined for CRDT document operations
- Field-level CRDT routing based on CrdtFieldType works
- Unit tests with a mock CRDT backend
