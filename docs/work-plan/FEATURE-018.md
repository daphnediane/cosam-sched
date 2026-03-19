# Editor v5 data model

## Summary

Update `apps/cosam-editor` to work with the v5 `Schedule` struct and expose
the base→part→session hierarchy in the UI.

## Status

In Progress

## Priority

Medium

## Description

The editor currently uses the v4 flat `events` list. This work item updates
the editor to read, display, and edit v5 data using the new `Panel`,
`PanelPart`, and `PanelSession` types from FEATURE-012.

## Implementation Details

### Data model

Replace all uses of `Event` and `Vec<Event>` in editor code with the v5
`Panel`/`PanelPart`/`PanelSession` types. The editor's internal state holds
a `Schedule` with `panels: IndexMap<String, Panel>`.

### Panel list view

Display base panels grouped by base ID. Show part/session count badges. Allow
expanding a base panel to see its parts and sessions inline.

### Edit panel

The panel edit form should show:

- **Base level**: name, panelType, description, note, prereq, altPanelist,
  cost, capacity, preRegMax, difficulty, ticketUrl, isFree, isKids,
  creditedPresenters, uncreditedPresenters, simpleTixEvent, haveTicketImage.
- **Part level** (per part): partNum, description, note, prereq, altPanelist,
  creditedPresenters, uncreditedPresenters.
- **Session level** (per session): sessionNum, description, note, prereq,
  altPanelist, roomIds, startTime, endTime, duration, isFull, capacity,
  seatsSold, preRegMax, hidePanelist, creditedPresenters, uncreditedPresenters,
  notesNonPrinting, workshopNotes, powerNeeds, sewingMachines, avNotes.

### Effective value preview

Show computed effective values (description, credits) in a read-only preview
panel so the user can see what the public export will contain.

### Import/export

- **Import from xlsx**: calls the updated `xlsx_import` (FEATURE-014).
- **Save full JSON**: calls `save_json_with_mode(Full)` (FEATURE-015).
- **Export public JSON**: calls `save_json_with_mode(Public)` (FEATURE-016).

## Acceptance Criteria

- Editor loads a v5 full JSON file and displays panels correctly.
- Editing any field at any level and re-saving produces valid v5 JSON.
- Effective value preview updates reactively.
- Import from xlsx produces the same v5 structure as the command-line converter.

## References

- [json-private-v5.md](../json-private-v5.md)
- [json-public-v5.md](../json-public-v5.md)
- [FEATURE-012.md](FEATURE-012.md) — v5 Rust structs (prerequisite)
- [FEATURE-014.md](FEATURE-014.md) — xlsx_import v5 (prerequisite)
- [FEATURE-015.md](FEATURE-015.md) — v5 private export (prerequisite)
- [FEATURE-016.md](FEATURE-016.md) — v5 public export (prerequisite)
