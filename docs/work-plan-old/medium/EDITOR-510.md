# Multi-Device Schedule Sync Strategy

## Summary

Define how multiple people and devices can safely edit a single schedule with conflict handling independent of any specific storage backend.

## Status

Open

## Priority

Medium

## Description

Design the synchronization and conflict-resolution model for concurrent editing across desktop clients. This is intentionally backend-agnostic so it can support Google Sheets, OneDrive, or future storage options without rewriting core merge behavior.

## Implementation Details

- Define authoritative data model and versioning strategy for concurrent edits
- Choose conflict detection granularity (event-level, field-level, row-level)
- Define merge rules and user-facing conflict resolution UX
- Add audit metadata for editor identity and change timestamps
- Document offline edit behavior and reconciliation when reconnecting
- Evaluate transport backends (Google Sheets, OneDrive, and others) against the same sync contract
