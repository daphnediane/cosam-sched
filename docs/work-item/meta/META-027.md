# Phase 3 — CRDT Integration

## Summary

Phase tracker for adding CRDT-backed storage underneath the entity/field system.

## Status

Blocked

## Priority

Medium

## Description

Design and implement the CRDT abstraction layer and replace the direct HashMap
entity storage with a CRDT-backed equivalent. This enables concurrent offline
editing and eventual merge without a central server.

## Work Items

- FEATURE-011: CRDT abstraction layer design
- FEATURE-012: CRDT-backed entity storage
- FEATURE-013: Change tracking and merge operations

## Blocked By

- META-026: Phase 2 — Core Data Model

## Blocks

- META-031: Phase 7 — Sync & Multi-User
