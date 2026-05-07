# Field Comparison: All Branches vs main

Investigation for FEATURE-103. Documents which fields/columns exist in each
branch and how they map to the current main branch.

## Branch Overview

| Branch           | Language | Approach                                                     |
| ---------------- | -------- | ------------------------------------------------------------ |
| v9               | Rust     | XLSX-column-definition structs; flat entity data model       |
| v10-try1         | Rust     | XLSX-column-definition structs; nearly identical to v9       |
| v10-try3         | Rust     | `EntityFields` derive macro; attribute-based fields          |
| **main**         | Rust     | `FieldDescriptor` statics + `inventory` registration         |
| schedule-to-html | Perl     | `Readonly` column constants; dynamic presenter pattern-match |

All Rust branches have five entity types: Panel, Presenter, EventRoom, HotelRoom,
PanelType. The schemas are broadly compatible; the differences are in field
granularity, how computed/edge fields are represented, and a handful of columns
that were added or dropped across versions.

`schedule-to-html` is an independent Perl project that reads the same XLSX files
to generate HTML output. It has its own column registry and handles a subset of
the fields that main tracks.

---

## Panel / Schedule Sheet Columns

| XLSX Column              | v9  | v10-try1 | v10-try3 | main                  | Notes                                                      |
| ------------------------ | --- | -------- | -------- | --------------------- | ---------------------------------------------------------- |
| Uniq ID                  | ✓   | ✓        | ✓        | ✓ `uid` / `code`      | Stored in `PanelInternalData.code` (PanelUniqId), not CRDT |
| Old Uniq Id              | ✓   | ✓        | —        | (ignored)             | Import ignore; v9/v10-try1 only wrote it for reference     |
| Name                     | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Room                     | ✓   | ✓        | ✓        | ✓ (edge)              | Edge to EventRoom; not a scalar field in main              |
| Start Time               | ✓   | ✓        | ✓        | ✓ (computed)          | Stored inside `time_slot`; exposed as computed field       |
| Duration                 | ✓   | ✓        | ✓        | ✓ (computed)          | Stored inside `time_slot`                                  |
| End Time                 | ✓   | ✓        | ✓        | ✓ (computed/writable) | `FIELD_END_TIME` is a writable Derived field               |
| Description              | ✓   | ✓        | ✓        | ✓ (`Text`)            | RGA CRDT type in main                                      |
| Prereq                   | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Note                     | ✓   | ✓        | ✓        | ✓ (`Text`)            |                                                            |
| Notes (Non Printing)     | ✓   | ✓        | ✓        | ✓ (`Text`)            |                                                            |
| Workshop Notes           | ✓   | ✓        | ✓        | ✓ (`Text`)            |                                                            |
| Power Needs              | ✓   | ✓        | ✓        | ✓ (`Text`)            |                                                            |
| Sewing Machines          | ✓   | ✓        | ✓        | ✓ (Boolean)           |                                                            |
| AV Notes                 | ✓   | ✓        | ✓        | ✓ (`Text`)            | Promoted to `FieldDescriptor` in main                      |
| Difficulty               | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Cost                     | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Seats Sold               | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Pre-Reg Max              | ✓   | ✓        | ✓        | ✓ `pre_reg_max`       |                                                            |
| Capacity                 | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Have Ticket Image        | ✓   | ✓        | ✓        | ✓                     |                                                            |
| SimpleTix Event          | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Ticket Sale / Ticket URL | ✓   | ✓        | ✓        | ✓ `ticket_url`        | Two column names for same field; hyperlink-formula support |
| SimpleTix Link           | —   | —        | —        | ✓ `simpletix_link`    | **New in main** — separate admin link distinct from event  |
| Hide Panelist            | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Alt Panelist             | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Kind                     | ✓   | ✓        | ✓        | ✓ (import alias)      | Resolved via PanelType edge; direct Kind only as fallback  |
| Full                     | ✓   | ✓        | ✓        | ✓ `is_full`           |                                                            |
| Is Free                  | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Is Kids                  | ✓   | ✓        | ✓        | ✓                     |                                                            |
| Lstart                   | ✓   | ✓        | —        | Formula only          | Moved to `FormulaColumnDef`; regenerated on export         |
| Lend                     | ✓   | ✓        | —        | Formula only          | Moved to `FormulaColumnDef`; regenerated on export         |

