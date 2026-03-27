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
- [PanelSet-v9.md](PanelSet-v9.md) - Flat panel sets
- [Panel-v9.md](Panel-v9.md) - Self-contained panel objects with TimeRange timing
- [panelTypes-v7.md](panelTypes-v7.md) - Panel type definitions
- [rooms-v7.md](rooms-v7.md) - Room definitions
- [timeline-v7.md](timeline-v7.md) - Timeline markers
- [conflicts-v7.md](conflicts-v7.md) - Conflict detection
- [changeLog-v8.md](changeLog-v8.md) - Edit history
- [ImportedSheetPresence-v6.md](ImportedSheetPresence-v6.md) - Sheet tracking

## Key Changes from v8

- **PresenterSortRank**: New struct replaces `columnRank` and `indexRank` with unified sorting
- **memberIndex**: Eliminates index-doubling hack for group vs member ordering
- **Flat panel model**: PanelSets now contain a flat `panels` array instead of hierarchical `base → parts → sessions` nesting
- **TimeRange timing**: Panel timing uses a tagged enum (`Unspecified`, `UnspecifiedWithDuration`, `UnspecifiedWithStart`, `Scheduled`) instead of flat `startTime`/`endTime`/`duration` fields

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
      "baseId": "panel-001",
      "panels": [
        {
          "id": "panel-001",
          "baseId": "panel-001",
          "name": "Opening Ceremony",
          "panelType": "panel-type-ceremony",
          "roomIds": [1],
          "timing": {
            "Scheduled": {
              "start_time": "2026-05-29T17:00:00",
              "duration": 60
            }
          },
          "creditedPresenters": ["MC Host"]
        }
      ]
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
