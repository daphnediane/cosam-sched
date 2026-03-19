# xlsx_import: build v5 panel hierarchy

## Summary

Update `xlsx_import` to directly build the v5 base→part→session hierarchy
when importing spreadsheet data.

## Status

Open

## Priority

High

## Description

Currently `xlsx_import` produces a flat `Vec<Event>`. This work item replaces
that with direct population of `IndexMap<String, Panel>` per the v5 private
format (`docs/json-private-v5.md`).

## Implementation Details

### Uniq ID parsing

Use `PanelId` (from FEATURE-012) to split each row's Uniq ID into
`{ base_id, part_num, session_num }`.

### Hierarchy construction

For each spreadsheet row:

1. Look up or create `Panel` entry at `panels[base_id]`.
2. If it is the first row for this base ID, store all inherited fields
   (name, panelType, cost, capacity, preRegMax, difficulty, ticketUrl,
   creditedPresenters, uncreditedPresenters, simpleTixEvent, haveTicketImage)
   at the base level.
3. Find or create the `PanelPart` with the matching `part_num`.
4. Append a new `PanelSession` to that part's sessions list with the
   session-level fields (roomIds, startTime, endTime, duration, isFull,
   seatsSold, notesNonPrinting, workshopNotes, powerNeeds, sewingMachines,
   avNotes, hidePanelist, creditedPresenters, uncreditedPresenters).

### Description common-prefix algorithm

When adding a second row to an existing base ID or part, run the
common-prefix factoring algorithm described in `docs/json-private-v5.md`
§ Description Common-Prefix Algorithm for `description`, `note`, and `prereq`
fields.

### `altPanelist` mapping

Map the spreadsheet `Alt Panelist` column to `session.altPanelist`. If the
value is the same across all sessions of a part, promote it to `part.altPanelist`
and clear the session values. Apply the same promotion to the base level if
uniform across all parts.

### Multiple rooms

The spreadsheet `Room` column may contain comma-separated room names. Map each
name to its `uid` from the rooms lookup and store as `roomIds: Vec<u32>`.

## Acceptance Criteria

- Single-row panels produce `parts: [{partNum: null, sessions: [{sessionNum: null, ...}]}]`.
- Multi-part panels (e.g. `GW097P1`, `GW097P2`) group correctly under `"GW097"`.
- Description common-prefix algorithm produces correct factoring for the example
  in `json-private-v5.md`.
- All private fields from the spreadsheet are stored at the correct level.
- Existing round-trip tests pass with v5 output.

## References

- [json-private-v5.md](../json-private-v5.md)
- [FEATURE-012.md](FEATURE-012.md) — v5 Rust structs (prerequisite)
