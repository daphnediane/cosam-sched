# REFACTOR-147: Shared "panel-like" abstraction for Panel/Break/Timeline

## Summary

Share field definitions and lookup across Panel, Break, and Timeline via a
`PanelLike` trait and generic `const fn` field builders, keeping the three
entity types distinct.

## Status

Completed

## Priority

Medium

## Description

Panel, Break, and Timeline shared most of their shape (a parsed Uniq ID `code`,
`name` / `description` / `note`, a panel-type edge, and timing) but each defined
its own copy of those `FieldDescriptor` statics, and the widget-JSON/XLSX
consumers duplicated the per-kind logic. This made adding or changing a common
field a three-place edit.

The three entity types are intentionally kept separate — their distinct
`EntityId<E>` types and `InternalData` `TypeId`s are load-bearing for storage and
`Schedule::identify`. Instead the *differences are made virtual*: a `PanelLike`
trait exposes the shared fields by redirecting into whatever common-data each
type already stores, so one field can be defined once and used by every kind.

## Implementation Details

- New `tables/panel_like.rs`:
  - `EventKind` — the internal "mode" (Panel / Break / Timeline), exposed as
    `PanelLike::KIND`.
  - `PanelLike` trait — field-level accessors (`name`/`description`/`note`/`code`
    + `_mut`) that each type implements by pointing into its own `*CommonData`.
    No shared storage struct is imposed.
  - `PanelLikeTimed: PanelLike` — exposes timing as a **virtual** `TimeRange`
    via `time_slot()` (by value) / `set_time_slot()`. Break/Panel hold a real
    `TimeRange`; Timeline stores a single `Option<NaiveDateTime>` and
    wraps/unwraps it, so no type is forced into a representation it does not want.
  - Generic `const fn` field builders (`code_field`, `name_field`,
    `description_field<M>`, `note_field<M>`, `time_field`, `start_time_field`,
    `end_time_field`, `duration_field`). Each returns a `FieldDescriptor<E>`; the
    text marker `M` lets Break/Timeline use `AsString` while Panel uses `AsText`.
    Entity modules instantiate them as per-type statics (own `order`/`aliases`)
    and register them through the existing `inventory::submit!` — no macro, and
    `.data` access is preserved.
  - `PanelLikeRef<'a>` + `Schedule::iter_panel_like` /
    `find_panel_like_by_code` — uniform, kind-agnostic iteration and a single
    Uniq-ID → panel-like-object lookup.
- Each entity (`breaks.rs`, `timeline.rs`, `panel/mod.rs`) implements the traits
  and replaces its hand-written shared-field statics with the builders. Each
  keeps its own `*CommonData` struct.
- `widget_json/export.rs`: `panelType` is now derived from the Uniq ID prefix
  (`code.type_prefix()`) instead of traversing the panel-type edge; the three
  `get_*_type_prefix` helpers were removed. Verified byte-identical output and
  forward-aligned with FEATURE-146 (prefix-authoritative type).
- `value/uniq_id.rs`: `type_prefix()` returns up to **3** characters when the
  prefix starts with `%` (sentinel codes like `%IB`/`%NB`), else 2.

## Acceptance Criteria

- [x] `cargo clippy --workspace --all-targets` clean
- [x] `cargo test --workspace` green (798 passing)
- [x] Widget public-JSON export byte-identical to pre-refactor baseline
- [x] Canonical XLSX round-trip byte-identical

## Notes

- Future: allow adding more common fields without per-type `CommonData` edits.
- Complementary to FEATURE-146, which will rework how panel types are linked and
  make the Uniq ID prefix authoritative; the prefix derivation here anticipates
  that. Land FEATURE-146's linking change before/with this branch.
