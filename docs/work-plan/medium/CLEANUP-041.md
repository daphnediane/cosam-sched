# Unify JSON field case conventions

## Summary

Standardize case conventions

## Status

Open

## Priority

Medium

## Description

The V9 format introduced inconsistent case conventions by changing presenters from snake_case to camelCase while keeping other fields the same. This cleanup will:

1. **Establish clear convention rules** based on data consumer:
   * JavaScript-consumed data: camelCase (panels, panelTypes)
   * Rust-internal data: snake_case (presenters, rooms, timeline)

2. **Update V9 documentation** to reflect correct case conventions

3. **Verify all JSON output** follows the established patterns

4. **Add convention documentation** to prevent future inconsistencies

## Current State

* V7: Mixed conventions (panels=camelCase, presenters=snake_case, rooms=snake_case)
* V9: Inconsistent (presenters accidentally changed to camelCase)
* JavaScript widget expects V7 conventions

## Implementation Details

* [ ] Revert presenter fields to snake_case (is_group, always_grouped, panel_ids)
* [ ] Update V9 documentation to specify case conventions
* [ ] Add case convention rules to JSON format documentation
* [ ] Verify all Display* structs use correct serde rename_all attributes
* [ ] Test widget with both V7 and V9 JSON outputs

## Acceptance Criteria

* All V9 JSON fields follow established V7 case patterns
* Documentation clearly specifies which fields use which case
* JavaScript widget works with both V7 and V9 formats
* No breaking changes for existing V7 consumers

## Notes

The mixed convention approach is actually optimal since it matches the consumer language (JavaScript vs Rust) while maintaining backward compatibility.
