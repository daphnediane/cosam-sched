# REFACTOR-149: Finish field generalization — remaining fields, CommonData/InternalData merge, FieldDescriptor bound

## Summary

Carry the REFACTOR-148 capability-trait pattern to completion: generalize the
remaining broadly-shared fields, collapse `*CommonData` into `*InternalData`, and
investigate whether `FieldDescriptor` still needs its `EntityType` bound.

## Status

Open

## Priority

Low

## Blocked By

- REFACTOR-148: established the capability-trait pattern (`HasName` /
  `HasDescription` / `HasNotes` / `HasStartTime` / `HasDuration`), the per-field
  builder modules under `tables/fields/`, and `PanelLike` as their intersection.
  This item picks up its remaining/deferred scope.

## Description

REFACTOR-148 decoupled the shared field builders from the bundled `PanelLike`
trait and unified `name` (all seven name-bearing entities), `description` /
`notes` (uniform `AsText`), and the timing fields (`HasStartTime` /
`HasDuration`). Three threads remain, each independent and incremental.

### 1. Generalize the remaining shared fields

Extend the same "capability trait + `tables/fields/<field>.rs` builder" pattern
to other fields that recur across entity types but are still declared per-type
via `accessor_field_properties!`. Survey candidates and migrate the ones that are
genuinely the same concept (not merely the same Rust type — cf. the REFACTOR-148
call that `room_name` *is* a name but a panel's `power_needs` is not). Reduce the
per-type cost of a new shared field to: one accessor impl + one builder static.

### 2. Collapse `*CommonData` into `*InternalData`

The `*CommonData` ↔ `*InternalData` split is now vestigial; this is the work that
removes its last justification. Why the split exists today:

- **Serde reuse for export (weak):** `*CommonData` exists mostly so the export
  `*Data` struct can `#[serde(flatten)] data` for free. But every field that
  gains a typed home (`code: PanelUniqId`, `time_slot: TimeRange`, `notes:
  NoteBag`) leaves `CommonData` and forces an explicit projection line in
  `export()` — the notes already do exactly this. The flatten earns less each
  step. (`*CommonData` is only (de)serialized via that flatten; all standalone
  `serde_json::from_str::<*CommonData>` uses are test round-trips.)
- **Cheap `internal.data.clone()` (trivial):** an explicit struct build is no
  worse.
- **The real reason — the macro:** `accessor_field_properties!` generates
  `d.data.<field>`, so any field declared that way *must* live in `CommonData`.
  That's why `power_needs: Option<String>` sits in `CommonData` but `notes` /
  `code` / `time_slot` do not. The boundary no longer tracks "user-facing vs
  derived"; it tracks "has this field been given a typed accessor yet" — an
  artifact, not a design.

Target end-state: once every field is reached through an accessor (thread 1's
direction), fold `*CommonData` into `*InternalData` and make `*Data` the sole
serde surface, built explicitly in `export()` (the pattern `notes` now uses). Do
**not** merge piecemeal — a half-`.data`/half-flat struct is worse than either
end; the merge is gated on migrating a type's remaining
`accessor_field_properties!` fields off `.data` first. Break and Timeline are
nearly there (`{name, description}` / `{name, description, time}`) and are the
cheap first dominoes to validate the merged shape before touching Panel.

### 3. Investigate the `FieldDescriptor<E: EntityType>` bound

REFACTOR-148 concluded `FieldDescriptor` correctly keeps a minimal,
field-agnostic `E: EntityType` bound while the *builders* depend on the focused
capability traits. Re-examine that now that the capability-trait surface exists:

- What does `FieldDescriptor<E>` actually use `E` for? Today: `E::TYPE_NAME`
  (via `NamedField::entity_type_name`) and the storage/mirror plumbing in
  `read` / `write` / `add` / `remove` (`schedule.get_internal::<E>`,
  `get_internal_mut::<E>`, `mirror_field_value::<E>`).
- Could those be expressed through a narrower bound (e.g. a storage/identity
  capability trait) or even type-erased, decoupling the descriptor from
  `EntityType` entirely? Weigh against the `inventory::submit!` registration and
  the `FieldSet<E>` machinery that are generic over `E`.
- Likely outcome is "keep `EntityType` — it *is* the storage/identity
  capability"; the deliverable is a documented decision (in
  `field/descriptor.rs`) either way, not necessarily a code change.

## Acceptance Criteria

- [ ] A representative additional shared field is migrated to a `tables/fields/`
      builder + capability trait (or a documented finding that no further field
      is genuinely shared).
- [ ] At least one entity type (Break or Timeline first) has its `*CommonData`
      folded into `*InternalData`, with `*Data` built explicitly in `export()`;
      exports remain byte-identical.
- [ ] A documented decision on the `FieldDescriptor` `EntityType` bound (kept or
      narrowed), recorded in `field/descriptor.rs`.
- [ ] `cargo clippy --workspace --all-targets` clean; `cargo test --workspace`
      green; widget JSON / XLSX exports byte-identical.

## Notes

- Deliberately broad/low-priority; land incrementally (one entity's merge, one
  field, the bound investigation — each its own commit).
- Watch interactions with FEATURE-146 (Uniq ID / type rework) and the
  `accessor_field_properties!` path, which hardcodes `d.data.<field>` and is the
  blocker for the `*CommonData` merge.
