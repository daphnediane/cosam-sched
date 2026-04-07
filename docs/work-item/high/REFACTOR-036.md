# Presenter Entity Field Alignment

## Summary

Align Presenter entity field aliases with schedule-core canonical column definitions.

## Status

Not Started

## Priority

High

## Description

Ensure Presenter entity field aliases include canonical forms from schedule-core for proper field resolution. Classification and groups/members handling already match schedule-core pattern.

## Implementation Details

### Classification Handling (Already Correct)

- The current implementation with `rank: PresenterRank` as stored field and `classification` as computed field matches schedule-core pattern
- No changes needed

### Groups/Members Handling (Already Correct)

- Groups and members are correctly handled via edges with computed fields for display
- Do not add direct stored fields - this respects the edge-based architecture

### Verify and Update Field Aliases

- `name`: Add canonical "Person" to aliases
- `is_group`: Add canonical "Is_Group" to aliases
- `always_grouped`: Add canonical "Always_Grouped" to aliases
- `always_shown_in_group`: Add canonical "Always_Shown" to aliases

## Acceptance Criteria

- All field aliases include canonical forms from schedule-core
- Presenter entity compiles and passes tests
- Edge-based architecture respected (groups/members via edges only)
