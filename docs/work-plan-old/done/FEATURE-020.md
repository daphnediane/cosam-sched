# Implement credit display logic

## Summary

Implement the `credits` field generation based on `always_shown`/`always_grouped` semantics from schedule-to-html.

## Status

Completed

## Priority

Medium

## Description

The `credits` field on public/display panels is currently written as an empty array. The credit display logic needs to resolve group membership, `always_shown`, and `always_grouped` flags to produce the correct public-facing presenter names.

### Credit Resolution Algorithm

Based on `schedule-to-html`'s `PresenterSet._get_credits_shown` and `_get_credited_as`:

1. For each credited presenter on a panel, check their group memberships
2. If the presenter's group has `always_shown` set:
   - If the member has `always_grouped` → show just the group name
   - If only one non-`always_grouped` member is present → show "Group (Member)"
   - If all members are present → show group name
3. If all members of a group are present on the panel → show group name
4. Otherwise → show the individual presenter name
5. Deduplicate group names that appear multiple times

### Implementation Details

- Add credit resolution to the display export path (`public_export.rs` / `display_export.rs`)
- Use the presenter list and group relationships from the `Schedule` struct
- Handle edge cases: presenters in multiple groups, nested groups, `hidePanelist`, `altPanelist`

### Dependencies

- Requires v7 presenter struct changes (`always_shown` field)
- Requires BUGFIX-007 (correct `==Group` parsing)

## Acceptance Criteria

- Credits correctly show group names when all members are present
- `always_shown` groups appear in credits even with partial membership
- `always_grouped` members never appear individually
- `hidePanelist` suppresses all credits
- `altPanelist` overrides computed credits
- Output matches schedule-to-html behavior for known test cases
