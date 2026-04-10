# Merge Conflict Resolution UI

## Summary

Provide UI for reviewing and resolving merge conflicts after sync.

## Status

Open

## Priority

Low

## Description

When two peers edit the same field concurrently, the CRDT automatically picks
a winner (typically last-writer-wins), but the user should be able to review
these decisions and override them.

### Conflict Display

- List of entities with conflicting fields after a sync
- For each conflict: field name, local value, remote value, auto-resolved value
- Timestamp and author attribution for each side

### Resolution Options

- Accept auto-resolved value (default)
- Choose local value
- Choose remote value
- Enter a custom value
- Defer resolution (mark for later review)

### Integration Points

- `cosam-editor` (GUI): Conflict panel in the sidebar or a dedicated dialog
- `cosam-modify` (CLI): `cosam-modify conflicts <file>` to list,
  `cosam-modify resolve <file> --entity <uuid> --field <name> --pick <local|remote>`

### Conflict History

- Keep a log of resolved conflicts for auditing
- Show conflict count in schedule metadata

## Acceptance Criteria

- Conflicts are surfaced after sync in both GUI and CLI
- User can accept, override, or defer each conflict
- Resolved conflicts are logged
- All resolution paths produce valid schedule state
- Unit tests for conflict detection and resolution paths
