# Repository Layout and Legacy Cleanup Plan

## Summary

Migrate to an `apps/` + `crates/` Rust workspace layout, retire the legacy Perl converter now that parity is reached, and track the remaining non-blocking cleanup follow-up items.

## Status

Completed

## Priority

Medium

## Description

This cleanup pass standardized the Rust workspace layout and removed deprecated converter paths.

## Implementation Details

- Adopted Rust workspace layout:
  - `crates/schedule-core/`
  - `apps/cosam-editor/`
  - `apps/cosam-convert/`
- Updated workspace manifests, build scripts, and CLI usage docs for the new paths
- Moved planning docs under `docs/` (`work-plan/` -> `docs/work-plan/`, `WORK_PLAN.md` -> `docs/WORK_PLAN.md`)
- Removed legacy Perl converter implementation under `converter/`
- Removed deprecated Google Sheets converter docs/config (`GOOGLE_SHEETS.md`, `google-sheets-config.example.yaml`)
- Preserved Perl formatting/lint configs because work-plan maintenance scripts are still in Perl
- Recorded archived legacy Google Sheets details in `docs/work-plan/EDITOR-507.md` with branch reference `feature/final-perl-converter`
- Moved work-plan utility scripts to `scripts/` (`combine_workplans.pl`, `fix_markdown_format.pl`, `update_workplan.sh`)

## Acceptance Criteria

- Rust workspace uses the canonical `apps/` + `crates/` layout
- Legacy Perl converter paths are removed from repository and docs
- Build/rebuild scripts and README are updated to the new workspace structure
- Work plan docs and tools are in canonical `docs/` and `scripts/` locations
