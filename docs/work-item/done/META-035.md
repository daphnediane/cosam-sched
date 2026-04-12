# Virtual Edge Refactor — Design Documentation and Work Item Setup

## Summary

Update system documentation to describe the virtual edge design and create
work items for implementation phases REFACTOR-036, REFACTOR-037, REFACTOR-038.

## Status

Completed

## Priority

High

## Work Items

- REFACTOR-036: Entity field changes (Panel, EventRoom, Presenter)
- REFACTOR-037: EntityStorage reverse indexes and hook system
- REFACTOR-038: Schedule methods, macro cleanup, edge file deletion

## Description

Meta work item for the virtual edge refactor. This work item tracks the overall
refactor effort but contains no code changes itself. The actual implementation
is split into three child work items:

- REFACTOR-036: Add stored relationship fields to entities
- REFACTOR-037: Implement reverse indexes and hook system
- REFACTOR-038: Update Schedule methods and delete edge infrastructure

This meta work item will be marked Done when all three child work items are complete.

### Work completed

- `docs/system-analysis.md` updated:
  - §4.2: edge-entities table replaced with virtual edge ownership table
  - §4.3: key design decisions revised (virtual edges, reverse indexes, hooks)
  - §5: `UuidPreference::Edge` variant removed
  - §6: `DirectedEdge` row removed from macro generated items table
  - §10: EntityStorage layout revised; `EntityType` hook methods added;
    `TypedEdgeStorage`/`EdgeEntityType` sections removed; convenience methods
    table updated; membership mutation helpers updated
  - §11: `AddEdge`/`RemoveEdge` edit commands removed; replaced with note that
    relationship changes are field mutations
  - §12: `PresenterToGroup` edge and self-loop concept replaced with
    `is_explicit_group` field and `Vec<PresenterId>` groups; credit display rules
    updated for entity-level flags
  - §14: CRDT note updated for virtual edge conflict model
  - §15: UUID generation note updated
  - §16: edge-entity references updated
- `FEATURE-033` marked Superseded (subsumed by REFACTOR-037)
- `REFACTOR-036`, `REFACTOR-037`, `REFACTOR-038` work items created
- `IDEA-039` created for deferred per-membership flags
