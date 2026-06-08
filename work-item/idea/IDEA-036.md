# Per-Membership Edge Flags (subsumes_members / show_individually)

## Summary

Explore restoring per-membership granularity for `subsumes_members` and
`show_individually` if entity-level flags prove insufficient.

## Status

Open

## Priority

Low

## Description

Currently `subsumes_members` and `show_individually` are entity-level fields
on `Presenter`, meaning they apply to **all** of a presenter's group memberships
equally. This matches the old `schedule-to-html` Perl implementation behavior.

The old `PresenterToGroup` edge stored these as per-edge flags, allowing a
presenter to be `subsumes_members` with respect to Group A but not Group B. This
distinction was not actually used in the spreadsheet data, but the model
supported it.

### Options to explore

- Store `groups` as `Vec<GroupMembership { group_id, subsumes_members,
  show_individually }>` on `Presenter` instead of `Vec<PresenterId>`
- Keep entity-level flags as the default; override per-membership optionally
- Introduce a separate `PresenterGroupFlags` entity keyed by `(member, group)`
  pair

### Considerations

- How does this interact with the CRDT / undo model?
- Per-membership structs complicate reverse index maintenance
- Not needed for current convention data; defer until a real use case emerges
