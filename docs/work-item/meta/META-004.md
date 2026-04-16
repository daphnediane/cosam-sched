# Phase 3 — CRDT Integration

## Summary

Phase tracker for adding CRDT-backed storage underneath the entity/field system.

## Status

Blocked

## Priority

Medium

## Blocked By

- META-003: Phase 2 — Core Data Model

## Description

Design and implement the CRDT abstraction layer and replace the direct HashMap
entity storage with a CRDT-backed equivalent. This enables concurrent offline
editing and eventual merge without a central server.

The integration leverages field-level CRDT semantics (`CrdtFieldType` on each
field descriptor) to avoid per-entity boilerplate. Write-through and materialize
patterns iterate the field metadata — no per-entity-kind tables needed.

## Work Items

- FEATURE-022: CRDT abstraction layer design
- FEATURE-023: CRDT-backed entity storage
- FEATURE-024: Change tracking and merge operations
