# v6-Public

Public format documentation for JSON schedule format v6.

This document is generated from the structured documentation in [json-schedule](json-schedule).

---

## Top-Level Structure

```json
{
  "meta": { ... },
  "panels": [ ... ]
}
```

## Structures Overview

- [meta-v6.md](meta-v6.md) - Metadata structure (Excel metadata partially included)
- [panels-public-v5.md](panels-public-v5.md) - Flattened panels array (public) (unchanged from v5)

## Structure Details

### [`meta`](json-schedule/meta-v6.md)

`meta` is a JSON object containing metadata about the schedule file itself.

**Access:** Public

**Status:** Supported in v6

**Key Fields:**

| Field            | Type    | Public | Description                                                 |
| ---------------- | ------- | ------ | ----------------------------------------------------------- |
| `title`          | string  | yes    | Display title for the schedule                              |
| `generated`      | string  | yes    | ISO 8601 UTC timestamp when the file was generated          |
| `version`        | integer | yes    | Schema version number (always `6` for this format)          |
| `variant`        | string  | yes    | Format variant: `"full"` for private, `"public"` for public |
| `generator`      | string  | yes    | Identifier of the tool that produced the file               |
| `startTime`      | string  | yes    | ISO 8601 UTC timestamp of the schedule start date           |
| `endTime`        | string  | yes    | ISO 8601 UTC timestamp of the schedule end date             |
| `creator`        | string  | no     | Excel file creator/author (private format only)             |
| `lastModifiedBy` | string  | no     | Excel file last modified by (private format only)           |

*See full details in: [`meta-v6.md`](json-schedule/meta-v6.md)*

### [`panels`](json-schedule/panels-public-v5.md)

`panels` is a JSON array where each entry represents one **session** - the smallest schedulable unit, flattened from the private hierarchical format.

**Access:** Public

**Status:** Supported in v5 (public format only)

**Key Fields:**

| Field         | Type            | Public | Description                                                  |
| ------------- | --------------- | ------ | ------------------------------------------------------------ |
| `id`          | string          | yes    | Full Uniq ID of this session (e.g. `"GW097P1S2"`, `"GP002"`) |
| `baseId`      | string          | yes    | Base panel ID (e.g. `"GW097"`, `"GP002"`)                    |
| `partNum`     | integer \| null | yes    | Part number; `null` when no part subdivision                 |
| `sessionNum`  | integer \| null | yes    | Session number; `null` when no session subdivision           |
| `name`        | string          | yes    | Display name (from base panel)                               |
| `panelType`   | string \| null  | yes    | Panel type UID (e.g. `"panel-type-gw"`)                      |
| `roomIds`     | integer[]       | yes    | Room UIDs for this session; empty array if unscheduled       |
| `startTime`   | string \| null  | yes    | ISO 8601 local datetime; null if unscheduled                 |
| `endTime`     | string \| null  | yes    | ISO 8601 local datetime                                      |
| `duration`    | integer         | yes    | Duration in minutes                                          |
| `description` | string \| null  | yes    | Effective description (base + part + session concatenated)   |
| `note`        | string \| null  | yes    | Effective note                                               |
| `prereq`      | string \| null  | yes    | Effective prerequisite text                                  |
| `cost`        | string \| null  | yes    | Cost string from base (see Cost Values in v4 documentation)  |
| `capacity`    | string \| null  | yes    | Effective seat capacity (session override or base default)   |
| `difficulty`  | string \| null  | yes    | Skill level indicator (from base)                            |
| `ticketUrl`   | string \| null  | yes    | Effective ticket URL (session override or base default)      |
| `isFree`      | boolean         | yes    | True if no additional cost                                   |
| `isFull`      | boolean         | yes    | True if this session is at capacity                          |
| `isKids`      | boolean         | yes    | True for kids-only panels                                    |
| `credits`     | string[]        | yes    | Formatted credit strings for public display                  |

*See full details in: [`panels-public-v5.md`](json-schedule/panels-public-v5.md)*

## Migration Notes

---

## Related Documentation

- [JSON Schedule Documentation](json-schedule/) - Complete structured documentation
- [Display Format v10](json-v10-display.md) - Public-facing schedule format with DisplayPresenter objects and filtered presenter list.
- [Full Format v10](json-v10-full.md) - Complete internal schedule format with flat presenter relationship fields and edit history support.
- [Schedule JSON Format v4](json-format-v4.md) - This document describes version 4 of the schedule JSON format. V4 introduces timeline support and time types while maintaining backward compatibility with earlier versions.
- [Schedule JSON Format v5 - Private/Full](json-private-v5.md) - This document describes version 5 of the schedule JSON format, private/full variant. This format is produced and consumed by the Rust editor and converter for internal data storage and editing.
- [Schedule JSON Format v5 - Public/Widget](json-public-v5.md) - This document describes version 5 of the schedule JSON format, public/widget variant. This format is produced by the Rust converter or editor in public export mode and consumed by the schedule widget.
- [v6-Private](json-private-v6.md) - Private format documentation for JSON schedule format v6.
- [v7-Display](json-v7-display.md) - Display format documentation for JSON schedule format v7. This is the public-facing format consumed by the schedule widget.
- [v7-Full](json-v7-full.md) - Full format documentation for JSON schedule format v7. This is the editable master format used by the editor and converter.
- [v8-Full](json-v8-full.md) - Full format documentation for JSON schedule format v8. This is the editable master format used by the editor and converter, with support for persistent edit history via the optional `changeLog` field.
- [Display Format v9](json-v9-display.md) - Public-facing schedule format with DisplayPresenter objects and filtered presenter list.
- [Full Format v9](json-v9-full.md) - Complete internal schedule format with full presenter data and edit history support.

*This document is automatically generated. Do not edit directly.*
