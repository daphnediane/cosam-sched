# `meta`

`meta` is a JSON object containing metadata about the schedule file itself.

## Access

Public

## Status

Supported in v4

## Fields

| Field       | Type    | Public | Description                                        |
| ----------- | ------- | ------ | -------------------------------------------------- |
| `title`     | string  | yes    | Display title for the schedule                     |
| `generated` | string  | yes    | ISO 8601 UTC timestamp when the file was generated |
| `version`   | integer | yes    | Schema version number (always `4` for this format) |
| `generator` | string  | yes    | Identifier of the tool that produced the file      |
| `startTime` | string  | yes    | ISO 8601 UTC timestamp of the schedule start date  |
| `endTime`   | string  | yes    | ISO 8601 UTC timestamp of the schedule end date    |

## Description

The `meta` object provides essential metadata about the schedule file including its title, when it was generated, and by what tool. The `version` field indicates the schema version, which is `4` for this format.

### Version Information

- **v1**: Implicit version (no `version` field)
- **v2**: Added `version` and `generator` fields
- **v3**: Same structure as v2
- **v4**: Added `startTime` and `endTime` fields for schedule bounds

### Timestamp Formats

All timestamps use ISO 8601 UTC format: `YYYY-MM-DDTHH:MM:SSZ`

- `generated`: When the file was created
- `startTime`/`endTime`: Overall schedule start and end times

## Examples

```json
{
  "title": "Cosplay America 2026 Schedule",
  "generated": "2026-06-01T12:00:00Z",
  "version": 4,
  "generator": "cosam-editor 0.2.0",
  "startTime": "2026-06-26T17:00:00Z",
  "endTime": "2026-06-28T18:00:00Z"
}
```

## Notes

- For older files without a `version` field, the format is assumed to be v1
- The `generator` field helps identify which tool created the file and can be useful for debugging
- `startTime` and `endTime` are used for schedule boundary calculations and validation