### Panel: Computed / Edge Fields

| Field                                           | v9       | v10-try3 | main   | Notes                                  |
| ----------------------------------------------- | -------- | -------- | ------ | -------------------------------------- |
| `presenters` (credited union)                   | implied  | ✓        | ✓      | Derived from edge list                 |
| `credited_presenters` / `uncredited_presenters` | one list | partial  | ✓ each | Split into two independent edge lists  |
| `inclusive_presenters` (groups expanded)        | —        | —        | ✓      | **New in main** via transitive cache   |
| `credits` (formatted credit string)             | —        | ✓        | ✓      | Computed from credited presenters list |
| `event_rooms` edge                              | implied  | ✓        | ✓      |                                        |
| `hotel_rooms` (via event rooms)                 | —        | —        | ✓      | **New in main** — transitive lookup    |
| `panel_type` edge                               | implied  | ✓        | ✓      |                                        |

---

## Presenter / People Sheet Columns

| XLSX Column     | v9     | v10-try1 | v10-try3                     | main                            | Notes                                                                     |
| --------------- | ------ | -------- | ---------------------------- | ------------------------------- | ------------------------------------------------------------------------- |
| Name / Person   | ✓      | ✓        | ✓ `name`                     | ✓                               |                                                                           |
| Classification  | ✓      | ✓        | ✓ `rank`                     | ✓ `rank`                        | Stored as enum                                                            |
| Is Group        | ✓      | ✓        | ✓                            | ✓ `is_explicit_group`           |                                                                           |
| Always Grouped  | ✓      | ✓        | ✓                            | ✓                               |                                                                           |
| Always Shown    | ✓      | ✓        | ✓ `always_shown_in_group`    | ✓ `always_shown_in_group`       |                                                                           |
| Bio             | —      | —        | —                            | ✓                               | **New in main** — not in any XLSX column                                  |
| Sort rank/index | struct | struct   | `sort_rank` (col/row/member) | ✓ `sort_index` (u32 normalized) | v9/v10-try1: struct never populated from XLSX; main: assigned post-import |
| Members         | col    | col      | edge                         | edge `members`                  | In XLSX as column; in main as CRDT edge list                              |
| Groups          | col    | col      | edge                         | edge `groups`                   |                                                                           |

### Presenter: Computed / Edge Fields in main

| Field               | Notes                                      |
| ------------------- | ------------------------------------------ |
| `is_group`          | True if `is_explicit_group` or has members |
| `inclusive_groups`  | Transitive group membership (upward)       |
| `inclusive_members` | Transitive member list (downward)          |
| `credited_panels`   | Panels where this presenter is credited    |
| `uncredited_panels` | Panels where this presenter is uncredited  |
| `inclusive_panels`  | Union of credited + uncredited panels      |

---

## Rooms / RoomMap Sheet Columns

| XLSX Column | v9            | v10-try1      | v10-try3 | main        | Notes                                                    |
| ----------- | ------------- | ------------- | -------- | ----------- | -------------------------------------------------------- |
| Room Name   | ✓             | ✓             | ✓        | ✓           |                                                          |
| Sort Key    | ✓             | ✓             | ✓        | ✓           |                                                          |
| Long Name   | ✓             | ✓             | ✓        | ✓           |                                                          |
| Hotel Room  | ✓             | ✓             | ✓        | ✓ (edge)    |                                                          |
| Is Pseudo   | —             | —             | —        | ✓           | **New in main** — marks non-physical pseudo rooms        |
| Name Alt    | `EXTRA` const | `EXTRA` const | —        | extra field | Alternate display name; now auto-captured as extra field |
| Suffix      | `EXTRA` const | `EXTRA` const | —        | extra field | Room naming suffix; auto-captured as extra field         |
| Orig Sort   | `EXTRA` const | `EXTRA` const | —        | extra field | Original sort key before renumbering                     |
| Orig Suffix | `EXTRA` const | `EXTRA` const | —        | extra field | Original suffix before renaming                          |
| Notes       | `EXTRA` const | `EXTRA` const | —        | extra field | Facility notes                                           |

