# IDEA-101: Decide what ScheduleMetadata.version is for

## Summary

Decide the long-term use of `ScheduleMetadata.version` and update its doc comment and all
call sites accordingly.

## Status

Open

## Priority

Low

## Description

`ScheduleMetadata` has a `version: u32` field whose doc comment says "Monotonically
increasing edit version counter" but the user says it is a file-format/schema version that
should stay at `0`. There is a discrepancy between the comment and the intended use.

### Options

1. **Schema-migration version** — bump only when a data-schema change requires migration
   tooling (e.g., a field is renamed or its storage format changes). Currently always `0`
   since no migrations have been needed. This matches the user's intent.
2. **Edit version counter** — bump on every `EditContext::apply()` call via
   `touch_modified()`. Provides a cheap "how many edits have been made" signal without
   inspecting the automerge history. Matches the current doc comment.
3. **Remove** — rely on automerge change history for provenance; remove the field entirely
   from `ScheduleMetadata` to eliminate confusion.

### Resolution needed

Pick one option, update the doc comment on `ScheduleMetadata::version`, and update any code
that currently writes or reads `version` (including `cosam-convert` which sets it, and any
tests that assert its value).
