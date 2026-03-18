# Repository Layout and Legacy Cleanup Plan

## Summary

Define and execute a staged cleanup plan to move project planning and generated planning outputs under `docs/`, centralize maintenance scripts/tools, and retire legacy Perl conversion code once Rust parity is fully validated.

## Status

Open

## Priority

Medium

## Description

Capture the deferred repository reorganization work so it can be completed in one focused pass after current parity and build goals are stabilized. This cleanup should improve top-level repo clarity, reduce mixed-purpose script placement, and formalize retirement steps for legacy Perl tooling.

## Implementation Details

- Move `work-plan/` under `docs/` and update all references in scripts, docs, and rules
- Move `WORK_PLAN.md` into `docs/` and keep generation logic aligned with that location
- Create a dedicated home for maintenance tooling (for example `tools/` or `scripts/`) and relocate:
  - `work-plan/update_workplan.sh`
  - `rebuild_json.sh`
  - `work-plan/combine_workplans.pl`
- Define clear script naming and invocation conventions for cross-platform use
- Document the final canonical repo layout in `README.md`
- Audit `converter/` and other Perl assets, then define phased retirement milestones
- Remove deprecated Perl paths only after Rust converter output parity and operational workflows are verified

## Acceptance Criteria

- Planned target layout for docs, plans, and tooling is documented and approved
- A migration checklist exists for path updates and script relocations
- Legacy Perl cleanup steps are explicit, sequenced, and gated on parity verification
