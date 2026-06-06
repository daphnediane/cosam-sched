# REFACTOR-140: Tiered presenter rank + consistent presenter creation

## Summary

Introduce a `RankSource` tier model for presenter rank, route all presenter
creation through the single tagged API with deterministic v5 UUIDs, and clean up
`presenter.rs` visibility and People-sheet membership helpers.

## Status

Completed

## Priority

Medium

## Description

Parsing the People/Presenters sheet produced inconsistent presenter entities:
the People-sheet "new entity" path created entities with deterministic v5 UUIDs
(`build_entity` + `PreferFromV5`), while the tagged path
(`find_or_create_presenter_by_name`) used random `EntityId::generate()` UUIDs.
The same presenter created via different paths got different UUIDs, so
re-importing or merging the same sheet produced duplicate entities and unstable
merges.

Rank handling was also split across two overlapping mechanisms (immediate
promotion in `find_or_create_presenter_by_name` plus the `Option`-based
`PresenterImportCache`), making the precedence rules hard to reason about.

## Implementation Details

- Add a `RankSource` enum carrying both tier and rank:
  - `None` (tier 0 → effective `Panelist`), `Implied(PresenterRank)` (tier 1),
    `Declared(PresenterRank)` (tier 2).
  - Precedence: `Declared > Implied > None`; within a tier, normal rank
    promotion (min `priority()`).
  - CRDT field encoding round-trips the tier (`""` / `~rank` / `rank`); JSON
    export keeps a single effective `rank` string.
- Tier assignment: Classification column and schedule Named-column headers and
  the named token of a tag prefix (`G:member`) are `Declared`; the `=group`
  reference in `G:member=group` is `Implied` for the group; untagged `Other`
  cells and inherited `Members`/`Groups` entries are `Implied`.
- Route `find_or_create_presenter_by_name` (and therefore the single public
  `find_or_create_tagged_presenter` API) through
  `build_entity(PreferFromV5 { name })` so every presenter gets a deterministic
  v5 UUID. People-sheet reader stops calling `build_entity` directly.
- Replace `apply_membership` / `apply_group_membership` / `build_entity_tag` /
  `build_membership_tag` with the structured tagged API plus a small local
  seen/sort-key recorder in the reader.
- Tighten `presenter.rs` visibility: keep `find_or_create_tagged_presenter`,
  `find_tagged_presenter`, `MatchedTagPresenter`, `RankSource`, `PresenterRank`,
  field descriptors and the builder public; make `parse_tag`, `ParsedTag`,
  `is_group_entity`, `find_group_by_name`, `find_or_create_presenter_by_name`
  private.

## Acceptance Criteria

- `cargo test -p schedule-core` is green.
- Importing the same XLSX twice yields identical presenter UUIDs and ranks.
- No caller outside `presenter.rs` reaches into presenter internals to set
  group/rank flags.
