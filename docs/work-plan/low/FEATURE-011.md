# Groups-of-Groups Presenter Processing

## Summary

Support nested group membership where a group's members can include other groups.

## Status

Not Started

## Priority

Medium

## Description

The `schedule-to-html` Perl project supported groups-of-groups, where a group's members list could include the name of another group. The current Rust code does not handle this. Implement recursive group expansion with cycle detection for credit resolution and conflict detection.

## Implementation Details

- When resolving group membership, recursively expand nested groups
- Handle circular references gracefully (detect and break cycles, matching `relationship.rs` tolerance)
- Update credit resolution for nested groups:
  - If umbrella group's sub-groups are all fully present, show umbrella group name
  - If only some sub-groups present, show sub-group names
- Update conflict detection to consider all individual members of nested groups
- Spreadsheet syntax: `G:==SubGroup` or `G:=SubGroup` defines a group without listing members directly

## Acceptance Criteria

- Groups can list other groups as members
- Recursive expansion correctly identifies all individual members
- Circular group references handled without infinite loops
- Credit resolution works correctly with nested groups
- Conflict detection considers all individual members of nested groups
