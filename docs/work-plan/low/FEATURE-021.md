# Support `<Name` prefix syntax for always_grouped

## Summary

Support the `<Name` prefix syntax in spreadsheet presenter headers to set `always_grouped` on individual members.

## Status

Open

## Priority

Low

## Description

In the original `schedule-to-html` Perl code (`Presenter.pm` line 302), the `<` prefix on a member name sets `always_grouped` on that individual:

```perl
my $always_grouped = $name =~ s{ \A < }{}xms;
```

This syntax is not currently recognized by the Rust `xlsx_import.rs` code. The `<Name` prefix means this member should always appear under their group name in credits, never as an individual.

### Implementation Details

1. In `xlsx_import.rs` presenter header parsing, check for `<` prefix on the member name portion
2. Strip the `<` prefix and set `always_grouped: true` on the resulting presenter
3. Handle combinations: `G:<Name=Group` and `G:<Name==Group`
4. Update the `Other` column parsing to also support `<Name` syntax

### Dependencies

- Requires v7 presenter struct changes (though `always_grouped` field already exists)
- Should be coordinated with BUGFIX-007 (`==Group` fix)

## Acceptance Criteria

- `G:<Name=Group` header correctly sets `always_grouped` on the member
- `G:<Name==Group` sets both `always_grouped` on member and `always_shown` on group
- Existing headers without `<` prefix continue to work unchanged
- Round-trip through JSON preserves the `always_grouped` flag
