# REFACTOR-148: Generalize common fields across all entity types

## Summary

Let common fields (name, description, notes, etc.) be defined once and reused by
any entity type, without per-type `CommonData` edits — extending beyond the
panel-like trio.

## Status

Completed

### Progress

Capability-trait landing (generalizes past the panel-like trio):

- The shared accessors split out of `PanelLike` into focused, entity-agnostic
  capability traits, each co-located with the builder that consumes it:
  `HasName` (in `fields/name.rs`), `HasDescription` (+ associated
  `DescriptionMapping`, in `fields/description.rs`), `HasNotes` (+ associated
  `NoteMapping` + `SUPPORTED_NOTES`, in `fields/note.rs`). Each `: EntityType`.
- The field builders are re-based onto the narrow trait they actually need:
  `name_field<E: HasName>`, `description_field<E: HasDescription>`,
  `note_field<E: HasNotes, K>`. Call sites drop the per-site `::<…, AsText>`
  value-flavour marker turbofish entirely (see the AsText note below for why the
  flavour ended up hardcoded rather than a per-entity knob).
- `PanelLike` is now the *intersection* `HasName + HasDescription + HasNotes`
  plus what is genuinely panel-like-only (`KIND` + the `PanelUniqId` `code`
  accessors). The trio (`Panel`/`Break`/`Timeline`) split their single
  `impl PanelLike` into the three capability impls + a slim `PanelLike` impl —
  behaviour unchanged.
- **Proof the decoupling generalizes:** `presenter`'s `name` now reuses the one
  shared `name_field` (via `name_field_described`, preserving its exact
  aliases/display/description/example), opting in with a two-line `HasName` impl
  and no panel-like machinery. CRDT type (`Scalar`) and the canonical `name`
  field are identical, so exports stay byte-identical.

Answers the open design question (`FieldDescriptor` keeps its minimal,
field-agnostic `E: EntityType` bound — it's only the *builders* that should
depend on capability traits, not the bundled `PanelLike`).

The shared definition also surfaced a latent inconsistency: Break/Timeline
stored `description` *and* `note` as `AsString` (scalar) while Panel used
`AsText`, for no principled reason — all are long prose. Unified everything to
`AsText` (CRDT text). Once neither field diverged, the per-entity value-flavour
knob was dead weight, so it was removed: `description_field` / `note_field`
hardcode `AsText`, `name_field` hardcodes `AsString` (a name is a scalar). The
capability traits are now pure accessors (plus `SUPPORTED_NOTES`). JSON/XLSX
exports are unchanged (these serialize as plain strings); CRDT description / note
fields regenerate as text.

Earlier partial landing (panel-like only):

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

Name unification (done for every string-name entity): panel, break, timeline,
presenter, **event_room**, **hotel_room**, and **panel_type** all reuse the
shared `name_field` via a two-line `HasName` impl. Each keeps its UI metadata
(display "Room Name" / "Hotel Room Name" / "Panel Kind", descriptions) but
adopts the **canonical key `name`**, demoting its legacy key (`room_name`,
`hotel_room_name`, `panel_kind`) to an *alias* so existing key-based lookups
(and XLSX column resolution) keep resolving. Widget JSON / XLSX output is
unaffected — those read the serde struct fields (`data.room_name`, …), not the
field-system canonical name; only CRDT field keys change (regenerated).
Required-field assertions updated to `name`.

`panel_type`'s `panel_kind` *is* the panel type's name (the "Panel Kind" sheet
column; `EntityStringResolver` already returned it), so it became `name`. Its
unused computed `FIELD_DISPLAY_NAME` ("Kind (Prefix)") — which had carried the
colliding `name` alias and had no consumer (widget exports `panel_kind`
directly) — was removed along with its tests.

Timing capability split (done): `PanelLikeTimed` is gone, replaced by two
capability traits + builder modules mirroring the name/description/note split:

- `tables/fields/time.rs` — [`HasStartTime`], the *start instant* only.
  Accessor is `Option<NaiveDateTime>` (what Timeline stores natively). Every
  panel-like entity has a start, so `PanelLike: … + HasStartTime`; all three
  opt in. Builds `time` (Timeline) and `start_time` (Panel/Break).
- `tables/fields/duration.rs` — [`HasDuration`], the `end_time` and `duration`
  fields. Accessor is the full `TimeRange` (read–modify–write). Only the
  duration-carrying kinds (Panel, Break) opt in; Timeline (a single instant)
  does *not* implement it, so it no longer synthesises a start-only `TimeRange`
  nor carries duration-only fields. `PanelLikeRef::time_slot` keeps presenting a
  uniform `TimeRange` (Timeline as start-only) for the unified view.

Remaining scope split out to **REFACTOR-149**: generalizing any other
genuinely-shared fields, the `*CommonData` → `*InternalData` collapse, and the
investigation of whether `FieldDescriptor` still needs its `EntityType` bound.

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

- [x] A new common field can be added and attached to multiple entity types
      without editing each type's `CommonData` definition beyond a trivial
      accessor. (Proven: `name` reused across seven entities via a two-line
      `HasName` impl + one builder static.)
- [x] At least the `name` field is unified across the entity types that have it
      (panel, presenter, event_room, hotel_room, panel_type, timeline, break).
- [x] Note kinds (public / non-printing / workshop / AV / …) share one field
      definition, with the supported subset declared per entity type.
      (`NoteBag` + `note_field<E: HasNotes, K>` + `SUPPORTED_NOTES`.)
- [x] Exports (widget JSON, XLSX) remain byte-identical. CRDT field names: the
      `name`-unification (room_name/hotel_room_name/panel_kind → canonical `name`)
      and the description/notes `AsString → AsText` change *do* alter CRDT field
      keys/types — accepted by design, as CRDT files regenerate (the original
      "CRDT field names unchanged" goal was relaxed deliberately, per the choices
      recorded in Progress above).
- [x] `cargo clippy --workspace --all-targets` clean; `cargo test --workspace`
      green.

## Notes

- Scope is deliberately broad/low-priority — likely worth landing incrementally
  (e.g. `name` first, then notes) rather than one big change.
- Watch interactions with FEATURE-146 (Uniq ID / type rework) and the existing
  `accessor_field_properties!` path, which hardcodes `d.data.<field>`.

### Follow-up: collapse `*CommonData` into `*InternalData`

Moved to **REFACTOR-149** (thread 2), along with the survey of why the split
exists today and the gated merge plan (Break/Timeline first). The capability-trait
work here removes the split's last justification, but the merge itself is
deferred to 149.
