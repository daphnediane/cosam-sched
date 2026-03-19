# `meta`

`meta` is a JSON object containing metadata about the schedule file itself.

## Access

Public

## Status

Supported in v5

## Fields

| Field       | Type    | Public | Description                                                 |
| ----------- | ------- | ------ | ----------------------------------------------------------- |
| `title`     | string  | yes    | Display title for the schedule                              |
| `generated` | string  | yes    | ISO 8601 UTC timestamp when the file was generated          |
| `version`   | integer | yes    | Schema version number (always `5` for this format)          |
| `variant`   | string  | yes    | Format variant: `"full"` for private, `"public"` for public |
| `generator` | string  | yes    | Identifier of the tool that produced the file               |
| `startTime` | string  | yes    | ISO 8601 UTC timestamp of the schedule start date           |
| `endTime`   | string  | yes    | ISO 8601 UTC timestamp of the schedule end date             |

## Description

The `meta` object provides essential metadata about the schedule file including its title, version, variant, and generation information.

### Version Information

- **v5**: Current version with hierarchical panels structure and variant support

### Variant Field

The `variant` field distinguishes between the two v5 formats:

- **`"full"`**: Private/internal format with complete hierarchical data
- **`"public"`**: Public format with flattened panels and filtered fields

### Timestamp Formats

All timestamps use ISO 8601 UTC format: `YYYY-MM-DDTHH:MM:SSZ`

- `generated`: When the file was created
- `startTime`/`endTime`: Overall schedule start and end times

## Examples

### Private Format

```json
{
  "title": "Cosplay America 2026 Schedule",
  "generated": "2026-06-01T12:00:00Z",
  "version": 5,
  "variant": "full",
  "generator": "cosam-editor 0.2.0",
  "startTime": "2026-06-26T17:00:00Z",
  "endTime": "2026-06-28T18:00:00Z"
}
```

### Public Format

```json
{
  "title": "Cosplay America 2026 Schedule",
  "generated": "2026-06-01T12:00:00Z",
  "version": 5,
  "variant": "public",
  "generator": "cosam-editor 0.2.0",
  "startTime": "2026-06-26T17:00:00Z",
  "endTime": "2026-06-28T18:00:00Z"
}
```

## Notes

- The `variant` field is required in v5 to distinguish between private and public formats
- All other fields are identical between private and public variants
- The `generator` field helps identify which tool created the file and can be useful for debugging
- `startTime` and `endTime` are used for schedule boundary calculations and validation
