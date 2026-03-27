# Full Format v9

**Access Level**: Private  
**Status**: Supported  
**Version**: 9

Complete internal schedule format with full presenter data and edit history support.

## Top-Level Structure

```json
{
  "meta": Meta,
  "conflicts": Array<ScheduleConflict>,
  "panelSets": Object<PanelSet>,
  "panelTypes": Object<PanelType>,
  "rooms": Array<Room>,
  "presenters": Array<Presenter>,
  "timeline": Array<TimelineEntry>,
  "importedSheets": ImportedSheetPresence,
  "changeLog": Array<EditCommand>
}
```

## Structures

- [meta-v9.md](meta-v9.md) - Metadata structure
- [presenters-v9.md](presenters-v9.md) - Presenters with PresenterSortRank
- [PanelSet-v8.md](PanelSet-v8.md) - Hierarchical panel sets
- [Panel-v8.md](Panel-v8.md) - Panel objects
- [PanelType-v7.md](PanelType-v7.md) - Panel type definitions
- [rooms-v7.md](rooms-v7.md) - Room definitions
- [timeline-v7.md](timeline-v7.md) - Timeline markers
- [conflicts-v7.md](conflicts-v7.md) - Conflict detection
- [changeLog-v8.md](changeLog-v8.md) - Edit history
- [ImportedSheetPresence-v6.md](ImportedSheetPresence-v6.md) - Sheet tracking

## Key Changes from v8

- **PresenterSortRank**: New struct replaces `columnRank` and `indexRank` with unified sorting
- **memberIndex**: Eliminates index-doubling hack for group vs member ordering
- **No other structural changes** - v9 maintains compatibility with v8 for all other structures

## Complete Example

```json
{
  "meta": {
    "title": "Event Schedule 2026",
    "version": 9,
    "variant": "full",
    "generator": "cosam-editor 0.1.0",
    "generated": "2026-03-26T22:00:00Z",
    "creator": "Daphne Pfister",
    "lastModifiedBy": "Daphne Pfister",
    "modified": "2026-03-26T21:45:00Z",
    "startTime": "2026-05-29T17:00:00Z",
    "endTime": "2026-06-01T15:00:00Z",
    "nextPresenterId": 100
  },
  "conflicts": [],
  "panelSets": {
    "panel-001": {
      "base": {
        "id": "panel-001",
        "baseId": "panel-001",
        "name": "Opening Ceremony",
        "panelType": "panel-type-ceremony",
        "roomIds": [1],
        "timing": {
          "startTime": "2026-05-29T17:00:00Z"
        },
        "credits": ["MC Host"],
        "presenters": ["MC Host"]
      },
      "parts": [],
      "sessions": []
    }
  },
  "panelTypes": {
    "panel-type-ceremony": {
      "prefix": "ceremony",
      "kind": "Ceremony",
      "colors": {
        "color": "#FF6B6B"
      },
      "isBreak": false,
      "isCafe": false,
      "isWorkshop": false,
      "isHidden": false,
      "isRoomHours": false,
      "isTimeline": false,
      "isPrivate": false
    }
  },
  "rooms": [
    {
      "uid": 1,
      "name": "Main Hall",
      "building": "Convention Center",
      "isBreak": false
    }
  ],
  "presenters": [
    {
      "id": 1,
      "name": "MC Host",
      "rank": "guest",
      "sortRank": {
        "columnIndex": 0,
        "rowIndex": 0,
        "memberIndex": 0
      },
      "isMember": {
        "NotMember": {}
      },
      "isGrouped": {
        "NotGroup": {}
      }
    }
  ],
  "timeline": [],
  "importedSheets": {
    "hasRoomMap": true,
    "hasPanelTypes": true,
    "hasPresenters": true,
    "hasSchedule": true
  },
  "changeLog": []
}
```
