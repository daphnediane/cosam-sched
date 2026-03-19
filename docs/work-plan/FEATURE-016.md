# v5 public JSON export (flattened for widget)

## Summary

Implement the public export mode that flattens the v5 hierarchy into an
ordered `panels` array suitable for the `cosam-calendar.js` widget.

## Status

Open

## Priority

High

## Description

`save_json_with_mode(Public)` must flatten the `panels` hash into a
chronologically ordered array per `docs/json-public-v5.md`, computing
effective field values, credits, and filtering out all private fields.

## Implementation Details

### Flattening algorithm

For each base panel, iterate parts in order; for each part iterate sessions
in order. Each session produces one entry in the output `panels` array.

### Effective field computation

For each output entry:

- **`description`**, **`note`**, **`prereq`**: concatenate non-null values from
  base → part → session, joined with a single space.
- **`altPanelist`**: first non-null among session → part → base.
- **`capacity`**, **`preRegMax`**: session value if set, else base value.
- **`cost`**, **`difficulty`**, **`ticketUrl`**, `isFree`, `isKids`, `name`,
  `panelType`: always from base.
- **`isFull`**: from session.
- **`roomIds`**: from session.

### Credits computation

Per the rules in `docs/json-public-v5.md` § Credits:

1. `hidePanelist` true on session → `credits: []`.
2. Effective `altPanelist` set → `credits: ["<value>"]`.
3. Otherwise apply group resolution to union of all `creditedPresenters`
   across base, part, session (same algorithm as v4).

### `presenters` (per entry)

Union of all `creditedPresenters` and `uncreditedPresenters` across base,
part, and session levels.

### Hidden panel types

Panels whose `panelType` maps to a hidden `PanelType` are excluded from
public output (same filtering as v4).

### Sorting

Output `panels` array sorted by `startTime` ascending (null last).

### `meta` fields

Set `meta.version = 5`, `meta.variant = "public"`. Omit `conflicts`.

## Acceptance Criteria

- Single-session panels produce one entry with `partNum: null`,
  `sessionNum: null`.
- Effective description correctly concatenates base + part + session values.
- Credits computed correctly for all three cases (hidePanelist, altPanelist,
  group resolution).
- Private fields (`seatsSold`, `notesNonPrinting`, etc.) absent from output.
- Output matches schema in `docs/json-public-v5.md`.

## References

- [json-public-v5.md](../json-public-v5.md)
- [json-private-v5.md](../json-private-v5.md)
- [FEATURE-012.md](FEATURE-012.md) — v5 Rust structs (prerequisite)
- [FEATURE-014.md](FEATURE-014.md) — xlsx_import v5 (prerequisite)
