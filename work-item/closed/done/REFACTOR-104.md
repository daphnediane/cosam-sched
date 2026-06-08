# REFACTOR-104: Replace cost string with AdditionalCost enum in Panel

## Summary

Replace `PanelCommonData.cost: Option<String>` with a typed `AdditionalCost` enum
and a separate `for_kids: bool` flag, making invalid cost states unrepresentable.

## Status

Completed

## Priority

High

## Description

The current `cost: Option<String>` field stores raw cost strings (e.g. `"$35"`,
`"TBD"`) and four computed fields derive classification from it. This allows
contradictory states (e.g. cents set but also marked TBD). The refactor replaces
the raw string with a proper enum.

## Implementation Details

1. Add `value/cost.rs` — define `AdditionalCost { Included, TBD, Premium(u64) }`,
   `FieldTypeItem::AdditionalCost`, `FieldValueItem::AdditionalCost(AdditionalCost)`,
   and `AsAdditionalCost` converter marker.
2. Extend `schedule-macro` `common_output.rs` to recognize `AdditionalCost` item type.
3. Replace `PanelCommonData.cost: Option<String>` with
   `additional_cost: AdditionalCost` and add `for_kids: bool`.
4. Rewrite panel field statics: `FIELD_ADDITIONAL_COST` (stored accessor),
   `FIELD_COST` (computed string — synthesized), `FIELD_FOR_KIDS` (stored bool),
   and keep derived read-only aliases (`panel_is_included`, `cost_is_tbd`,
   `effective_cost`, `is_kid_panel`).
5. Update xlsx read (`normalize_cost` → `parse_additional_cost`), xlsx write
   (synthesize cost string), and `query/export.rs` widget panel export.

## Acceptance Criteria

- `cargo test` passes
- `cargo clippy` clean
- Field count test updated
- Serialization round-trip tests for `AdditionalCost`
