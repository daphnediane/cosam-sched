# v7-Display

Display format documentation for JSON schedule format v7. This is the public-facing format consumed by the schedule widget.

## Top-Level Structure

```json
{
  "meta": { ... },
  "panelTypes": { ... },
  "rooms": [ ... ],
  "presenters": [ ... ],
  "panels": [ ... ],
  "timeline": [ ... ]
}
```

## Structures

- [meta-v7.md](meta-v7.md) - Metadata with variant `"display"` (no `nextPresenterId`)
- [panelTypes-v7.md](panelTypes-v7.md) - Panel types hashmap keyed by prefix (no `metadata`)
- [rooms-v7.md](rooms-v7.md) - Room definitions (no `metadata`)
- [presenters-v7.md](presenters-v7.md) - Presenters with stable integer `id` (no `metadata`)
- [panels-display-v7.md](panels-display-v7.md) - Flattened panels array with baked-in breaks
- [timeline-v7.md](timeline-v7.md) - Timeline markers (no `metadata`)

## Key Changes from v6

### Variant Naming

The variant string changed from `"public"` to `"display"`.

### panelTypes Hashmap

Same as full format — hashmap keyed by uppercase prefix. The `uid` and `prefix` fields are removed. Panels reference panel types by prefix directly (e.g. `"panelType": "GP"`).

### Named Color Sets

Same as full format — `colors` hashmap replaces fixed `color` + `bwColor` fields.

### Baked-In Breaks

The display variant now includes implicit break panels as regular entries in the panels array:

- **`%IB` (Implicit Break)**: Gaps between scheduled panels during active hours, expanded across inactive rooms
- **`%NB` (Overnight Break)**: Overnight gaps between programming days

The widget no longer needs to compute implicit breaks at runtime.

### No Metadata

The optional `metadata` field present on items in the full format is **stripped** in the display variant. No `metadata` field appears on any item.

### No Conflicts

The `conflicts` array is not included in the display variant.

### Dropped Legacy Structures

- `timeTypes` top-level array (merged into panelTypes)
- `events` legacy array
- `panel-type-{prefix}` and `time-type-{prefix}` UID formats

## Complete Example

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-06-01T12:00:00Z",
    "version": 7,
    "variant": "display",
    "generator": "cosam-editor 0.3.0",
    "startTime": "2026-06-26T17:00:00Z",
    "endTime": "2026-06-28T18:00:00Z",
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
    "%IB": {
      "kind": "Implicit Break",
      "colors": { "color": "#F5F5F5" },
      "isBreak": true,
      "isHidden": false,
      "isTimeline": false
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
  "panels": [
    {
      "id": "GP002",
      "baseId": "GP002",
      "name": "Cosplay Contest Misconceptions",
      "panelType": "GP",
      "roomIds": [10],
      "startTime": "2026-06-26T14:00:00",
      "endTime": "2026-06-26T15:00:00",
      "duration": 60,
      "description": "A deep-dive into competition issues.",
      "isFree": true,
      "credits": ["December Wynn"],
      "presenters": ["December Wynn"]
    },
    {
      "id": "%IB001",
      "baseId": "%IB001",
      "name": "Break",
      "panelType": "%IB",
      "roomIds": [10],
      "startTime": "2026-06-26T15:00:00",
      "endTime": "2026-06-26T15:30:00",
      "duration": 30,
      "isFree": true,
      "credits": [],
      "presenters": []
    }
  ],
  "timeline": [
    {
      "id": "SPLIT01",
      "startTime": "2026-06-26T17:00:00Z",
      "description": "Thursday Evening",
      "panelType": "SPLIT"
    }
  ]
}
```

## Notes

- This format is consumed by `cosam-calendar.js` widget
- All effective values are pre-computed — the widget does not need hierarchy logic
- `credits` contains formatted display strings; `presenters` contains raw names for filtering
- Implicit break panels have synthetic panel type prefixes starting with `%`
- Private fields (`creator`, `lastModifiedBy`, `nextPresenterId`, `metadata`, etc.) are excluded
