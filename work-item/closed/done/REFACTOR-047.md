# REFACTOR-047: Share field-descriptor macros across entity types

## Summary

Extract the `macro_rules!` helpers from `panel.rs` into a shared `field_macros.rs`
and adopt them in `panel_type.rs` to eliminate per-entity boilerplate.

## Status

Completed

## Priority

Medium

## Blocked By

- None (FEATURE-014 and FEATURE-015 are Completed)

## Description

FEATURE-014's evaluation point asked whether `macro_rules!` helpers for field
descriptor declarations reduce boilerplate enough to warrant adopting. FEATURE-016
introduced five local macros in `panel.rs` (`req_string_field!`, `opt_string_field!`,
`opt_text_field!`, `bool_field!`, `opt_i64_field!`) plus private edge-stub helper
fns (`edge_read_empty_list`, `edge_read_none`, `edge_write_noop`). These collapse
~20-line `FieldDescriptor` literals into ~4-line invocations.

`panel_type.rs` still hand-writes all 11 stored descriptors (~250 lines) matching
the same shapes, and duplicates `substring_match` logic inline. Future entities
(Presenter, EventRoom, HotelRoom in FEATURE-016 follow-ons) will hit the same
patterns.

### Decision

Adopt `macro_rules!` as the shared mechanism. Alternatives considered and
rejected:

- **Proc-macros deriving from struct fields** — explicitly rejected (v10-try3
  approach obscures the hand-written `CommonData` struct).
- **`const fn` builders / `FieldDescriptorConfig`** — cannot abstract the
  per-field accessor closure (`|d| &d.data.$field`) while keeping `static`
  construction, so they don't actually shrink the boilerplate.
- **Hand-written descriptors** — current `panel_type.rs` state; high repetition,
  scales poorly as entity types are added.

### Scope

- New `crates/schedule-core/src/field_macros.rs` with:
  - `pub(crate) fn substring_match` (moved from `panel.rs`).
  - Stored-field macros generalized on `$entity:ty, $internal:ty`:
    `req_string_field!`, `opt_string_field!`, `opt_text_field!`, `bool_field!`,
    `opt_i64_field!`.
  - Edge-stub macros: `edge_list_field!` (read-only), `edge_list_field_rw!`
    (read + no-op write), `edge_none_field_rw!` (singular), `edge_mutator_field!`
    (write-only).
- Refactor `panel.rs` to consume shared macros; delete local macros and edge
  helper fns.
- Refactor `panel_type.rs` to consume shared macros for the 11 uniform fields
  plus `FIELD_PANELS`; leave `FIELD_DISPLAY_NAME` hand-written (bespoke
  computed read + dual-source index logic).
- Leave `CommonData` structs, `FIELD_START_TIME`, `FIELD_END_TIME`,
  `FIELD_DURATION`, and future real edge mutators (FEATURE-018) hand-written.

## Acceptance Criteria

- `field_macros.rs` exists and is consumed by both `panel.rs` and `panel_type.rs`.
- `panel.rs` no longer defines local field macros or `substring_match`.
- `panel_type.rs` stored descriptors and `FIELD_PANELS` are macro invocations.
- All existing tests pass unchanged (field counts, aliases, read/write,
  index matching, serde round-trips).
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- FEATURE-014 eval note updated to record the decision and link here.

## Notes

Parent meta: META-003.

Written with assistance from Windsurf AI.
