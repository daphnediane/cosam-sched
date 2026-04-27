# FEATURE-071: Introduce schedule-macro proc-macro crate

## Summary

Replace the declarative `macro_rules!` field-declaration helpers (`stored_field!`,
`edge_field!`, `define_field!`) with attribute-style proc-macros in a new
`schedule-macro` crate; add an `exclusive_with:` clause to express
cross-partition edge exclusivity declaratively.

## Status

Completed

## Resolution

- New `crates/schedule-macro/` crate added with a function-like
  `define_field!` proc-macro covering stored, edge, and custom modes
  (read / write / verify closures supported, `exclusive_with:` clause
  implemented).
- All entity files migrated: `panel_type.rs`, `hotel_room.rs`,
  `event_room.rs`, `presenter.rs`, `panel.rs`.
- Legacy `stored_field!`, `edge_field!`, and old `macro_rules!
  define_field!` removed from `crates/schedule-core/src/field_macros.rs`;
  only `define_entity_builder!` remains in that file.
- `cargo build`, `cargo test --workspace`, and
  `cargo clippy --workspace --all-targets -- -D warnings` all clean.

Follow-ups tracked separately:

- **BUGFIX-072** — FIELD_MEMBERS / FIELD_GROUPS near/far audit and
  alias proposal surfaced during the presenter migration.
- **BUGFIX-073** — Panel `time_slot` is silently dropped on save/load
  (start/end/duration are `Derived` with no stored backing field);
  surfaced while migrating the `panel.rs` time projections.

## Priority

High

## Description

The current declarative `macro_rules!` helpers in
`crates/schedule-core/src/field_macros.rs` cover 90% of field declarations
cleanly, but they have two pain points worth resolving before the next round
of edge-field work:

1. **Closures can't be passed as macro arguments cleanly.**  Most non-trivial
   fields with custom read/write logic must drop down to `define_field!` and
   inline a `WriteFn::Schedule(|sched, id, val| …)` block by hand, even when
   the field is otherwise edge-shaped.  This is verbose and easy to get wrong.

2. **Cross-partition edge exclusivity has no declarative form.**  FEATURE-065's
   credited/uncredited presenter split needs each partition's write closure to
   first remove the same target from the sibling partition.  Today this can
   only be expressed by hand-writing four `define_field!` blocks; with
   proc-macros we can add an `exclusive_with: &SIBLING_FIELD` clause that
   generates the prelude.

Note: a previous proc-macro experiment (`v10-try3/crates/schedule-macro`,
~1600 lines) handled both field declarations *and* type conversion logic.
Conversion has since been factored out into a separate system (see
`docs/conversion-and-lookup.md`), so the new proc-macros should be
significantly smaller — they only need to emit `FieldDescriptor` + closures.

After FEATURE-070, `EdgeDescriptor` is gone; the proc-macros only need to set
`crdt_type: EdgeOwner { target_field: <target_field> }` directly when the
`owner` flag is present, with no separate descriptor static or inventory
submission to coordinate.

## Implementation Details

### Crate layout

- New `crates/schedule-macro/` with `[lib] proc-macro = true`.
- BSD-2-Clause header on every source file; `license = "BSD-2-Clause"` and
  `authors = ["Daphne Pfister"]` in `Cargo.toml`.
- Workspace member.

### Macro form

Function-like proc-macro (not attribute), matching today's call style.  A
single unified `define_field!` macro that branches on parameter shape — most
of what's shared between today's `stored_field!`, `edge_field!`, and
`define_field!` macro_rules can be expressed as one macro with optional
parameter groups.

**Stored field** (auto-derives crdt, read, and write from the accessor):

```rust
define_field! {
    static FIELD_NAME: FieldDescriptor<PanelEntityType>,
    accessor: name, required,
    name: "name", display: "Name",
    desc: "Display name of the panel.",
    aliases: &["title"],
    example: "\"My Panel\"",
    order: 0,
}
```

**Edge field** (auto-derives crdt, read, and write from `edge:` mode + `owner`):

```rust
define_field! {
    static FIELD_CREDITED_PRESENTERS: FieldDescriptor<PanelEntityType>,
    edge: rw, target: PresenterEntityType,
    target_field: &crate::presenter::FIELD_PANELS,
    owner,
    exclusive_with: &FIELD_UNCREDITED_PRESENTERS,
    name: "credited_presenters", display: "Credited Presenters",
    desc: "Presenters credited on this panel.",
    aliases: &["credited_panelists"],
    example: "[]",
    order: 2710,
}
```

**Custom / computed field** (closures accepted as first-class
`syn::ExprClosure` arguments instead of hand-inlined `ReadFn::Schedule(...)`):

```rust
define_field! {
    static FIELD_INCLUSIVE_PRESENTERS: FieldDescriptor<PanelEntityType>,
    cardinality: list, item: entity(PresenterEntityType),
    crdt: Derived,
    read: |sched, id| {
        // closure body, parsed as ExprClosure, stitched into ReadFn::Schedule
    },
    name: "inclusive_presenters", display: "Inclusive Presenters",
    desc: "...",
    aliases: &[],
    example: "[]",
    order: 3000,
}
```

Branch detection:

- `accessor:` present → stored mode (auto crdt, auto read/write from accessor)
- `edge:` present → edge mode (auto crdt from `owner` flag, auto read/write
  from `edge: ro|rw|one|add|remove`)
- Neither → custom mode; require explicit `crdt:`, `cardinality:`, `item:`,
  and at least one of `read:` / `write:`.

`read:` / `write:` / `verify:` closures may also override the auto-generated
ones in stored or edge modes.

### `exclusive_with` semantics

When present on `edge: rw` or `edge: add`, the generated write closure
prepends a per-id `edge_remove` against the sibling field for every entity
in the new value.  Symmetric: declared on both sides (half-edge model).

### Re-export

Re-export `define_field!` from `schedule-core` so consumers continue to
`use schedule_core::define_field;`.  The `schedule-macro` crate is an
implementation detail.

### Migration

Migrate entity files one at a time (behavior-preserving):

- `panel.rs`, `presenter.rs`, `event_room.rs`, `hotel_room.rs`, `panel_type.rs`.

Delete the old `field_macros.rs` `macro_rules!` after the last entity is
migrated (or replace with thin shims that forward to the proc-macros for the
duration of FEATURE-065's rewrite, then delete).

## Acceptance Criteria

- All entity files use the new `#[…]` attribute proc-macros.
- `cargo test --workspace` passes with no behavior changes.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo fmt` clean.
- The new `schedule-macro` crate is meaningfully smaller than v10-try3's
  ~1600-line implementation, since conversion logic stays in
  `conversion-and-lookup`.
- `exclusive_with:` is exercised by at least one test fixture.

## Notes

- Half-edge model preserved: each field declares its own direction and any
  exclusivity partners; no auto-pairing of inverse fields.
- `mode = remove` over multiple sibling lists (the `FIELD_REMOVE_PRESENTERS`
  case) stays hand-written for now; revisit if more cases emerge.
- Field-level proc-macro attributes (one `pub static` per field) intentionally
  keep declarations explicit, addressing the original objection to v10-try3's
  struct-level `#[derive(EntityFields)]` ("hid the data").
