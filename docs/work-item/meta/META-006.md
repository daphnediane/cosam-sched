# Phase 5 — CLI Tools

## Summary

Phase tracker for the cosam-convert and cosam-modify command-line applications.

## Status

In Progress

## Priority

Low

## Blocked By

- META-005: Phase 4 — File Formats & Import/Export

## Description

Implement the two CLI applications for format conversion and batch editing.
These applications wrap `schedule-core`'s import/export and edit command systems.

## Work Items

- CLI-030: cosam-convert: format conversion tool
- CLI-031: cosam-modify: CLI editing tool
- CLI-090: schedule-core metadata update API
- CLI-091: cosam-modify scaffold, file I/O, module structure
- CLI-092: list and get commands
- CLI-093: set command
- CLI-094: create command
- CLI-095: delete command
- CLI-096: add-edge / remove-edge commands
- CLI-097: undo / redo / show-history (in-memory)
- CLI-098: help text, exit codes, integration tests, polish
- CLI-099: undo/redo history persistence in binary file (not started)
- CLI-100: interactive mode — --interactive REPL (not started)
- IDEA-101: decide what ScheduleMetadata.version is for
