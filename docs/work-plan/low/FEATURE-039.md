# Support groups-of-groups in presenter processing

## Summary

Support nested group membership (groups whose members include other groups) in presenter processing and credit resolution.

## Status

Open

## Priority

Low

## Description

The `schedule-to-html` project supported groups-of-groups, where a group's members list could include the name of another group. The current Rust code does not handle this case — group membership is assumed to be individuals only.

### Use Cases

- A "Convention Staff" group that includes "Technical Staff" and "Programming Staff" sub-groups
- Umbrella cosplay groups that contain smaller teams

### Implementation Details

1. When resolving group membership, recursively expand nested groups
2. Handle circular references gracefully (detect and break cycles)
3. Update credit resolution to work with nested groups:
   - If an umbrella group's sub-groups are all fully present → show umbrella group name
   - If only some sub-groups are present → show sub-group names
4. Update conflict detection to properly handle nested group membership

### Spreadsheet Syntax

Groups of groups can be defined with the same `=Group` syntax:

- `G:==SubGroup` or `G:=SubGroup` — defines a presenter entry as a group (because `is_group` is set) without defining its members

### Dependencies

- Requires v7 presenter struct changes
- Requires FEATURE-020 (credit display logic) for credit resolution

## Acceptance Criteria

- Groups can list other groups as members
- Recursive expansion correctly identifies all individual members
- Circular group references are handled without infinite loops
- Credit resolution works correctly with nested groups
- Conflict detection considers all individual members of nested groups
