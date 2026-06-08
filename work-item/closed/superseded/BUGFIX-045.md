# BUGFIX-045: Duration stored as Integer instead of Duration in field_update_logic.rs

## Summary

In `scratch/field_update_logic.rs`, duration values are incorrectly stored as `FieldValue::Integer(minutes)` instead of `FieldValue::Duration(Duration)`.

## Status

Superseded

## Priority

Medium

## Description

The `FieldValue` enum has a dedicated `Duration(Duration)` variant for representing time durations. However, in `scratch/field_update_logic.rs`, duration values are being pushed as `FieldValue::Integer(new_duration_minutes)` instead of using the proper `FieldValue::Duration` variant with a `chrono::Duration`.

This is a type safety issue — durations should be typed as `Duration`, not raw integers, to ensure:

- Type-safe operations (can't accidentally add minutes to a count field)
- Proper serialization (duration format vs raw number)
- Clear semantic meaning in the type system

## How Found

Code review while documenting the verify callback feature (FEATURE-043). Noticed the example used `into_integer()` for duration when `into_duration()` should exist.

## Location

File: `scratch/field_update_logic.rs`
Line: 200

```rust
affected_fields.push(FieldValue::Integer(new_duration_minutes));
```

## Steps to Fix

Change to:

```rust
affected_fields.push(FieldValue::Duration(chrono::Duration::minutes(new_duration_minutes)));
```

Also check for other occurrences in the file where duration might be stored as Integer.

## Testing

- Verify the file compiles after change
- Check that `into_duration()` is available on `FieldValue` (it is — defined in `value.rs:151`)
- No runtime tests needed for scratch file, but ensure the pattern is correct for future reference
