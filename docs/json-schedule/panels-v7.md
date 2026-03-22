# `panels`

`panels` is a JSON object keyed by **base ID** containing hierarchical panel data with base→part→session nesting.

## Access

Private

## Status

Supported in v7 (full format only)

## Fields

| Field                  | Type                           | Public | Description                                                      |
| ---------------------- | ------------------------------ | ------ | ---------------------------------------------------------------- |
| `id`                   | string                         | yes    | Base ID (same as hash key, e.g. `"GW097"`)                       |
| `name`                 | string                         | yes    | Display name of the panel                                        |
| `panelType`            | string \| null                 | yes    | Panel type prefix (e.g. `"GW"`), references panelTypes hash key  |
| `description`          | string \| null                 | yes    | Base portion of description (see Effective Values)               |
| `note`                 | string \| null                 | yes    | Base note text                                                   |
| `prereq`               | string \| null                 | yes    | Base prerequisite text                                           |
| `altPanelist`          | string \| null                 | yes    | Override text for credits line (see Effective Values)            |
| `cost`                 | string \| null                 | yes    | Cost string (see Cost Values in v4 documentation)                |
| `capacity`             | string \| null                 | yes    | Default seat capacity; sessions may override                     |
| `preRegMax`            | string \| null                 | no     | Default pre-reg maximum; sessions may override                   |
| `difficulty`           | string \| null                 | yes    | Skill level indicator (e.g. `"Beginner"`, `"3"`)                 |
| `ticketUrl`            | string \| null                 | yes    | Default URL for ticket purchase; sessions may override           |
| `isFree`               | boolean                        | yes    | True if no additional cost                                       |
| `isKids`               | boolean                        | yes    | True for kids-only panels                                        |
| `creditedPresenters`   | string[]                       | yes    | Individual presenter names who appear in credits                 |
| `uncreditedPresenters` | string[]                       | no     | Individual presenter names attending but suppressed from credits |
| `simpleTixEvent`       | string \| null                 | no     | Default SimpleTix admin portal link; sessions may override       |
| `parts`                | [PanelPart](PanelPart-v5.md)[] | yes    | Parts list; always at least one entry                            |
| `metadata`             | object \| null                 | no     | Optional key-value metadata (full format only)                   |

## Description

The panels hash contains the hierarchical representation of all panels in the schedule. Each entry represents a base panel with all its parts and sessions.

### Panel Type Reference

In v7, the `panelType` field references the panel type by its prefix directly (e.g. `"GP"`, `"GW"`), which is the key in the [panelTypes](panelTypes-v7.md) hashmap. This replaces the `"panel-type-{prefix}"` UID format used in v4–v6.

### Base ID Key Structure

The panels object is keyed by **base ID**, which is the panel type prefix plus number portion of the Uniq ID, with no part or session suffix:

| Uniq ID     | Base ID |
| ----------- | ------- |
| `GP002`     | `GP002` |
| `GW097P1`   | `GW097` |
| `GW097P2S3` | `GW097` |
| `ME001`     | `ME001` |

Panels with part or session suffixes all nest under the same base key.

### Effective Values

Several fields have effective values computed from the hierarchy:

#### Concatenated fields

`description`, `note`, and `prereq` are concatenated across levels. The effective value for a session is:

```text
[base.field, part.field, session.field]
```

joined with a single space, skipping any null or empty-string levels.

#### Override fields

The following fields use **first-wins override** semantics:

| Field            | Override chain                 |
| ---------------- | ------------------------------ |
| `altPanelist`    | session → part → base          |
| `ticketUrl`      | session → base (no part level) |
| `simpleTixEvent` | session → base (no part level) |

### Metadata

The `metadata` field on the base panel can store non-standard key-value pairs from extra spreadsheet columns or user-defined fields. It is only present in the full format and is stripped in the display variant. Session-level metadata is stored on [PanelSession](PanelSession-v7.md).

### Optional Fields

All fields whose type includes `null`, `boolean` fields that default to `false`, and array fields may be **omitted entirely** from the JSON file. Absent fields are treated identically to their default value.

## Examples

```json
{
  "GP002": {
    "id": "GP002",
    "name": "Cosplay Contest Misconceptions",
    "panelType": "GP",
    "description": "A deep-dive into competition issues.",
    "creditedPresenters": ["December Wynn", "Pro", "Con"],
    "isFree": true,
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
}
```

## Notes

- The panels hash is only present in the full (`"full"`) variant
- Display format uses a flattened [panels array](panels-display-v7.md) instead
- `panelType` now uses the prefix directly (e.g. `"GP"`) instead of `"panel-type-gp"`
- Base ID keys are derived from the Uniq ID by removing part and session suffixes
- All panels have at least one part, and all parts have at least one session
- `metadata` is only present in the full format and is stripped in the display variant
