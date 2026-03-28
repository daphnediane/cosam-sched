# v6-Private

Private format documentation for JSON schedule format v6.

This document is generated from the structured documentation in [json-schedule](json-schedule).

---

## Top-Level Structure

```json
{
  "meta": { ... },
  "panelTypes": { ... },
  "rooms": { ... },
  "panels": { ... },
  "conflicts": [ ... ]
}
```

## Structures Overview

- [meta-v6.md](meta-v6.md) - Metadata structure with Excel file integration
- [panels-v5.md](panels-v5.md) - Hierarchical panels hash (private) (unchanged from v5)
- [PanelPart-v5.md](PanelPart-v5.md) - Panel part objects (private) (unchanged from v5)
- [PanelSession-v5.md](PanelSession-v5.md) - Panel session objects (private) (unchanged from v5)
- [panelTypes-v5.md](panelTypes-v5.md) - Panel type categories (unchanged from v5)
- [rooms-v5.md](rooms-v5.md) - Room definitions (unchanged from v5)
- [conflicts-v5.md](conflicts-v5.md) - Conflict detection structures (unchanged from v5)

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

### [`panels`](json-schedule/panels-v5.md)

`panels` is a JSON object keyed by **base ID** containing hierarchical panel data with base→part→session nesting.

**Access:** Private

**Status:** Supported in v5 (private format only)

**Key Fields:**

| Field                  | Type                           | Public | Description                                                      |
| ---------------------- | ------------------------------ | ------ | ---------------------------------------------------------------- |
| `id`                   | string                         | yes    | Base ID (same as hash key, e.g. `"GW097"`)                       |
| `name`                 | string                         | yes    | Display name of the panel                                        |
| `panelType`            | string \| null                 | yes    | Panel type UID (e.g. `"panel-type-gw"`)                          |
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
| `creditedPresenters`   | string[]                       | yes    | Individual presenter names who appear in credits (non-`*` flag)  |
| `uncreditedPresenters` | string[]                       | no     | Individual presenter names attending but suppressed from credits |
| `simpleTixEvent`       | string \| null                 | no     | Default SimpleTix admin portal link; sessions may override       |

*See full details in: [`panels-v5.md`](json-schedule/panels-v5.md)*

### [`PanelPart`](json-schedule/PanelPart-v5.md)

`PanelPart` is an object representing a subdivision of a base panel, containing one or more sessions.

**Access:** Private

**Status:** Supported in v5 (private format only)

**Key Fields:**

| Field                  | Type                                 | Public | Description                                                             |
| ---------------------- | ------------------------------------ | ------ | ----------------------------------------------------------------------- |
| `partNum`              | integer \| null                      | yes    | Part number (e.g. `1` for `P1` suffix); `null` when no part subdivision |
| `description`          | string \| null                       | yes    | Additive description for this part (appended to base description)       |
| `note`                 | string \| null                       | yes    | Additive note for this part                                             |
| `prereq`               | string \| null                       | yes    | Additive prerequisite text for this part                                |
| `altPanelist`          | string \| null                       | yes    | Override credits text; takes precedence over base when set              |
| `creditedPresenters`   | string[]                             | yes    | Additional credited presenter names for this part                       |
| `uncreditedPresenters` | string[]                             | no     | Additional uncredited presenter names for this part                     |

*See full details in: [`PanelPart-v5.md`](json-schedule/PanelPart-v5.md)*

### [`PanelSession`](json-schedule/PanelSession-v5.md)

`PanelSession` is an object representing a specific scheduled occurrence of a panel part.

**Access:** Private

**Status:** Supported in v5 (private format only)

**Key Fields:**

