# Spreadsheet Format

The converter reads XLSX spreadsheets in the same format used by
[schedule-to-html](https://github.com/daphnediane/schedule-to-html).
This document describes the expected sheet layout and column headers.

## General Notes

- All timestamps are in the form `M/DD/YYYY HH:MM` (month 1–12, day 1–31,
  24-hour clock) or as Excel date/time values.
- Spaces or underscores may be used interchangeably in header names.
- The converter canonicalizes headers by collapsing whitespace and certain
  punctuation into underscores, so `Start Time` and `Start_Time` are
  equivalent.

## Schedule Sheet

The main schedule data lives on a sheet named **Schedule** (case-insensitive).
If no sheet with that name exists, the first sheet is used. The first row is
treated as a header; every subsequent row is one event.

### Standard Columns

| Column Name          | Required?        | Description                                                                                             | Example                           |
| -------------------- | ---------------- | ------------------------------------------------------------------------------------------------------- | --------------------------------- |
| Uniq ID              | Yes if scheduled | Panel type prefix + number + optional suffix. Prefix maps to PanelTypes sheet.                          | `GP032`, `FW001`, `GW019A`        |
| Name                 | Yes if scheduled | Display name of the event.                                                                              | `Cosplay Foam Armor 101`          |
| Room                 | If scheduled     | Room name, must match the Rooms sheet. Multiple rooms separated by commas.                              | `Panel Room 1`                    |
| Start Time           | If scheduled     | Start date/time. Leave blank to "unschedule" a panel.                                                   | `6/25/2023 7:00 PM`               |
| End Time             |                  | End date/time. Computed from Start Time + Duration if omitted.                                          | `6/25/2023 8:00 PM`               |
| Duration             | If scheduled     | Length of the event in `H:MM` format or plain minutes.                                                  | `1:00`, `90`                      |
| Description          |                  | Event description shown to attendees.                                                                   | `Learn the basics of foam armor…` |
| Kind                 | If no PanelTypes | Panel kind string. Normally inferred from the Uniq ID prefix via the PanelTypes sheet.                  | `Workshop`                        |
| Cost                 |                  | Additional cost. Blank / `Free` / `$0` / `N/A` = included. `*` hides cost. `Kids` = kids event.         | `$35`, `Free`, `Kids`             |
| Full                 |                  | Non-blank if the event is full.                                                                         | `Yes`                             |
| Capacity             |                  | Total seats available.                                                                                  | `20`                              |
| Seats Sold           |                  | Number of seats already pre-sold or reserved via ticketing.                                             | `3`                               |
| PreReg Max           |                  | Maximum seats available for pre-registration (remainder reserved for at-convention sales).              | `15`                              |
| Note                 |                  | Extra note displayed verbatim.                                                                          | `All materials provided`          |
| Notes (Non Printing) |                  | Notes for internal use only                                                                             |                                   |
| Workshop Notes       |                  | Notes for the workshop staff use                                                                        |                                   |
| Power Needs          |                  | Power requirements for the workshop                                                                     |                                   |
| Sewing Machines      |                  | If sewing machines are required                                                                         |                                   |
| AV Notes             |                  | Audio/visual setup notes (not used by the widget).                                                      | `Mic: 2 handheld`                 |
| Difficulty           |                  | Skill level indicator (see below for format).                                                           | `Beginner`, `3`                   |
| Prereq               |                  | Comma-separated prerequisite panel IDs.                                                                 | `FW001,FW002`                     |
| Ticket Sale          |                  | URL for purchasing tickets. May be a `HYPERLINK()` formula.                                             | `https://…/simpletix.com/`        |
| Ticket URL           |                  | Alternate header for Ticket Sale                                                                        |                                   |
| Have Ticket Image    |                  | Record-keeping flag: whether a ticket/flyer image has been received and uploaded to the ticketing site. | `Yes`                             |
| SimpleTix Event      |                  | Link to the SimpleTix admin portal for this event, for quick access when updating details or images.    | `https://admin.simpletix.com/…`   |
| Hide Panelist        |                  | Non-blank to suppress presenter credits.                                                                | `Yes`                             |
| Alt Panelist         |                  | Override text for presenter line.                                                                       | `Mystery Guest`                   |

Uniq ID should be auto assigned if not supplied, with an unused prefix and random number. Once we fully switch to the new system, we can remove the Uniq Id and Old Uniq Id columns and
replace them with Uuids.

### Internal-Use Columns

The following columns appear in the spreadsheet for internal record-keeping and
are **ignored by the converter**:

| Column      | Description                                                               |
| ----------- | ------------------------------------------------------------------------- |
| Old Uniq Id | Previous Uniq ID if the panel was renumbered, for cross-referencing only. |
| Lstart      | Internally computed start timestamp used by the legacy scheduling tool.   |
| Lend        | Internally computed end timestamp used by the legacy scheduling tool.     |

### Uniq Id

This is the ID of the panel, typically it should be unique, though the system
will still work if IDs are shared. The Uniq ID format consists of:

- Two letters (required prefix) optionally followed by additional letters
- A number
- One or more suffixes, which are either:
  - `P` or `S` followed by a number (for parts or sessions)
  - Any letter besides `P` or `S`

The first two characters (prefix) are used to determine the panel type.
All panels with the same prefix and number are considered related, even if they
have different suffixes. Note that the ID is considered case-insensitive and
will be converted to uppercase when read, so GW032 and FP032 are different
panels even though both contain the number 32. If multiple panels have the
same exact id an internal sequence we be added as a suffix to make them unique.

Parts and Sessions

Parks are for series of tightly related panels and panels that extend over
multiple days. The assumption is that each part builds on the previous part.
This is mostly used for multi-day workshops.

Sessions are repeats or reruns of the same panel at different times to
accommodate demand and scheduling conflicts.

It is possible to have both parts and sessions for the same series, for
example a casting workshop that might have multiple times to make the molds
for part one GW093P1S1, GW093P1S2. And then a single session for part two
GW099P2 where the molds are opened and cleaned. Typically the part number
should be first, but either order is equivalent.

Examples

- GP032 - This is a guest panel 32.
- FP032 - This is fan panel 32.
- GW020P1 - This is part 1 of GW020
- GW020P2 - This is part 2 of GW020
- GW020P3 - This is part 3 of GW020
- GW021S1 - This is session 1 of GW021
- GW021S2 - This is session 2 of GW021
- SPLIT01 - Special panel used to indicate when to split the grid
- BREAK01 - Special panel used to indicate a convention wide break

### Room column

It is important to note that some panels have have multiple rooms listed
separated by columns.

### Duration or End Time

Some versions of the spreadsheet included an `End Time` column possibly
in addition to a `Duration` column. One of the columns might have a
Excel formula that computed it based on the other columns.

### Presenter Columns

Presenter attendance is encoded in **separate columns** — one per named
presenter — rather than a single "Presenters" field.

Two header formats are supported: tagged format and legacy format.

#### Tagged format: `Kind:Name=Group`

This is the format used by actual Cosplay America spreadsheets, with support for individual presenters and presenter groups.

| Header syntax       | Meaning                                                                            |
| ------------------- | ---------------------------------------------------------------------------------- |
| `G:Name`            | Guest named *Name*. Cell is a flag — any non-blank value means they are attending. |
| `G:Name=Group`      | Guest named *Name*, member of *Group*. Cell is a flag.                             |
| `G:Name==Group`     | Guest named *Name*, member of *Group*. Sets `always_shown` on the **Group**.       |
| `G:<Name=Group`     | Guest named *Name*, member of *Group*. Sets `always_grouped` on **Name**.          |
| `G:<Name==Group`    | Combination: `always_shown` on Group, `always_grouped` on Name.                    |
| `G:Other`           | Cell contains a **comma-separated list** of additional guest names.                |
| `J:Name`, `J:Other` | Same as above for **judges**.                                                      |
| `S:Name`, `S:Other` | Same as above for **staff**.                                                       |
| `I:Name`, `I:Other` | Same as above for **invited panelists**.                                           |
| `P:Name`, `P:Other` | Same as above for **panelists**.                                                   |
| `F:Name`, `F:Other` | Same as above for **fan panelists**.                                               |

**Group Handling:**

- **Single `=`** (`G:Name=Group`): Individual presenter who is a member of Group. The presenter may be shown individually or as part of the group depending on context.
- **Double `==`** (`G:Name==Group`): Sets `always_shown` on the **Group** — the group name is shown in credits even when not all members are present. This flag applies to the group, not the individual member.
- **`<` prefix** (`G:<Name=Group`): Sets `always_grouped` on the **individual member** — this member always appears under their group name in credits, never individually.
- **Combined** (`G:<Name==Group`): Both flags set — the group is always shown, and this member always appears under the group.
- **Group names in `Other` columns**: Names are processed for `=Group` and `==Group` syntax similar to header columns. If no `=` syntax is used, the name is treated as an individual presenter unless it has already been defined as a group elsewhere.
- **Group relationships**: Groups can have multiple members, and presenters can belong to multiple groups. Groups of groups are supported.

**Name Parsing:**

- `Kind:Other` columns use separator regex: `\s*(?:,\s*(?:and\s+)?|\band\s+)` to split comma-separated names
- Groups like "UNC Staff" are identified by patterns (ending with "Staff", "Team", etc.) or explicit configuration
- Individual presenters vs groups are distinguished for conflict detection and scheduling

Kind prefixes and their rank labels:

| Prefix | Rank            |
| ------ | --------------- |
| `G`    | `guest`         |
| `J`    | `judge`         |
| `S`    | `staff`         |
| `I`    | `invited_guest` |
| `P`    | `panelist`      |
| `F`    | `fan_panelist`  |

For `Kind:Name` columns the cell value is just a flag (`Yes`, `*`, etc.).
For `Kind:Other` columns the cell value is a comma-separated list of names.

#### Presenter Credit Display

**How groups vs individuals are shown:**

- **Individual presenters**: Shown by name when they are the only member attending from their group
- **Group names**: Shown instead of individuals when all group members are attending
- **Always-grouped presenters** (`==Group`): Always shown as part of their group, never individually
- **Mixed attendance**: "Group Name (Individual Name)" when some but not all members are present

**Example scenarios:**

```text
G:John Doe==UNC Staff    # John always shown as "UNC Staff"
G:Jane Smith=UNC Staff   # Jane shown individually or as "UNC Staff"
S:Other                 # "UNC Staff, Bob Johnson" (if UNC Staff defined as group elsewhere)
                        # "UNC Staff=Staff, Bob Johnson" (UNC Staff as group with = syntax)
```

**Group definition in Other columns:**

- `UNC Staff` - Treated as individual unless already defined as a group
- `UNC Staff=Staff` - Defines UNC Staff as a group named "Staff"
- `UNC Staff==Staff` - Defines UNC Staff as always-shown group "Staff"

**Conflict detection implications:**

- **Individual presenters**: Cannot be double-booked (conflicts detected)
- **Group presenters**: Can be "double-booked" (groups represent multiple people)
- **Mixed scenarios**: Group conflicts ignored, individual conflicts enforced

#### Legacy format

For older spreadsheets, the panelist will be at the end of the spreadsheet
and organized into categories. Other standard fields might appear afterwards,
typically `Alt Panelist` and `Hide Panelist`.

For each rank the following headers will be used:

1. Group header: one of `Guests`, `Judge`, `Staff`, `Invited`, `Fan Panelist`.
   This defines the rank, the data in the column will be a formula that builds a list of members of this rank from the columns that follow it. `Industry Panelist` might
   be used instead of `Invited`.
2. Name headers: one or more presenter name. `Name`, `Name=Group` or `Name==Group`.
   These headers are for a single present, but are not tagged like the newer format.
   The data cells work the same way as the newer format.
3. Other headers: either `Other <rank>` or `<rank> Other`. This is for additional
   panelist that do not have their own column, like the tag format they may
   be separated by commas and/or `and` (e.g. "John Doe, Jane Smith and Bob Johnson").

#### Fallback: generic Presenter column

If no dedicated presenter columns are detected, the converter will look for a
single `Presenter` or `Presenters` column and split it on commas.

### Difficulty

Difficulty for the panel, normally used for workshops. Will be displayed
as part of the description. No pre-defined values, so can use 1-5, or
Easy, etc.

Example:

- 1
- 5
- Easy
- Beginning
- Intermediate
- Challenging

## Rooms Sheet

A sheet named **Rooms** (or **RoomMap**) defines the room list.

| Column     | Required? | Description                                                                                                                                                                                     |
| ---------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Room Name  | Yes       | Short name, must match the Room column in the Schedule sheet.                                                                                                                                   |
| Sort Key   |           | Numeric sort order for the room grid display.                                                                                                                                                   |
| Long Name  |           | Display name shown in the widget (falls back to Room Name if absent).                                                                                                                           |
| Hotel Room |           | Physical hotel room or building name.                                                                                                                                                           |
| Is Pseudo  |           | Non-blank if this is a scheduling artifact, not a real physical room. Pseudo rooms are imported but excluded from the public export; panels assigned to them appear with no room in the widget. |

### Pseudo Rooms

Pseudo rooms are entries in the Rooms sheet that represent scheduling
conventions rather than physical spaces, marked with **Is Pseudo = Yes**.
Common examples:

| Room Name    | Purpose                                                               |
| ------------ | --------------------------------------------------------------------- |
| `SPLIT`      | Legacy schedule-to-html page-break marker; no physical room.          |
| `SPLITDAY`   | Same as SPLIT — day-boundary break.                                   |
| `SPLITNIGHT` | Same as SPLIT — night-boundary break.                                 |
| `BREAK`      | Convention-wide break; breaks are now encoded via panel type instead. |

Pseudo rooms are stored in the internal schedule so that panels
referencing them can still be read. However they do not appear in the
`rooms` array of the exported widget JSON, and panels assigned to
them export with `roomIds: []`.

Break panels (meals, day-end breaks, etc.) should use a panel type
with **Is Break = Yes** (e.g. prefix `BR`) rather than a pseudo room.

## PanelTypes Sheet

A sheet named **PanelTypes** maps Uniq ID prefixes to panel kinds.

| Column        | Description                                                             |
| ------------- | ----------------------------------------------------------------------- |
| Prefix        | Two-letter prefix of Uniq ID (e.g. `GP`, `FW`). This is the panel type  |
|               | identifier in v7+ (used as the hashmap key in the JSON format).         |
| Panel Kind    | Human-readable kind name (e.g. `Guest Panel`).                          |
| Hidden        | Non-blank if this type should be hidden from public schedule.           |
| Is Workshop   | Non-blank if this type is a paid workshop.                              |
| Is Break      | Non-blank if this type represents a break.                              |
| Is Café       | Non-blank if this type is a café panel.                                 |
| Is Room Hours | Non-blank if this type represents room operating hours.                 |
| Is TimeLine   | Non-blank if this type is a timeline/page-split marker. If absent,      |
|               | inferred from prefix starting with `SP` or `SPLIT`.                     |
| Is Private    | Non-blank if this type is private/staff-only (e.g. Staff Meal, ZZ).     |
| Color         | CSS color for the panel type (e.g. `#db2777`). Stored in the `colors`   |
|               | hashmap under the key `"color"` in v7+.                                 |
| BW            | Alternate color for monochrome output. Stored in `colors` under `"bw"`. |

## Sample Column Layouts by Year

The following sections document the column order used in each year's
spreadsheet. Presenter columns vary by year as guests change.
Note: 2020 and 2021 are not listed as the convention was not held those years.

### 2026

**Standard columns:** `Uniq ID`, `Name`, `Room`, `Start Time`, `Duration`,
`Description`, `Prereq`, `Note`, `Notes (Non Printing)`, `Workshop Notes`,
`Power Needs`, `Sewing Machines`, `AV Notes`, `Difficulty`, `Cost`,
`Seats Sold`, `PreReg Max`, `Capacity`, `Have Ticket Image`, `SimpleTix Event`,
`Ticket Sale`, `Hide Panelist`, `Alt Panelist`

**Presenter columns:**

```text
G: Name, Name, Name, Name==Group, Name==Group, Name, Name, Name, Name, Name
J: Name, Name, Name, Name
S: Name, Name, Name
I: Name*
P: Other
```

*\* The raw spreadsheet header has a stray space (`I: CUT/SEW`); the converter
normalizes it.*

**Internal columns (ignored):** `Old Uniq Id`

**Computed columns (ignored):** `Lstart`, `Lend`

### 2025

**Standard columns:** `Uniq ID`, `Name`, `Room`, `Start Time`, `Duration`,
`Description`, `Prereq`, `Note`, `Notes (Non Printing)`, `Workshop Notes`,
`Power Needs`, `Sewing Machines`, `AV Notes`, `Difficulty`, `Cost`,
`Seats Sold`, `PreReg Max`, `Capacity`, `Have Ticket Image`, `SimpleTix Event`,
`Ticket Sale`, `Hide Panelist`, `Alt Panelist`

**Presenter columns:**

```text
G: Name, Name, Name, Name==Group, Name==Group, Name, Name, Name, Name, Name,
   Name, Name, Name, Other
J: Name, Name, Name, Other
S: Name, Name, Other
I: Other
P: Other
```

**Internal columns (ignored):** `Old Uniq Id`

**Computed columns (ignored):** `Lstart`, `Lend`

### 2024

**Standard columns:** `Uniq ID`, `Name`, `Room`, `Start Time`, `Duration`,
`Description`, `Note`, `Notes (Non Printing)`, `Workshop Notes`, `Power Needs`,
`Sewing Machines`, `AV Notes`, `Difficulty`, `Cost`, `Seats Sold`, `Capacity`,
`Ticket Sale`, `Hide Panelist`, `Alt Panelist`

**Presenter columns:**

```text
G: Name, Name, Name, Name==Group, Name==Group, Name, Name, Name, Name, Name,
   Name, Name, Other
J: Name, Name, Name, Other
S: Name, Name, Other
I: Other
P: Other
```

**Internal columns (ignored):** `Old Uniq Id`

**Computed columns (ignored):** `Lstart`, `Lend`

### 2023

**Standard columns:** `Uniq ID`, `Name`, `Room`, `Start Time`, `Duration`,
`Description`, `Note`, `Notes (Non Printing)`, `AV Notes`, `Difficulty`,
`Cost`, `Seats Sold`, `Capacity`, `Hide Panelist`, `Alt Panelist`

**Presenter columns:**

```text
G: Name, Name, Name, Name, Name==Group, Name==Group, Name, Name, Name, Name,
   Name, Other
I: Name, Name, Name, Other
S: Name, Name, Name, Other
P: Other
```

**Internal columns (ignored):** `Old Uniq Id`

**Computed columns (ignored):** `Lstart`, `Lend`

### 2022

**Standard columns:** `Uniq ID`, `Changed`, `Name`, `Room`, `Start Time`,
`Duration`, `Description`, `Note`, `Notes (Non Printing)`, `AV Notes`,
`Difficulty`, `Cost`, `Seats Sold`, `Capacity`, `Hide Panelist`, `Alt Panelist`

**Presenter columns:**

```text
G: Name, Name, Name, Name, Name, Name==Group, Name==Group, Name, Name, Other
S: Name, Name, Name, Other
I: Name, Name, Name, Other
P: Other
```

**Computed columns (ignored):** `Prefix`, `Room from Grid`,
`Start Time from Grid`, `Duration from Grid`, `Grid Slot Same`

---

Legacy years (2016–2019) use an older column layout with computed formula
columns (`Panelist`, `End Time`, `Kind`, etc.) and group-header columns
(`Guests`, `Staff`, `Invited Groups`, etc.) that precede the individual
presenter name columns for that group. See
[Legacy format](#legacy-format) above for details on how these are parsed.

### 2019 (legacy format)

**Standard columns:** `Uniq ID`, `Changed`, `Name`, `Room`, `Original Time`,
`Start Time`, `Duration`, `Description`, `Note`, `Difficulty`, `Tokens`,
`Full`, `Alt Panelist`

**Presenter columns:**

```text
Guests:         Name x2, Named Group, Name ×11, Other Guests
Staff:          Name ×4, Other Staff
Invited Groups: Name ×4, Other Groups
Panelists:      Name ×4, Other Panelists
```

Instead of supporting groups the spreadsheet has a computed named group column
that combines the group members into a `Yes`, `*` or `` (empty) value depending of
the individual members. Would have been `Name=Group` in the modern style.

**Computed columns (ignored):** `Panelist`, `Newly Changed`, `End Time`, `Kind`,
`Day`, `12hr Time`, `Room Idx`, `Real Room`, `Long Room`

### 2018 (legacy format)

**Standard columns:** `Uniq ID`, `Changed`, `Name`, `Room`, `Original Time`,
`Start Time`, `Duration`, `Description`, `Note`, `Social Hook`, `Premium`,
`Alt Panelist`

**Presenter columns:**

```text
Guests:         Name ×7, Other Guests
Staff:          Name ×5, Other Staff
Invited Groups: Name ×4, Other Groups
Panelists:      Name ×4, Other Panelists
```

**Computed columns (ignored):** `Panelist`, `Newly Changed`, `End Time`, `Kind`, `Day`,
`12hr Time`, `Room Idx`, `Real Room`

### 2017 (legacy format)

**Standard columns:** `Uniq ID`, `Changed`, `Name`, `Room`, `Original Time`,
`Start Time`, `Duration`, `Description`, `Note`, `Social Hook`, `Premium`,
`Alt Panelist`

**Presenter columns:**

```text
Guests:         Name ×7, Other Guests
Staff:          Name ×4, Other Staff
Invited Groups: Name ×4, Other Groups
Panelists:      Name ×4, Other Panelists
```

**Computed columns (ignored):** `Panelist`, `Newly Changed`, `End Time`, `Kind`, `Day`,
`12hr Time`, `Room Idx`, `Real Room`

### 2016 (legacy format)

**Standard columns:** `Uniq ID`, `Changed`, `Name`, `Room`, `Original Time`,
`Start Time`, `Duration`, `Description`, `Social Hook`, `18+`, `Fan Panelist`

**Presenter columns:**

```text
Guests:         Name ×6
Staff:          Name ×9
Invited Groups: Name ×25
```

**Computed columns (ignored):** `Panelist`, `End Time`, `Kind`, `Day`, `12hr Time`, `Room Idx`, `Real Room`
