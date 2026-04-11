# Per-Membership Edge Flags (always_grouped / always_shown_in_group)

## Summary

Explore restoring per-membership granularity for `always_grouped` and
`always_shown_in_group` if entity-level flags prove insufficient.

## Status

Open

## Priority

Low

## Description

Currently `always_grouped` and `always_shown_in_group` are entity-level fields
on `Presenter`, meaning they apply to **all** of a presenter's group memberships
equally.  This matches the old `schedule-to-html` Perl implementation behavior.

The old `PresenterToGroup` edge stored these as per-edge flags, allowing a
presenter to be `always_grouped` with respect to Group A but not Group B.  This
distinction was not actually used in the spreadsheet data, but the model
supported it.

### Options to explore

- Store `groups` as `Vec<GroupMembership { group_id, always_grouped,
  always_shown_in_group }>` on `Presenter` instead of `Vec<PresenterId>`
- Keep entity-level flags as the default; override per-membership optionally
- Introduce a separate `PresenterGroupFlags` entity keyed by `(member, group)`
  pair

### Considerations

- How does this interact with the CRDT / undo model?
- Per-membership structs complicate the reverse index maintenance in
  `PresenterEntityType::on_update`
- The feature is not needed for current convention data; defer until a real
  use case emerges

## Notes

Deferred from REFACTOR-036 (virtual edge refactor).  The old per-edge flags
were implemented in `PresenterToGroupData` and described in the original
FEATURE-007/008 documentation.