**v9/v10-try1 context:** These five extra columns had explicit `FieldDef` constants in
`room_map::EXTRA` but were not stored as `FieldDescriptor` fields — they were
recognized for import but had no backing struct field.

**main context:** These columns are now automatically captured into the CRDT `__extra`
map via the FEATURE-082 extra-field routing. They survive save/load and are included
in export. No hardcoded handling needed.

---

## PanelTypes / Prefix Sheet Columns

| XLSX Column   | v9  | v10-try1 | v10-try3 | main | Notes                                      |
| ------------- | --- | -------- | -------- | ---- | ------------------------------------------ |
| Prefix        | ✓   | ✓        | ✓        | ✓    |                                            |
| Panel Kind    | ✓   | ✓        | ✓        | ✓    |                                            |
| Color         | ✓   | ✓        | ✓        | ✓    |                                            |
| BW (Color)    | ✓   | ✓        | ✓        | ✓    |                                            |
| Hidden        | ✓   | ✓        | ✓        | ✓    |                                            |
| Is Timeline   | ✓   | ✓        | ✓        | ✓    |                                            |
| Is Private    | ✓   | ✓        | ✓        | ✓    |                                            |
| Is Break      | ✓   | ✓        | ✓        | ✓    |                                            |
| Is Workshop   | ✓   | ✓        | ✓        | ✓    |                                            |
| Is Room Hours | ✓   | ✓        | ✓        | ✓    |                                            |
| Is Café       | ✓   | ✓        | ✓        | ✓    |                                            |
| Display Name  | —   | —        | —        | —    | Not present; `Panel Kind` serves this role |

---

## HotelRoom

In all branches, HotelRoom has a single field: `hotel_room_name`. In main, this
is a first-class entity type with `FIELD_HOTEL_ROOM_NAME`; in v9/v10 it was
typically a string field on EventRoom. The main branch models the association
as an edge (`EventRoom → HotelRoom`), enabling many-to-one and one-to-many
hotel-to-event-room mappings.

---

## Spreadsheet Format Variation by Year

The XLSX files in `input/` span 2016–2026. **Note:** the historical sheets (2016–2025)
have been maintained and modernized over time — columns such as `Is Pseudo` have been
backfilled into older files as the tooling evolved. The column inventory below reflects
the current state of each file in the repo, not necessarily the column set that existed
at the time of the original convention.

Starting with 2024, each year's file includes embedded copies of the prior year's
schedule sheet (e.g., `2025 Schedule.xlsx` contains both `2025 Schedule` and
`2024 Schedule` tabs). Those embedded sheets retain their original format and were
not updated when columns changed; the tables below analyze only the current-year
sheet for each file.

### Schedule Sheet Columns by Year

Core data columns:

