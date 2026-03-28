# Full Format v10

**Access Level**: Private  
**Status**: Current  
**Version**: 10

Complete internal schedule format with flat presenter relationship fields and edit history support.

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

- [meta-v9.md](meta-v9.md) - Metadata structure (unchanged from v9)
- [presenters-v10.md](presenters-v10.md) - Presenters with flat relationship fields
- [PanelSet-v9.md](PanelSet-v9.md) - Flat panel sets (unchanged from v9)
- [Panel-v9.md](Panel-v9.md) - Self-contained panel objects with TimeRange timing
- [panelTypes-v7.md](panelTypes-v7.md) - Panel type definitions
- [rooms-v7.md](rooms-v7.md) - Room definitions
- [timeline-v7.md](timeline-v7.md) - Timeline markers
- [conflicts-v7.md](conflicts-v7.md) - Conflict detection
- [changeLog-v8.md](changeLog-v8.md) - Edit history

## Key Changes from v9

- **Flat presenter relationship fields**: `isGroup`, `members`, `groups`, `alwaysGrouped`, `alwaysShown` replace the enum-based `isMember`/`isGrouped` fields on presenters
- **RelationshipManager**: Internally, group/member relationships are managed as edges in a `RelationshipManager`; the flat JSON fields are populated from this manager on save
- **Backward-compatible loading**: Deserialization still accepts the v9 enum format for old files

## Complete Example

```json
{
  "meta": {
    "title": "Event Schedule 2026",
    "version": 10,
    "variant": "full",
    "generator": "cosam-sched 0.1.0",
    "generated": "2026-03-27T22:00:00Z",
    "creator": "Daphne Pfister",
    "lastModifiedBy": "Daphne Pfister",
    "modified": "2026-03-27T21:45:00Z",
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
          "creditedPresenters": ["MC Host", "Pro"]
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
      "name": "MC Host",
      "rank": "guest",
      "isGroup": false,
      "members": [],
      "groups": [],
      "alwaysGrouped": false,
      "alwaysShown": false,
      "sortRank": {
        "columnIndex": 0,
        "rowIndex": 0
      }
    },
    {
      "name": "Pro",
      "rank": "guest",
      "isGroup": false,
      "members": [],
      "groups": ["Pros and Cons Cosplay"],
      "alwaysGrouped": true,
      "alwaysShown": false,
      "sortRank": {
        "columnIndex": 5,
        "rowIndex": 0,
        "memberIndex": 1
      }
    },
    {
      "name": "Pros and Cons Cosplay",
      "rank": "guest",
      "isGroup": true,
      "members": ["Pro", "Con"],
      "groups": [],
      "alwaysGrouped": false,
      "alwaysShown": true,
      "sortRank": {
        "columnIndex": 5,
        "rowIndex": 0,
        "memberIndex": 0
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
