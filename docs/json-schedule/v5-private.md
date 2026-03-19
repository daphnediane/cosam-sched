# Schedule JSON Format v5 - Private/Full

This document describes version 5 of the schedule JSON format, private/full variant. This format is produced and consumed by the Rust editor and converter for internal data storage and editing.

## Top-Level Structure

```json
{
  "meta": { ... },
  "panels": { "GP002": { ... }, "GW097": { ... } },
  "rooms": [ ... ],
  "panelTypes": [ ... ],
  "timeTypes": [ ... ],
  "timeline": [ ... ],
  "presenters": [ ... ],
  "conflicts": [ ... ]
}
```

All top-level keys are required except `conflicts`, which may be omitted when there are no scheduling conflicts.

## Structures

- [meta](meta-v5.md) - Metadata about the schedule file (shared with public format)
- [panels](panels-v5.md) - Hierarchical panels hash with base→part→session nesting
- [PanelPart](PanelPart-v5.md) - Panel part objects (subdivision of base panels)
- [PanelSession](PanelSession-v5.md) - Panel session objects (individual scheduled occurrences)
- [rooms](rooms-v4.md) - Physical and virtual event spaces (same as v4)
- [panelTypes](panelTypes-v4.md) - Event category definitions (same as v4)
- [timeTypes](timeTypes-v4.md) - Time category definitions (same as v4)
- [timeline](timeline-v4.md) - Key time markers for layout and navigation (same as v4)
- [presenters](presenters-v4.md) - People and groups that present events (same as v4)
- [conflicts](conflicts-v4.md) - Detected scheduling conflicts (same as v4)

## Key Changes from v4

- **Hierarchical panels structure**: Replaces flat `events` array with nested `panels` hash
- **Base→Part→Session nesting**: Supports complex panel subdivisions
- **Effective value computation**: Fields accumulate across hierarchy levels
- **Private/internal fields**: Added workshop notes, power needs, AV requirements, etc.
- **Variant support**: `meta.variant` distinguishes private from public format
- **Extra fields support**: Preserves arbitrary spreadsheet columns

## Hierarchy Overview

```text
panels (hash by base ID)
├── Panel (base level)
│   ├── base fields (name, description, cost, etc.)
│   └── parts[]
│       └── PanelPart
│           ├── part fields (partNum, additive description, etc.)
│           └── sessions[]
│               └── PanelSession
│                   ├── session fields (id, time, room, capacity, etc.)
│                   ├── private fields (workshopNotes, powerNeeds, etc.)
│                   └── extras (arbitrary spreadsheet columns)
```

## Effective Values

### Concatenated Fields

`description`, `note`, and `prereq` are concatenated across levels:

```text
[base.field, part.field, session.field]
```

### Override Fields

Use first non-null value scanning from most specific to least:

- `altPanelist`: session → part → base
- `ticketUrl`: session → base (no part level)
- `simpleTixEvent`: session → base (no part level)

### Presenter Lists

Effective presenter lists are the ordered union of all levels:

- `creditedPresenters`: base + part + session
- `uncreditedPresenters`: base + part + session

## Migration Notes

### v4 → v5-private

1. Convert flat `events` array to hierarchical `panels` hash
2. Group events by base ID (remove part/session suffixes)
3. Create PanelPart and PanelSession objects for subdivisions
4. Move event-level fields to appropriate hierarchy levels
5. Add private/internal fields for workshop management
6. Set `meta.variant = "full"`
7. Preserve all existing v4 structures (rooms, panelTypes, etc.)

### Base ID Extraction

| Uniq ID     | Base ID | Part | Session |
| ----------- | ------- | ---- | ------- |
| `GP002`     | `GP002` | none | none    |
| `GW097P1`   | `GW097` | `P1` | none    |
| `GW097P2S3` | `GW097` | `P2` | `S3`    |

## Example

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-06-01T12:00:00Z",
    "version": 5,
    "variant": "full",
    "generator": "cosam-editor 0.2.0",
    "startTime": "2026-06-26T17:00:00Z",
    "endTime": "2026-06-28T18:00:00Z"
  },
  "panels": {
    "GP002": {
      "id": "GP002",
      "name": "Cosplay Contest Misconceptions",
      "panelType": "panel-type-gp",
      "description": "A deep-dive into competition issues.",
      "isFree": true,
      "isKids": false,
      "creditedPresenters": ["December Wynn", "Pro", "Con"],
      "parts": [
        {
          "partNum": null,
          "sessions": [
            {
              "id": "GP002",
              "roomIds": [10],
              "startTime": "2026-06-26T14:00:00",
              "endTime": "2026-06-26T15:00:00",
              "duration": 60,
              "isFull": false
            }
          ]
        }
      ]
    }
  },
  "rooms": [],
  "panelTypes": [],
  "timeTypes": [],
  "timeline": [],
  "presenters": [],
  "conflicts": []
}
```

## Notes

- This format preserves all data from the original spreadsheet including internal notes
- The hierarchical structure better represents the logical organization of complex panels
- Private fields are filtered out when exporting to public format
- The format is designed for round-trip conversion with spreadsheet data
