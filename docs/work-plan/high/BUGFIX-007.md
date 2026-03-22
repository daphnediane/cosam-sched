# Fix `==Group` parsing in xlsx_import

## Summary

The `==Group` syntax in presenter headers incorrectly sets `always_grouped` on the member instead of `always_shown` on the group.

## Status

Open

## Priority

High

## Description

In the current Rust `xlsx_import.rs`, when parsing presenter headers with `==Group` syntax (e.g. `G:Name==Group`), the code sets `always_grouped: true` on the **member** (Name). This is incorrect.

The original `schedule-to-html` Perl code (`Presenter.pm` lines 293–302) handles this correctly:

- `==Group` strips the leading `=` and sets `always_shown` on the **group** object
- `<Name` strips the `<` and sets `always_grouped` on the **member** object

### Steps to Fix

1. In `xlsx_import.rs`, locate the presenter header parsing logic that handles `==`
2. Change it so that `==Group` sets `always_shown: true` on the group presenter entry, not `always_grouped` on the member
3. Add the `always_shown` field to the `Presenter` struct (part of v7 struct changes)
4. Update existing tests to verify the correct behavior

### Root Cause

The original implementation conflated `always_shown` (a group property) with `always_grouped` (a member property). The schedule-to-html code clearly separates these:

```perl
# Line 293: == on group name sets always_shown on the GROUP
my $always_shown = $group =~ s{ \A = }{}xms;
$ginfo->set_is_always_shown( 1 ) if $always_shown;

# Line 302: < on member name sets always_grouped on the MEMBER
my $always_grouped = $name =~ s{ \A < }{}xms;
```

### Dependencies

- Requires v7 presenter struct changes (`always_shown` field)
- Should be coordinated with FEATURE-021 (`<Name` prefix support)

## Acceptance Criteria

- `G:Name==Group` sets `always_shown` on the Group presenter, not `always_grouped` on Name
- Existing `always_grouped` behavior remains correct for other cases
- JSON round-trip preserves both `always_shown` and `always_grouped` flags
- Unit tests verify the corrected parsing
