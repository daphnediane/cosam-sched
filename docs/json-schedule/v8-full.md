# v8-Full

Full format documentation for JSON schedule format v8. This is the editable master format used by the editor and converter, with support for persistent edit history via the optional `changeLog` field.

## Top-Level Structure

```json
{
  "meta": { ... },
  "panelTypes": { ... },
  "rooms": [ ... ],
  "presenters": [ ... ],
  "panels": { ... },
  "timeline": [ ... ],
  "conflicts": [ ... ],
  "changeLog": { ... }
}
```

## Structures

- [meta-v8.md](meta-v8.md) - Metadata with version 8 and variant `"full"`
- [panelTypes-v7.md](panelTypes-v7.md) - Panel types hashmap keyed by prefix, with named color sets
- [rooms-v7.md](rooms-v7.md) - Room definitions with `is_break` flag
- [presenters-v7.md](presenters-v7.md) - Presenters with stable integer `id`, `always_shown`, and `always_grouped`
- [panels-v7.md](panels-v7.md) - Hierarchical panels hash (full format)
- [PanelPart-v5.md](PanelPart-v5.md) - Panel part objects (unchanged from v5)
- [PanelSession-v7.md](PanelSession-v7.md) - Panel session objects (`extras` renamed to `metadata`)
- [timeline-v7.md](timeline-v7.md) - Timeline markers referencing panelType prefix
- [conflicts-v7.md](conflicts-v7.md) - Conflict detection structures
- [changeLog-v8.md](changeLog-v8.md) - Edit history with undo/redo stacks

## Key Changes from v7

### ChangeLog Support (New)

The most significant change in v8 is the addition of the optional `changeLog` field that enables persistent edit history across file saves and application invocations.

- **`changeLog`**: Optional top-level field containing undo/redo stacks
- **Edit persistence**: Edit history is saved with the file and restored on load
- **Cross-session undo/redo**: Applications can undo/redo edits made in previous sessions
- **Omitted when empty**: The `changeLog` field is not included when both undo and redo stacks are empty

### Version Update

- Schema version updated from `7` to `8`
- All other structures remain identical to v7
- Backward compatibility: Files without `changeLog` load as v7 files

## Complete Example

```json
{
  "meta": {
    "title": "Cosplay America 2026 Schedule",
    "generated": "2026-06-01T12:00:00Z",
    "version": 8,
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
  "conflicts": [],
  "changeLog": {
    "undoStack": [
      {
        "type": "EditPanelDescription",
        "panelId": "GP002",
        "oldDescription": "Old description",
        "newDescription": "A deep-dive into competition issues.",
        "timestamp": "2026-06-01T12:30:00Z"
      }
    ],
    "redoStack": [],
    "maxDepth": 50
  }
}
```

## ChangeLog Example

The `changeLog` field contains edit history that enables undo/redo functionality:

```json
{
  "changeLog": {
    "undoStack": [
      {
        "type": "EditPanelName",
        "panelId": "GP002",
        "oldName": "Old Panel Name",
        "newName": "Cosplay Contest Misconceptions",
        "timestamp": "2026-06-01T12:15:00Z"
      },
      {
        "type": "EditPanelDescription",
        "panelId": "GP002",
        "oldDescription": "Previous description",
        "newDescription": "A deep-dive into competition issues.",
        "timestamp": "2026-06-01T12:30:00Z"
      }
    ],
    "redoStack": [
      {
        "type": "EditPanelName",
        "panelId": "GP003",
        "oldName": "Another Panel",
        "newName": "Updated Panel Name",
        "timestamp": "2026-06-01T11:45:00Z"
      }
    ],
    "maxDepth": 50
  }
}
```

## Usage Notes

### When changeLog is Omitted

The `changeLog` field is omitted entirely when:

- Both `undoStack` and `redoStack` are empty arrays
- The file has no edit history (e.g., freshly converted from Excel)
- The file is a display/export variant

### Backward Compatibility

- Files without `changeLog` are treated as v7 files
- All existing v7 files will load correctly in v8-aware applications
- Edit history is only preserved when saving in v8 format

### Format Variants

- **Full format**: Includes `changeLog` when edit history exists
- **Display format**: Remains at version 7, no `changeLog` field
- **Empty files**: Remain at version 4

## Migration Notes

No automated v7→v8 migration is needed since this is alpha software. All JSON files are regenerated from canonical spreadsheets each release. The changeLog feature is only available for files edited and saved using the v8-aware toolchain.