| Column               | 2016 | 2017 | 2018 | 2019 | 2022 | 2023 | 2024 | 2025 | 2026 | main field           |
| -------------------- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | -------------------- |
| Uniq ID              | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `uid` / `code`       |
| Old Uniq Id          | —    | —    | —    | —    | —    | ✓    | ✓    | ✓    | ✓    | ignored on import    |
| Name                 | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `name`               |
| Room                 | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | edge to EventRoom    |
| Start Time           | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | via `time_slot`      |
| Duration             | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | via `time_slot`      |
| End Time             | —    | —    | —    | ✓    | —    | —    | —    | —    | —    | `FIELD_END_TIME`     |
| Description          | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `description` (Text) |
| Prereq               | —    | —    | —    | —    | —    | —    | —    | ✓    | ✓    | `prereq`             |
| Note                 | —    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `note` (Text)        |
| Notes (Non Printing) | —    | —    | —    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | `notes_non_printing` |
| Workshop Notes       | —    | —    | —    | —    | —    | —    | ✓    | ✓    | ✓    | `workshop_notes`     |
| Power Needs          | —    | —    | —    | —    | —    | —    | ✓    | ✓    | ✓    | `power_needs`        |
| Sewing Machines      | —    | —    | —    | —    | —    | —    | ✓    | ✓    | ✓    | `sewing_machines`    |
| AV Notes             | —    | —    | —    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | `av_notes` (Text)    |
| AV Connection        | —    | —    | —    | —    | —    | —    | —    | —    | ✓    | **not in main yet**  |
| Difficulty           | —    | —    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `difficulty`         |
| Cost                 | —    | —    | —    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | `cost`               |
| Seats Sold           | —    | —    | —    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | `seats_sold`         |
| PreReg Max           | —    | —    | —    | —    | —    | —    | —    | ✓    | ✓    | `pre_reg_max`        |
| Capacity             | —    | —    | —    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | `capacity`           |
| Have Ticket Image    | —    | —    | —    | —    | —    | —    | —    | ✓    | ✓    | `have_ticket_image`  |
| SimpleTix Event      | —    | —    | —    | —    | —    | —    | —    | ✓    | ✓    | `simpletix_event`    |
| Ticket Sale          | —    | —    | —    | —    | —    | —    | ✓    | ✓    | ✓    | `ticket_url` (alias) |
| Hide Panelist        | ✓    | —    | ✓    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | `hide_panelist`      |
| Alt Panelist         | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `alt_panelist`       |
| Lstart               | ✓    | ✓    | ✓    | —    | —    | ✓    | ✓    | ✓    | ✓    | formula sidecar      |
| Lend                 | ✓    | ✓    | ✓    | —    | —    | ✓    | ✓    | ✓    | ✓    | formula sidecar      |

Transient/era-specific columns (auto-captured as extra fields if present):
`Social Hook` (2016–2017), `18+` (2016), `Premium` (2017–2018), `Changed` /
`Newly Changed` (2017–2019), `Kind` / `Day` / `12hr Time` (2019),
`Tokens` (2019), `Full` (2019).

**Presenter column prefix notation:** `G:Name` = guest, `S:Name` = staff,
`I:Name` = invited group, `J:Name` = judge (added 2024), `P:Name` = panelist,
`==Name` or `G:Base==Sub` = sub-presenter within a group. The 2019 sheet used
flat names without prefixes; all other years use the prefix notation.

### People Sheet Columns by Year

| Column                              | 2016 | 2017 | 2018 | 2019 | 2022 | 2023 | 2024 | 2025 | 2026 | main field              |
| ----------------------------------- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ----------------------- |
| Person / Name                       | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `name`                  |
| Classification                      | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `rank`                  |
| Is Group                            | ✓    | ✓    | ✓    | —    | ✓    | —    | ✓    | ✓    | ✓    | `is_explicit_group`     |
| Members                             | ✓    | ✓    | ✓    | —    | ✓    | —    | ✓    | ✓    | ✓    | edge `members`          |
| Groups                              | ✓    | ✓    | ✓    | —    | ✓    | —    | ✓    | ✓    | ✓    | edge `groups`           |
| Group Shown / Always Grouped        | —    | —    | —    | —    | ✓    | —    | ✓    | —    | ✓    | `always_grouped`        |
| Always Shown / Always Show in Group | —    | —    | —    | —    | ✓    | —    | ✓    | —    | ✓    | `always_shown_in_group` |
| Year column (e.g. `2022`)           | —    | —    | —    | —    | ✓    | ✓    | ✓    | ✓    | —    | extra field             |
| Notes                               | —    | —    | —    | —    | ✓    | —    | —    | —    | —    | extra field             |

The People sheet structure is intentionally minimal in some years (2019, 2023 have
only `Person` + `Classification`). The importer gracefully handles this since `name`
is the only required column. Year-attendance columns (`2022`–`2025`) were used for
tracking and were dropped in 2026.

