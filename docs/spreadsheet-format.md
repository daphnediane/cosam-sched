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

| Column Name   | Required?        | Description                                                                                     | Example                           |
| ------------- | ---------------- | ----------------------------------------------------------------------------------------------- | --------------------------------- |
| Uniq ID       | Yes              | Panel type prefix + number + optional suffix. Prefix maps to PanelTypes sheet.                  | `GP032`, `FW001`, `GW019A`        |
| Name          | Yes              | Display name of the event.                                                                      | `Cosplay Foam Armor 101`          |
| Room          | If scheduled     | Room name, must match the Rooms sheet. Multiple rooms separated by commas.                      | `Panel Room 1`                    |
| Start Time    | If scheduled     | Start date/time. Leave blank to "unschedule" a panel.                                           | `6/25/2023 7:00 PM`               |
| End Time      |                  | End date/time. Computed from Start Time + Duration if omitted.                                  | `6/25/2023 8:00 PM`               |
| Duration      | If scheduled     | Length of the event in `H:MM` format or plain minutes.                                          | `1:00`, `90`                      |
| Description   |                  | Event description shown to attendees.                                                           | `Learn the basics of foam armor…` |
| Kind          | If no PanelTypes | Panel kind string. Normally inferred from the Uniq ID prefix via the PanelTypes sheet.          | `Workshop`                        |
| Cost          |                  | Additional cost. Blank / `Free` / `$0` / `N/A` = included. `*` hides cost. `Kids` = kids event. | `$35`, `Free`, `Kids`             |
| Full          |                  | Non-blank if the event is full.                                                                 | `Yes`                             |
| Capacity      |                  | Total seats available.                                                                          | `20`                              |
| Difficulty    |                  | Skill level indicator (free-form).                                                              | `Beginner`, `3`                   |
| Note          |                  | Extra note displayed verbatim.                                                                  | `All materials provided`          |
| Prereq        |                  | Comma-separated prerequisite panel IDs.                                                         | `FW001,FW002`                     |
| Ticket Sale   |                  | URL for purchasing tickets. May be a `HYPERLINK()` formula.                                     | `https://…/simpletix.com/`        |
| Hide Panelist |                  | Non-blank to suppress presenter credits.                                                        | `Yes`                             |
| Alt Panelist  |                  | Override text for presenter line.                                                               | `Mystery Guest`                   |
| AV Notes      |                  | Audio/visual setup notes (not used by the widget).                                              | `Mic: 2 handheld`                 |

### Special Uniq ID Prefixes

| Prefix  | Meaning                                                                |
| ------- | ---------------------------------------------------------------------- |
| `SPLIT` | Page-break marker for print layout (e.g. `SPLIT01`). Ignored entirely. |
| `BREAK` | Convention-wide break (e.g. `BREAK01`). Shown as a break banner.       |

### Presenter Columns

Presenter attendance is encoded in **separate columns** — one per named
presenter — rather than a single "Presenters" field. Two header formats are
supported:

#### Primary format: `Kind:Name=Group`

This is the format used by actual Cosplay America spreadsheets, with support for individual presenters and presenter groups.

| Header syntax       | Meaning                                                                            |
| ------------------- | ---------------------------------------------------------------------------------- |
| `G:Name`            | Guest named *Name*. Cell is a flag — any non-blank value means they are attending. |
| `G:Name=Group`      | Guest named *Name*, member of *Group*. Cell is a flag.                             |
| `G:Name==Group`     | Guest named *Name*, **always shown as member of Group** (group takes precedence).  |
| `G:Other`           | Cell contains a **comma-separated list** of additional guest names.                |
| `J:Name`, `J:Other` | Same as above for **judges**.                                                      |
| `S:Name`, `S:Other` | Same as above for **staff**.                                                       |
| `I:Name`, `I:Other` | Same as above for **invited panelists**.                                           |
| `P:Name`, `P:Other` | Same as above for **fan panelists**.                                               |

**Group Handling:**

- **Single `=`** (`G:Name=Group`): Individual presenter who is a member of Group. The presenter may be shown individually or as part of the group depending on context.
- **Double `==`** (`G:Name==Group`): Presenter is **always shown as part of Group**. The group name takes precedence in credits.
- **Group names in `Other` columns**: Names are processed for `=Group` and `==Group` syntax similar to header columns. If no `=` syntax is used, the name is treated as an individual presenter unless it has already been defined as a group elsewhere.
- **Group relationships**: Groups can have multiple members, and presenters can belong to multiple groups.

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
| `P`    | `fan_panelist`  |

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

#### Legacy format: letter + digits

Some older spreadsheets may use numbered columns instead:

| Header        | Meaning                                         |
| ------------- | ----------------------------------------------- |
| `g1`, `g2`, … | Guest columns (cell value = presenter name).    |
| `p1`, `p2`, … | Fan panelist columns.                           |
| `Guest1`, …   | Same, with full word prefix.                    |
| `Other`       | Additional presenters as comma-separated names. |

#### Fallback: generic Presenter column

If no dedicated presenter columns are detected, the converter will look for a
single `Presenter` or `Presenters` column and split it on commas.

## Rooms Sheet

A sheet named **Rooms** defines the room list.

| Column     | Description                                                   |
| ---------- | ------------------------------------------------------------- |
| Room Name  | Short name, must match the Room column in the Schedule sheet. |
| Long Name  | Display name shown in the widget.                             |
| Hotel Room | Physical hotel room name.                                     |
| Sort Key   | Numeric sort order. Values ≥ 100 are hidden.                  |

### Special Rooms

Room names beginning with `SPLIT` (e.g. `SPLITDAY`, `SPLITNIGHT`) are used
to control page breaks in schedule-to-html and are filtered out by the
converter.

## PanelTypes Sheet

A sheet named **PanelTypes** maps Uniq ID prefixes to panel kinds.

| Column        | Description                                     |
| ------------- | ----------------------------------------------- |
| Prefix        | Two-letter prefix of Uniq ID (e.g. `GP`, `FW`). |
| Panel Kind    | Human-readable kind name (e.g. `Guest Panel`).  |
| Hidden        | Non-blank if this type should be hidden.        |
| Is Workshop   | Non-blank if this type is a paid workshop.      |
| Is Break      | Non-blank if this type represents a break.      |
| Is Café       | Non-blank if this type is a café panel.         |
| Is Room Hours | Non-blank if this type represents room hours.   |
| Color         | CSS color for the panel type (e.g. `#db2777`).  |
| BW            | Alternate color for monochrome output.          |
