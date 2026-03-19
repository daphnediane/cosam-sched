# `PanelPart`

`PanelPart` is an object representing a subdivision of a base panel, containing one or more sessions.

## Access

Private

## Status

Supported in v5 (private format only)

## Fields

| Field                  | Type                                 | Public | Description                                                             |
| ---------------------- | ------------------------------------ | ------ | ----------------------------------------------------------------------- |
| `partNum`              | integer \| null                      | yes    | Part number (e.g. `1` for `P1` suffix); `null` when no part subdivision |
| `description`          | string \| null                       | yes    | Additive description for this part (appended to base description)       |
| `note`                 | string \| null                       | yes    | Additive note for this part                                             |
| `prereq`               | string \| null                       | yes    | Additive prerequisite text for this part                                |
| `altPanelist`          | string \| null                       | yes    | Override credits text; takes precedence over base when set              |
| `creditedPresenters`   | string[]                             | yes    | Additional credited presenter names for this part                       |
| `uncreditedPresenters` | string[]                             | no     | Additional uncredited presenter names for this part                     |
| `sessions`             | [PanelSession](PanelSession-v5.md)[] | yes    | Sessions list; always at least one entry                                |

## Description

Panel parts represent logical subdivisions of a base panel. A panel can have multiple parts (e.g., "Part 1", "Part 2"), or just one part with `partNum: null` when no subdivision is needed.

### Part Numbering

- `partNum: null`: No part subdivision (single part panel)
- `partNum: 1`: First part (corresponds to `P1` suffix in Uniq ID)
- `partNum: 2`: Second part (corresponds to `P2` suffix in Uniq ID)
- And so on...

### Effective Values

Part-level fields contribute to the effective values computed for sessions:

#### Concatenated fields

`description`, `note`, and `prereq` from the part level are concatenated with base and session values.

#### Override fields

`altPanelist` at the part level overrides base values but can be overridden by session-level values.

### Presenter Lists

Presenter lists at the part level are additive:

- `creditedPresenters`: Added to the effective credited presenter list
- `uncreditedPresenters`: Added to the effective uncredited presenter list

### Optional Fields

All fields whose type includes `null` and array fields may be **omitted entirely** when they have default values.

## Examples

### Single Part (No Subdivision)

```json
{
  "partNum": null,
  "description": null,
  "note": null,
  "prereq": null,
  "altPanelist": null,
  "creditedPresenters": [],
  "uncreditedPresenters": [],
  "sessions": [
    {
      "id": "GP002",
      "sessionNum": null,
      "roomIds": [10],
      "startTime": "2026-06-26T14:00:00",
      "endTime": "2026-06-26T15:00:00",
      "duration": 60
    }
  ]
}
```

### Multi-Part Panel

```json
{
  "partNum": 1,
  "description": "Part 1 unique content",
  "note": "Bring materials for hands-on practice",
  "prereq": null,
  "altPanelist": null,
  "creditedPresenters": ["Guest Speaker"],
  "uncreditedPresenters": [],
  "sessions": [
    {
      "id": "GW097P1",
      "sessionNum": null,
      "roomIds": [3],
      "startTime": "2026-06-26T10:00:00",
      "endTime": "2026-06-26T12:00:00",
      "duration": 120
    }
  ]
}
```

## Notes

- All parts must have at least one session
- Part numbers are used in Uniq ID generation (e.g., `GW097P1`)
- Part-level fields are additive to base-level fields for effective values
- The `partNum` field determines the suffix used in session IDs
