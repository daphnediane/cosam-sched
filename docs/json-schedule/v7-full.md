# v7-Full

Full format documentation for JSON schedule format v7. This is the editable master format used by the editor and converter.

## Top-Level Structure

```json
{
  "meta": { ... },
  "panelTypes": { ... },
  "rooms": [ ... ],
  "presenters": [ ... ],
  "panels": { ... },
  "timeline": [ ... ],
  "conflicts": [ ... ]
}
```

## Structures

- [meta-v7.md](meta-v7.md) - Metadata with `nextPresenterId` and variant `"full"`
- [panelTypes-v7.md](panelTypes-v7.md) - Panel types hashmap keyed by prefix, with named color sets
- [rooms-v7.md](rooms-v7.md) - Room definitions with `is_break` flag
- [presenters-v7.md](presenters-v7.md) - Presenters with stable integer `id`, `always_shown`, and `always_grouped`
- [panels-v7.md](panels-v7.md) - Hierarchical panels hash (full format)
- [PanelPart-v5.md](PanelPart-v5.md) - Panel part objects (unchanged from v5)
- [PanelSession-v7.md](PanelSession-v7.md) - Panel session objects (`extras` renamed to `metadata`)
- [timeline-v7.md](timeline-v7.md) - Timeline markers referencing panelType prefix
- [conflicts-v7.md](conflicts-v7.md) - Conflict detection structures

## Key Changes from v6

### panelTypes Hashmap

Panel types changed from an array to a hashmap keyed by uppercase prefix. The `uid` and `prefix` fields are removed — the key **is** the prefix. Panels reference panel types by prefix directly (e.g. `"panelType": "GP"` instead of `"panel-type-gp"`).

### Named Color Sets

The fixed `color` + `bwColor` fields on panel types are replaced with a `colors` hashmap supporting named color sets (e.g. `{"color": "#E2F9D7", "bw": "#CCCCCC"}`). Additional per-theme overrides can be added without schema changes.

### timeTypes Merged into panelTypes

The separate `timeTypes` array is removed. Timeline panel types are now entries in `panelTypes` with `isTimeline: true`. Timeline entries reference the panelType prefix directly instead of `"time-type-{prefix}"` UIDs.

### New panelType Flags

- `isTimeline`: Merged from timeTypes
- `isPrivate`: For staff-only panel types (e.g. Staff Meal)

### Stable Presenter IDs

Presenters now have a stable integer `id` field. The `meta.nextPresenterId` counter tracks the next available ID. IDs are never reused.

### Corrected Group Semantics

- **`always_shown`** (new): Set on groups via `==Group` spreadsheet syntax — group name shown in credits even when not all members present
- **`always_grouped`** (existing): Set on members via `<Name` spreadsheet syntax — member always appears under group name

### Optional Metadata on All Items

An optional `metadata` field (key-value object) is available on all major item types in the full format: panels, panel sessions, rooms, panel types, presenters, and timeline entries. This field captures non-standard spreadsheet columns and other round-trip data. It is stripped in the display variant.

The `extras` field on PanelSession is renamed to `metadata` for consistency.

### Room Break Flag

Rooms have a new `is_break` boolean for virtual break rooms.

### Variant Naming

The variant string changed from `"public"` to `"display"`.

### Dropped Legacy Structures

- `timeTypes` top-level array (merged into panelTypes)
- `events` legacy array
- `panel-type-{prefix}` and `time-type-{prefix}` UID formats
- All backward-compatibility migration code

## Complete Example

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-06-01T12:00:00Z",
    "version": 7,
    "variant": "full",
    "generator": "cosam-editor 0.3.0",
    "startTime": "2026-06-26T17:00:00Z",
    "endTime": "2026-06-28T18:00:00Z",
    "nextPresenterId": 42,
    "creator": "Schedule Editor",
    "lastModifiedBy": "Admin User",
    "modified": "2026-05-15T14:30:00Z"
  },
  "panelTypes": {
    "GP": {
      "kind": "Guest Panel",
      "colors": { "color": "#E2F9D7", "bw": "#CCCCCC" },
      "isBreak": false,
      "isCafe": false,
      "isWorkshop": false,
      "isHidden": false,
      "isRoomHours": false,
      "isTimeline": false,
      "isPrivate": false
    },
    "BR": {
      "kind": "Break",
      "colors": { "color": "#E8E8E8" },
      "isBreak": true,
      "isHidden": true
    }
  },
  "rooms": [
    {
      "uid": 10,
      "short_name": "GP",
      "long_name": "Main Panel Room",
      "hotel_room": "Salon B/C",
      "sort_key": 1,
      "is_break": false
    }
  ],
  "presenters": [
    {
      "id": 1,
      "name": "December Wynn",
      "rank": "guest",
      "is_group": false,
      "members": [],
      "groups": [],
      "always_grouped": false,
      "always_shown": false
    }
  ],
  "panels": {
    "GP002": {
      "id": "GP002",
      "name": "Cosplay Contest Misconceptions",
      "panelType": "GP",
      "description": "A deep-dive into competition issues.",
      "isFree": true,
      "creditedPresenters": ["December Wynn"],
      "parts": [
        {
          "partNum": null,
          "sessions": [
            {
              "id": "GP002",
              "roomIds": [10],
              "startTime": "2026-06-26T14:00:00",
              "endTime": "2026-06-26T15:00:00",
              "duration": 60
            }
          ]
        }
      ]
    }
  },
  "timeline": [
    {
      "id": "SPLIT01",
      "startTime": "2026-06-26T17:00:00Z",
      "description": "Thursday Evening",
      "panelType": "SPLIT"
    }
  ],
  "conflicts": []
}
```
