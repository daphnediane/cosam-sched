# EDITOR-111: Extract shared schedule_data module to crates/cosam-editor-shared

## Summary

Extract the duplicated `schedule_data.rs` UI helper present in both
`cosam-editor-gpui` and `cosam-editor-dioxus` into a new
`crates/cosam-editor-shared` crate once the GUI framework is chosen.

## Status

Open

## Priority

Low

## Blocked By

- EDITOR-032: GUI framework must be selected first

## Description

Both `apps/cosam-editor-gpui/src/ui/schedule_data.rs` and
`apps/cosam-editor-dioxus/src/ui/schedule_data.rs` contain identical
or near-identical logic for adapting `schedule-core` data for display.
Once the framework decision is made the surviving copy should move to
`crates/cosam-editor-shared` so it can be reused by any future editor
target without duplication.

## Implementation Details

- Create `crates/cosam-editor-shared` as a workspace member
- Move `schedule_data.rs` into it; adjust imports in the surviving editor app
- Remove the dead copy from the dropped editor app
- Update the `// TODO(EDITOR-111)` comment in the surviving file

## Acceptance Criteria

- `cargo build -p cosam-editor-shared` succeeds
- No duplicate `schedule_data` logic remains across editor crates
