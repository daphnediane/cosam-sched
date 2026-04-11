# Implement Schedule method delegation to entity types

## Summary

Implement proper delegation pattern for `Schedule` convenience methods,
moving business logic to entity-specific implementations.

## Status

Open

## Priority

Medium

## Description

The `Schedule` struct currently contains several methods that implement
business logic directly instead of delegating to entity types. This was
written against the planned architecture and needs to be corrected.

### Methods needing delegation

1. **Presenter-group membership helpers** (lines ~442-602 in `schedule/mod.rs`)
   - `mark_presenter_group`
   - `unmark_presenter_group`
   - `add_member` / `add_grouped_member` / `add_shown_member`
   - `remove_member`

2. **Edge mutation methods** (lines ~159-386 in `schedule/mod.rs`)
   - `add_edge`, `add_edge_with_policy`
   - `remove_edge`
   - Edge query methods

### Target architecture

- `Schedule` provides the top-level convenience API
- The actual logic lives in the entity type implementations
  (`PresenterToGroupEntityType`, `PanelToPresenterEntityType`, etc.)
- This mirrors the pattern established by `PanelEntityType::add_presenters`,
  `PanelToPresenterEntityType::add_presenters`, etc.

## Acceptance Criteria

- [ ] Presenter-group membership methods delegate to `PresenterToGroupEntityType`
- [ ] Edge mutation methods have corresponding entity type methods
- [ ] All methods follow the pattern: `Schedule` -> `XxxEntityType` -> `XxxToYyyEntityType`
- [ ] No logic duplication between Schedule and entity types

## Dependencies

- [FEATURE-009] Query System (for the established delegation pattern)

## Related

- [FEATURE-008] Schedule container (original implementation)

## Blocks

- [META-026] Phase 2 - Core Data Model