### Rooms Sheet Columns by Year

| Column      | 2016 | 2017 | 2018 | 2019 | 2022 | 2023 | 2024 | 2025 | 2026 | main field        |
| ----------- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ----------------- |
| Room Name   | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `room_name`       |
| Sort Key    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `sort_key`        |
| Long Name   | ✓    | ✓    | ✓    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | `long_name`       |
| Hotel Room  | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | edge to HotelRoom |
| Is Pseudo   | ✓    | —    | ✓    | ✓    | —    | —    | ✓    | ✓    | ✓    | `is_pseudo`       |
| Name Alt    | ✓    | ✓    | ✓    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | extra field       |
| Suffix      | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | extra field       |
| Orig Sort   | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | extra field       |
| Orig Suffix | ✓    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | extra field       |
| Notes       | ✓    | —    | ✓    | —    | —    | —    | ✓    | ✓    | ✓    | extra field       |
| Orig Hotel  | —    | —    | —    | ✓    | —    | —    | —    | —    | —    | extra field       |

`Name Alt`, `Suffix`, `Orig Sort`, `Orig Suffix`, and `Notes` are present in most
years' spreadsheets but not promoted to `FieldDescriptor`. They are auto-captured
in the CRDT `__extra` map via FEATURE-082.

### PanelTypes Sheet Columns by Year

| Column        | 2016 | 2017 | 2018 | 2019 | 2022 | 2023 | 2024 | 2025 | 2026 | main field      |
| ------------- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | ---- | --------------- |
| Prefix        | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `prefix`        |
| Panel Kind    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `panel_kind`    |
| Color         | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `color`         |
| BW            | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `bw`            |
| Hidden        | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `hidden`        |
| Is Timeline   | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `is_timeline`   |
| Is Private    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `is_private`    |
| Is Break      | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `is_break`      |
| Is Workshop   | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `is_workshop`   |
| Is Room Hours | —    | —    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | ✓    | `is_room_hours` |
| Is Café       | ✓    | ✓    | ✓    | ✓    | —    | ✓    | ✓    | ✓    | —    | `is_cafe`       |
| Visible       | —    | —    | —    | —    | ✓    | —    | —    | —    | —    | extra field     |

`Is Room Hours` was added in 2018. `Is Café` is absent from 2022 and 2026 but
present in all other years. `Visible` appeared only in 2022, possibly as a trial
counterpart to `Hidden`.

---

## schedule-to-html Column Handling

`schedule-to-html` is a Perl project (Perl 5.38+, `Spreadsheet::ParseXLSX`) that
reads the same XLSX files to produce HTML schedules. It has its own independent
column registry.

### Column registration approach

Column names are hardcoded as `Readonly` constants in `Field/*.pm` modules
(no `inventory`-style auto-registration). Headers are canonicalized on read:
spaces → underscores, special chars stripped. Lookup is by canonicalized name,
case-insensitive. **Unknown columns are silently ignored.**

### Panel sheet columns recognized

| schedule-to-html constant | Equivalent main field       | Notes                              |
| ------------------------- | --------------------------- | ---------------------------------- |
| `Uniq_ID`                 | `uid` / `code`              | Required                           |
| `Name`                    | `name`                      | Required                           |
| `Room`                    | edge to EventRoom           |                                    |
| `Start_Time`              | via `time_slot`             |                                    |
| `End_Time`                | `FIELD_END_TIME`            |                                    |
| `Duration`                | via `time_slot`             |                                    |
| `Description`             | `description`               |                                    |
| `Cost`                    | `cost`                      |                                    |
| `Capacity`                | `capacity`                  |                                    |
| `Full`                    | `is_full`                   |                                    |
| `Difficulty`              | `difficulty`                |                                    |
| `Prereq`                  | `prereq`                    |                                    |
| `Note`                    | `note`                      |                                    |
| `AV_Notes`                | `av_notes`                  |                                    |
| `Kind`                    | import alias / `panel_type` |                                    |
| `Ticket_Sale`             | `ticket_url`                |                                    |
| `Hide_Panelist`           | `hide_panelist`             |                                    |
| `Alt_Panelist`            | `alt_panelist`              |                                    |
| `Room_Idx`                | sort key (computed)         | Used for room ordering in grid     |
| `Hotel_Room`              | edge to HotelRoom           |                                    |
| `Real_Room`               | (no direct equivalent)      | 2019-era column; derived room name |

