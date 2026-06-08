# Phase 4 — File Formats & Import/Export

## Summary

Phase tracker for internal file format, multi-year archive, widget JSON, and
XLSX import/export.

## Status

Completed

## Priority

Medium

## Blocked By

- META-004: Phase 3 — CRDT Integration (FEATURE-025 wraps `Schedule::save`/`load`) ✓ resolved

## Description

Define and implement all file format support: the internal native format with
CRDT state, widget display JSON export, and round-trip XLSX import/export for
the convention spreadsheet workflow.

Multi-year archive support (FEATURE-026) deferred out of this phase.

## Work Items

- FEATURE-025: Internal schedule file format (save/load) — Completed
- FEATURE-056: Synthesized data fields for export — Completed
- FEATURE-027: Widget display JSON export — Completed
- FEATURE-028: XLSX spreadsheet import (blocked by FEATURE-020) -- Completed
- FEATURE-029: XLSX spreadsheet export (blocked by FEATURE-028) -- Completed
