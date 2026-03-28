# PanelSet Structure v9

**Access Level**: Private  
**Status**: Supported  
**Version**: v9-full

Groups related panels under a common base ID using a flat model.

## Fields

| Field    | Type          | Public | Description                                          |
| -------- | ------------- | ------ | ---------------------------------------------------- |
| base\_id | String        | ✗      | Base identifier for the panel group (e.g. `"GP002"`) |
| panels   | Array\<Panel> | ✗      | Flat array of all panels in this set                 |

## Key Changes from v8

- **Flat model**: v9 replaces the hierarchical `base → parts → sessions` nesting with a flat `panels` array. Each `Panel` is fully self-contained with its own `id`, `partNum`, `sessionNum`, `timing`, `roomIds`, etc.
- **`base_id`**: Replaces the old `id` field at the PanelSet level. Each panel within the set carries its own unique `id` (e.g. `"GP002P1S2"`).

## JSON Example

```json
{
  "GP002": {
    "baseId": "GP002",
    "panels": [
      {
        "id": "GP002",
        "baseId": "GP002",
        "name": "Cosplay Contest Misconceptions",
        "panelType": "panel-type-GP",
        "roomIds": [10],
        "timing": {
          "Scheduled": {
            "start_time": "2026-06-26T14:00:00",
            "duration": 60
          }
        },
        "creditedPresenters": ["December Wynn"]
      }
    ]
  }
}
```

## Notes

- The top-level key in the `panelSets` object is the base ID
- `panels` contains all panels for this set in a flat array — no intermediate `parts` or `sessions` nesting
- Panel IDs follow the pattern `{baseId}[P{partNum}][S{sessionNum}]`
- The `changeState` field is not serialized (runtime-only)
