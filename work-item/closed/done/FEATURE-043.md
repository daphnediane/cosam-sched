# FEATURE-043: Cross-Field Verification Callback (verify_fn)

## Summary

Add a `verify` callback to `FieldDescriptor` for cross-field consistency checks after batch writes to computed fields.

## Status

Completed

## Related

- FEATURE-046: Bulk Field Updates (uses verify callbacks during batch writes)

## Priority

High

## Description

The field system currently has `validate` for single-field value validation, but lacks a mechanism for **cross-field verification** after batch updates to interdependent computed fields.

### The Problem

Consider `time_slot` with three computed fields (`start_time`, `end_time`, `duration`) backed by a single `TimeRange`. When updating all three in one batch:

```rust
// Batch update: user sets all three fields
schedule.update_fields(panel_id, &[
    ("start_time", FieldValue::DateTime(start)),
    ("end_time", FieldValue::DateTime(end)),
    (duration", FieldValue::Integer(90)),
])?;
```

The writes apply sequentially, each potentially modifying `TimeRange`. After all writes complete, we need to verify that **fields that were written still have the values we intended** — e.g., if we set `duration` to 90 minutes, we verify it's still 90 minutes (not changed because `end_time` was also set).

### Distinction: validate vs verify

| Callback   | When Called                          | Purpose                                | Example                                            |
| ---------- | ------------------------------------ | -------------------------------------- | -------------------------------------------------- |
| `validate` | Before writing a single field        | Check if the value itself is valid     | Density must be > 0                                |
| `verify`   | After batch write of multiple fields | Check field has the value that was set | `duration` is still 90 after `end_time` changed it |

### Design Goals

- Data can be in temporarily invalid states (intentional design choice)
- Edges are typed to point to correct entity types
- Most text fields are free-form (no validation needed)
- No required fields — a panel without code or name is just soft-deleted/unscheduled
- Verification catches programmer/errors in batch update logic, not user input errors

### Implementation

Add to `FieldDescriptor<E>`:

```rust
pub verify_fn: Option<VerifyFn<E>>,
```

Where `VerifyFn<E>` is:

```rust
pub enum VerifyFn<E: EntityType> {
    /// Data-only verification — no schedule access needed.
    Bare(fn(&E::InternalData, &FieldValue) -> Result<(), VerificationError>),
    /// Schedule-aware verification — fn receives `(&Schedule, EntityId<E>, &FieldValue)`.
    Schedule(fn(&Schedule, EntityId<E>, &FieldValue) -> Result<(), VerificationError>),
    /// Re-read verification — read the field back and compare to attempted value.
    /// Uses `read_fn` internally; fails verification if field is write-only.
    ReRead,
}
```

The `attempted_value` parameter is the value that was passed to `write()` — verification checks that the final field value equals what was requested.

### Opt-In Verification

Verification is **opt-in** — fields only verify if they have a `verify_fn`:

- `verify_fn: None` — no verification (default, returns `Ok(())`)
- `verify_fn: Some(VerifyFn::ReRead)` — explicit read-back verification using `read_fn`
- `verify_fn: Some(VerifyFn::Bare(f))` — custom data-only verification
- `verify_fn: Some(VerifyFn::Schedule(f))` — custom schedule-aware verification

The `ReRead` variant is useful for fields that need value stability checking but don't need custom verification logic — it reads the field back via `read()` and compares to the attempted value.

### When to Verify

Verification runs only when:

1. A **batch write** affects **more than one writable field**, AND
2. The affected field has a `verify_fn` (verification is opt-in)

Single-field writes skip verification (no interdependency risk). Fields with `verify_fn: None` skip verification even during batch writes.

### Usage Pattern

```rust
static FIELD_DURATION: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "duration",
    // ... other fields ...
    write_fn: Some(WriteFn::Bare(|d, v| {
        let requested = v.into_duration()?;
        // Duration write moves end_time to achieve requested duration
        d.time_slot = TimeRange::new(
            d.time_slot.start,
            d.time_slot.start + requested
        );
        Ok(())
    })),
    verify_fn: Some(VerifyFn::Bare(|d, attempted| {
        let requested = attempted.as_duration()?;
        let actual = d.time_slot.duration();
        if requested != actual {
            // This fails if end_time was ALSO set in the same batch,
            // overriding the duration-driven end calculation
            return Err(VerificationError::ValueChanged {
                field: "duration",
                requested: requested.num_minutes(),
                actual: actual.num_minutes(),
            });
        }
        Ok(())
    })),
    // ...
};

// start_time with explicit ReRead verification
static FIELD_START_TIME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
    name: "start_time",
    // ...
    write_fn: Some(WriteFn::Bare(|d, v| {
        let new_start = v.into_datetime()?;
        d.time_slot = TimeRange::new(new_start, d.time_slot.end);
        Ok(())
    })),
    // Use ReRead to verify value survived batch writes
    verify_fn: Some(VerifyFn::ReRead),
    read_fn: Some(ReadFn::Bare(|d| Some(FieldValue::DateTime(d.time_slot.start)))),
    // ...
};
```

## Acceptance Criteria

- [x] `VerifyFn<E>` enum with `Bare`, `Schedule`, and `ReRead` variants
- [x] `VerificationError` type added to `value.rs`
- [x] `verify_fn` field added to `FieldDescriptor<E>`
- [x] `VerifiableField<E>` trait with `verify(&self, id, schedule, attempted_value)` method
- [x] `FieldDescriptor<E>` implements `VerifiableField<E>`
- [x] `VerifyFn::ReRead` for explicit read-back verification
- [x] Documentation updated in `field-system.md` explaining validate vs verify distinction
- [x] All existing `FieldDescriptor` statics updated with `verify_fn: None`

## Notes

This was originally part of the field system design but got lost when `validate` was added. The key difference:

- `validate` = "is this value acceptable?" (input validation)
- `verify` = "does the field have the value we attempted to set?" (value stability check)

The key insight: **we don't check cross-field formulas** (like `duration == end - start`). We only check that each written field **kept its intended value** after all batch writes completed. If two computed fields conflict, one of them will fail verification.

Verification is called by `FieldSet::write_multiple()` after all individual writes complete, not by single-field writes.

### Setting the Same Field Twice

If a batch contains duplicate field updates (e.g., `duration` set to 60 then 90), the verification logic should use the **final** attempted value (90) for comparison.

### Naming: `verify` vs `write_consistency`

Alternative names considered:

| Name                | Pros                                              | Cons                                                                     |
| ------------------- | ------------------------------------------------- | ------------------------------------------------------------------------ |
| `verify`            | Short, familiar, implies checking correctness     | Generic, could be confused with `validate` or cryptographic verification |
| `write_consistency` | Explicit about purpose — checking write stability | Verbose, "consistency" implies distributed systems / ACID                |
| `check_value`       | Clear action                                      | Generic, doesn't imply batch context                                     |
| `ensure_stable`     | Captures "value didn't change" semantics          | Unusual phrasing                                                         |

**Current preference: `verify`** — it's concise, pairs naturally with `validate` ("validate then verify"), and the context (batch writes) makes the purpose clear. The doc comment will clarify: "verify that the field still has the value that was written."
