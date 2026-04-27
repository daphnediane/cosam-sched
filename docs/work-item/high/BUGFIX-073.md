# BUGFIX-073: Panel `time_slot` is silently dropped on save/load

## Summary

`PanelInternalData::time_slot` has no CRDT backing field, so panel start /
end / duration are not mirrored to the Automerge document and are lost
through any save → load (or merge) round trip.

## Status

Open

## Priority

High

## Description

`PanelInternalData` carries the temporal state of a panel in a single
`time_slot: TimeRange` field (`@../../../crates/schedule-core/src/panel.rs:88-93`).
The field system exposes three projections onto that struct:

- `FIELD_START_TIME`
- `FIELD_END_TIME`
- `FIELD_DURATION`

All three are declared `crdt: Derived` (panel.rs lines around 542, 584,
626). There is no `FIELD_TIME_SLOT`, so `time_slot` itself is never
seen by the field-set / CRDT plumbing.

The write path mutates the in-memory cache and then deliberately skips
the Automerge mirror for `Derived` fields:

```text
crates/schedule-core/src/field.rs:289-310
if !schedule.mirror_enabled()
    || matches!(self.crdt_type,
        CrdtFieldType::Derived | CrdtFieldType::EdgeOwner { .. } |
        CrdtFieldType::EdgeTarget) {
    return Ok(());
}
```

`crdt::put_field` and the rehydrate path do the same:

```text
crates/schedule-core/src/crdt.rs:30-37     # "| Derived | not stored |"
crates/schedule-core/src/crdt.rs:469-473   # rehydrate skips Derived
```

Net effect:

- `with_start_time(…)` updates `d.time_slot` only in the cache; nothing
  is written to the Automerge document.
- On load, `rehydrate_entity` walks `field_set.fields()` and skips every
  `Derived` descriptor, so the rehydrated `PanelInternalData` falls back
  to the builder's `TimeRange::default()` → `TimeRange::Unspecified`.
- A merge of two replicas similarly carries no temporal information.

## How Found

Noticed during the FEATURE-071 proc-macro migration of `panel.rs`: while
rewriting the time projections, the question "how is `time_slot` stored
in the CRDT given that all three projections are `Derived`?" surfaced
the gap.

## Reproduction

1. Build a panel with a non-`Unspecified` time slot, e.g.
   `TimeRange::ScheduledWithDuration { start_time, duration }`.
2. Insert it into a `Schedule` and call `Schedule::save`.
3. `Schedule::load` the resulting bytes.

**Expected:** the loaded panel has the original `start_time` and
`duration`.

**Actual:** the loaded panel's `time_slot` is `TimeRange::Unspecified`;
start / end / duration all read back as `None`.

The existing tests don't catch this because `make_panel()` in
`@../../../crates/schedule-core/src/schedule.rs:1260-1272`
uses `TimeRange::Unspecified`, and `Unspecified → Unspecified` is a
no-op round trip.

## Steps to Fix

Preferred: promote the three projections from `Derived` to `Scalar`.

- Change `crdt: Derived` → `crdt: Scalar` in `FIELD_START_TIME`,
  `FIELD_END_TIME`, `FIELD_DURATION` (panel.rs).
- Drop the now-redundant `verify: ReRead` on those fields (the `Scalar`
  mirror already round-trips the value through `read_fn`).
- Confirm `crdt::put_field` / `get_field` already handle
  `Optional<DateTime>` and `Optional<Duration>` Scalars (they do today
  for `event_room`'s optional string / etc.).

This keeps `TimeRange` as the in-memory representation and lets the
three field writes naturally repopulate it on rehydrate. The existing
write closures already accept `DateTime`, `String`, `Duration`,
`Integer` etc. and project onto `time_slot`, so no closure change is
needed.

Alternative (heavier): introduce a real stored `FIELD_TIME_SLOT` whose
value is a serialized `TimeRange`. Rejected for now — `TimeRange` is an
enum with three variants, none of which fit the existing
`FieldTypeItem` palette without a new variant or custom serializer.

## Testing

- New regression test in `panel.rs` (or `schedule.rs` round-trip
  module): construct a panel with
  `TimeRange::ScheduledWithDuration { start, duration }`, save, load,
  assert `time_slot` matches.
- Tighten `make_panel()` (or add a `make_scheduled_panel()`) so future
  round-trip tests exercise non-`Unspecified` time slots by default.
- Verify `cargo test -p schedule-core --lib` still passes; the existing
  `save_load_*` tests should be unaffected.

