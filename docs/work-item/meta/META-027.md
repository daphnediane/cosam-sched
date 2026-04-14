# Phase 3 — CRDT Integration

## Summary

Phase tracker for adding CRDT-backed storage underneath the entity/field system.

## Status

In Progress

## Priority

Medium

## Description

Design and implement the CRDT abstraction layer and replace the direct HashMap
entity storage with a CRDT-backed equivalent. This enables concurrent offline
editing and eventual merge without a central server.

## Work Items

- FEATURE-010: Edit command system with undo/redo history
- FEATURE-011: CRDT abstraction layer design
- FEATURE-012: CRDT-backed entity storage
- FEATURE-013: Change tracking and merge operations