| Field                  | Type            | Public | Description                                                                   |
| ---------------------- | --------------- | ------ | ----------------------------------------------------------------------------- |
| `id`                   | string          | yes    | Full Uniq ID for this session (e.g. `"GW097P1"`, `"GP002"`)                   |
| `sessionNum`           | integer \| null | yes    | Session number (e.g. `2` for `S2` suffix); `null` when no session subdivision |
| `description`          | string \| null  | yes    | Additive description for this session                                         |
| `note`                 | string \| null  | yes    | Additive note for this session                                                |
| `prereq`               | string \| null  | yes    | Additive prerequisite text for this session                                   |
| `altPanelist`          | string \| null  | yes    | Override credits text; takes precedence over part and base when set           |
| `roomIds`              | integer[]       | yes    | Room UIDs for this session; empty array if unscheduled                        |
| `startTime`            | string \| null  | yes    | ISO 8601 local datetime; null if unscheduled                                  |
| `endTime`              | string \| null  | yes    | ISO 8601 local datetime                                                       |
| `duration`             | integer         | yes    | Duration in minutes                                                           |
| `isFull`               | boolean         | yes    | True if this session is at capacity                                           |
| `capacity`             | string \| null  | yes    | Per-session seat capacity override; null = use base value                     |
| `seatsSold`            | integer \| null | no     | Number of seats already pre-sold via ticketing                                |
| `preRegMax`            | string \| null  | no     | Per-session pre-reg maximum; null = use base value                            |
| `ticketUrl`            | string \| null  | yes    | Per-session ticket URL override; null = use base value                        |
| `simpleTixEvent`       | string \| null  | no     | Per-session SimpleTix admin portal link; null = use base value                |
| `hidePanelist`         | boolean         | no     | True to suppress presenter credits entirely for this session                  |
| `creditedPresenters`   | string[]        | yes    | Additional credited presenter names for this session                          |
| `uncreditedPresenters` | string[]        | no     | Additional uncredited presenter names for this session                        |
| `notesNonPrinting`     | string \| null  | no     | Internal notes (not shown publicly)                                           |
| `workshopNotes`        | string \| null  | no     | Notes for workshop staff                                                      |
| `powerNeeds`           | string \| null  | no     | Power requirements                                                            |
| `sewingMachines`       | boolean         | no     | True if sewing machines are required                                          |
| `avNotes`              | string \| null  | no     | Audio/visual setup notes                                                      |

*See full details in: [`PanelSession-v5.md`](json-schedule/PanelSession-v5.md)*

## Migration Notes

---

## Related Documentation

- [JSON Schedule Documentation](json-schedule/) - Complete structured documentation
- [Display Format v10](json-v10-display.md) - Public-facing schedule format with DisplayPresenter objects and filtered presenter list.
- [Full Format v10](json-v10-full.md) - Complete internal schedule format with flat presenter relationship fields and edit history support.
- [Schedule JSON Format v4](json-format-v4.md) - This document describes version 4 of the schedule JSON format. V4 introduces timeline support and time types while maintaining backward compatibility with earlier versions.
- [Schedule JSON Format v5 - Private/Full](json-private-v5.md) - This document describes version 5 of the schedule JSON format, private/full variant. This format is produced and consumed by the Rust editor and converter for internal data storage and editing.
- [Schedule JSON Format v5 - Public/Widget](json-public-v5.md) - This document describes version 5 of the schedule JSON format, public/widget variant. This format is produced by the Rust converter or editor in public export mode and consumed by the schedule widget.
- [v6-Public](json-public-v6.md) - Public format documentation for JSON schedule format v6.
- [v7-Display](json-v7-display.md) - Display format documentation for JSON schedule format v7. This is the public-facing format consumed by the schedule widget.
- [v7-Full](json-v7-full.md) - Full format documentation for JSON schedule format v7. This is the editable master format used by the editor and converter.
- [v8-Full](json-v8-full.md) - Full format documentation for JSON schedule format v8. This is the editable master format used by the editor and converter, with support for persistent edit history via the optional `changeLog` field.
- [Display Format v9](json-v9-display.md) - Public-facing schedule format with DisplayPresenter objects and filtered presenter list.
- [Full Format v9](json-v9-full.md) - Complete internal schedule format with full presenter data and edit history support.

*This document is automatically generated. Do not edit directly.*