**Notable omissions vs. main:** `notes_non_printing`, `workshop_notes`,
`power_needs`, `sewing_machines`, `simpletix_event`, `simpletix_link`,
`pre_reg_max`, `have_ticket_image`, `av_connection` — these columns exist in
2024–2026 spreadsheets but schedule-to-html does not read them.

### Presenter columns

Presenter columns are discovered dynamically by pattern-matching header names:

```text
G:Name          → Guest (rank 0)
J:Name          → Judge (rank 1)
S:Name          → Staff (rank 2)
I:Name          → Invited panelist (rank 3)
P:Name          → Fan panelist (rank 4)
G:Base==Sub     → sub-presenter within a group
Kind:Other      → catch-all bucket for that rank
```

No pre-registration needed; any column matching the pattern is automatically treated
as a presenter column. This matches the approach in main's XLSX importer.

### Rooms sheet columns recognized

`Room_Name` (required), `Long_Name`, `Hotel_Room`, `Sort_Key`. Extra room columns
(`Name_Alt`, `Suffix`, `Orig_Sort`, `Orig_Suffix`, `Notes`, `Is_Pseudo`) are present
in the spreadsheets but not read by schedule-to-html.

### PanelTypes sheet columns recognized

`Prefix` (required), `Panel_Kind`, `Hidden`, `Is_Break`, `Is_Cafe` / `Is_Café`,
`Is_Workshop`, `Color`, and any additional color-set columns (e.g. `BW`).
`Is_Room_Hours`, `Is_Timeline`, `Is_Private` are not read.

### People sheet

schedule-to-html does **not** read a People sheet. Presenter metadata (rank, group
membership) is derived entirely from the schedule sheet's presenter columns.

---

## Key Gaps: Fields in Older Branches Not Yet in main

The following columns exist in v9/v10-try1 `room_map::EXTRA` but are not yet
promoted to `FieldDescriptor` fields in main:

- **Name Alt** — alternate display name for a room (different from Long Name)
- **Suffix** — room naming suffix (e.g. `A`, `B` for sub-rooms)
- **Orig Sort** / **Orig Suffix** — pre-renaming identifiers used by legacy tooling

These are now captured in the `__extra` CRDT map and survive round-trips. If they
become important enough, adding a `FieldDescriptor` for them (and removing the
`ExtraFieldDescriptor` if one is declared) would promote them to first-class fields.

## Key Gaps: Spreadsheet Columns Not Yet in main

- **`AV Connection`** — new in the 2026 spreadsheet; not in any codebase version yet.
  Will be auto-captured as an extra field on import until promoted to a proper field.
- **`Prereq`** (schedule column) — first appeared in 2025 sheets. `FIELD_PREREQ` exists
  in main so it imports correctly. Absent from 2016–2024 spreadsheets.
- **`simpletix_link`** — admin portal link distinct from `simpletix_event`. In main
  as a `FieldDescriptor` but not yet in any spreadsheet column set.

## Key Gaps: Fields in main Not Carried Forward from Older Branches

- **`bio`** (Presenter) — not an XLSX column in any year; editor-only field in main.
- **`sort_index`** (Presenter) — replaces the multi-field `PresenterSortRank` struct
  that existed in v10-try3. In v9/v10-try1 the sort rank fields were populated on the
  data struct but never serialized or round-tripped through XLSX.
- **Transitive edge fields** (`inclusive_groups`, `inclusive_members`, `hotel_rooms`
  on Panel) — computed in main via the transitive edge cache; not modeled elsewhere.
