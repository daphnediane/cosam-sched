# REFACTOR-148: Generalize common fields across all entity types

## Summary

Let common fields (name, description, notes, etc.) be defined once and reused by
any entity type, without per-type `CommonData` edits — extending beyond the
panel-like trio.

## Status

In Progress

### Progress

Partial landing (panel-like only):

- New `crates/schedule-core/src/tables/fields/` module — field *definitions*,
  one file per field (`code`, `name`, `description`, `note`, `time`). This is to
  `crate::field` (the field *system*) what `tables` is to `entity`: concrete
  instances on top of the infrastructure.
- The shared `const fn` builders moved out of `tables/panel_like.rs` into those
  files; `panel_like.rs` now holds only `EventKind`, the `PanelLike` /
  `PanelLikeTimed` traits, and the `PanelLikeRef` view + lookup.
- Note kinds modelled as `NoteKind` (Public / NonPrinting / Workshop / Av) with
  a `NoteBag(HashMap<NoteKind, String>)` stored on each `*InternalData`. One
  `note_field<E, K: NoteSlot, M>` builder serves every kind (the kind is carried
  by a zero-size `NoteSlot` marker, since the read/write callbacks are
  non-capturing fn pointers). Panel's four note fields and Break/Timeline's
  single note now all live in the bag; each type declares
  `PanelLike::SUPPORTED_NOTES`.
- Export shapes preserved: widget JSON, `PanelData`/`BreakData`/`TimelineData`,
  and XLSX still expose the discrete `note` / `notesNonPrinting` /
  `workshopNotes` / `avNotes` keys, projected from the bag. CRDT field names are
  now bag-derived (acceptable — CRDT files are regenerated).

Still open: generalizing beyond the panel-like trio (presenter, event_room,
hotel_room, panel_type) via broader marker traits (`HasName`, …).

## Priority

Low

## Blocked By

- REFACTOR-147: established the per-field generic `const fn` builder pattern and
  the `PanelLike` accessor approach this generalizes from.

## Description

REFACTOR-147 made Panel/Break/Timeline share field *logic* via the `PanelLike`
trait + generic `const fn` field builders. But adding a new shared field still
requires, per type: a trait accessor method and a field on each `*CommonData`
struct. And the sharing stops at the three panel-like types — yet many fields are
common much more broadly:

- `name` — panel, presenter, event_room (room_name), hotel_room, panel_type
  (panel_kind), timeline, break
- `description` — panel, timeline, break (and plausibly others)
- assorted notes — panel has `note`, `notes_non_printing`, `workshop_notes`,
  `av_notes`; the "internal vs printing" note distinction recurs elsewhere

The goal: make a common field definable once and attachable to any entity type
that opts in, with near-zero per-type boilerplate, so the set of common fields
can grow freely.

## Implementation Details

Sketch / options to evaluate — not prescriptive:

- A small `HasCommon`-style trait (or a set of focused marker traits like
  `HasName`, `HasDescription`, `HasNotes`) with default-method accessors, so a
  type opts in by implementing only the accessor for the storage it already has.
- Consider a shared optional-field bag vs. keeping fields in each `*CommonData`
  — weigh against serde shape stability (exports must stay byte-identical) and
  the CRDT field-name contract.
- Field builders generic over the accessor trait (as in REFACTOR-147's
  `name_field`/`description_field<M>`) so one definition serves every opted-in
  entity type; keep `inventory::submit!` registration and `.data` access.
- Reduce the per-type cost of a new common field to: implement one accessor
  (often a one-liner) + instantiate the shared builder static.

### Note kinds

Panel already carries several notes that are *structurally identical* but stored
in separate fields: `note`, `notes_non_printing`, `workshop_notes`, `av_notes`
(and the pattern recurs elsewhere). Rather than a bespoke field per note, model
them as one note concept parameterized by a **note-kind discriminant**:

- A `NoteKind` enum (e.g. `Public`, `NonPrinting`, `Workshop`, `Av`, …), with
  each variant carrying its canonical field name / aliases / display / printing
  semantics (does it appear in attendee-facing exports?).
- A generic `note_field<E, …>(kind)` builder that reads/writes the storage for
  that kind through an accessor, so all note kinds share one definition.
- Let each entity type declare *which* note kinds it supports — e.g. an
  associated `const SUPPORTED_NOTES: &[NoteKind]` (a per-entity-type set), so
  Panel can have the full set while Break/Timeline have just `Public`. This is
  the "per-entity-type note-types enum" idea: the kinds are shared, the
  supported subset is per type.
- Storage options to weigh: keep the discrete fields (accessor maps kind →
  field) for serde/CRDT stability, or move to a `HashMap<NoteKind, String>`-style
  bag. Either way the export shape and CRDT field names must stay identical.

## Acceptance Criteria

- [ ] A new common field can be added and attached to multiple entity types
      without editing each type's `CommonData` definition beyond a trivial
      accessor.
- [ ] At least the `name` field is unified across the entity types that have it
      (panel, presenter, event_room, hotel_room, panel_type, timeline, break).
- [ ] Note kinds (public / non-printing / workshop / AV / …) share one field
      definition, with the supported subset declared per entity type.
- [ ] Exports (widget JSON, XLSX) remain byte-identical; CRDT field names
      unchanged.
- [ ] `cargo clippy --workspace --all-targets` clean; `cargo test --workspace`
      green.

## Notes

- Scope is deliberately broad/low-priority — likely worth landing incrementally
  (e.g. `name` first, then notes) rather than one big change.
- Watch interactions with FEATURE-146 (Uniq ID / type rework) and the existing
  `accessor_field_properties!` path, which hardcodes `d.data.<field>`.

### Follow-up: collapse `*CommonData` into `*InternalData`

The `*CommonData` ↔ `*InternalData` split is becoming vestigial and this refactor
is what removes its last justification. Survey of why the split exists today:

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

Target end-state: once every field is reached through an accessor (the direction
of this work item), fold `*CommonData` into `*InternalData` and make `*Data` the
sole serde surface, built explicitly in `export()` (the pattern `notes` now
uses). Do **not** merge piecemeal — a half-`.data`/half-flat struct is worse than
either end; the merge is gated on migrating a type's remaining
`accessor_field_properties!` fields off `.data` first. Break and Timeline are
nearly there (`{name, description}` / `{name, description, time}`) and are the
cheap first dominoes to validate the merged shape before touching Panel.
