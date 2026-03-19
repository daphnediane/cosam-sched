# Repository Layout and Legacy Cleanup Plan

## Summary

Complete repository layout cleanup by moving planning outputs under `docs/`, relocating work-plan tools to `scripts/`, and retiring deprecated Perl converter paths.

## Status

Completed

## Priority

Medium

## Description

This cleanup pass completed the deferred repository reorganization in one focused migration, improving top-level clarity and script placement while preserving historical references for legacy converter behavior.

## Implementation Details

- Moved planning docs and aggregate output:
  - `work-plan/` -> `docs/work-plan/`
  - `WORK_PLAN.md` -> `docs/WORK_PLAN.md`
- Moved work-plan maintenance tools into `scripts/`:
  - `work-plan/combine_workplans.pl` -> `scripts/combine_workplans.pl`
  - `work-plan/fix_markdown_format.pl` -> `scripts/fix_markdown_format.pl`
  - `work-plan/update_workplan.sh` -> `scripts/update_workplan.sh`
- Updated scripts, docs, and `.windsurf/rules/` path references for the new layout
- Updated `README.md` repository layout documentation
- Removed deprecated Perl converter paths and Google Sheets converter docs/config now tracked in archive branch context

## Acceptance Criteria

- `docs/work-plan/` and `docs/WORK_PLAN.md` are canonical plan locations
- Work-plan tools run from `scripts/` with updated invocations
- README and rules reflect final path conventions
- Deprecated legacy converter paths are removed from active docs/scripts
