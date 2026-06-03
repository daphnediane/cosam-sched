# Print Layout Formats

The `schedule-layout` crate turns a schedule (the widget JSON data model) into
[Typst](https://typst.app/) source, which is then compiled to PDF. This document
describes the available formats, the shared building blocks they compose, and
the output-file conventions.

## Front-ends

Two binaries drive the layout engine; both share the same `formats::*::generate`
functions and the same filename assembly:

- **`cosam-layout`** — standalone CLI. Takes a schedule JSON plus repeatable
  per-job specs (`--format`, `--paper`, `--orientation`, `--split`, `--stem`,
  `--filter-*`). See `apps/cosam-layout`.
- **`cosam-convert --export-layout <dir>`** — runs the jobs configured in
  `config/layout-default.toml` end-to-end from an XLSX/JSON input. This is the
  path `scripts/sync-schedule.sh` uses for publishing.

Both require the `typst` binary on `PATH` to compile PDFs (use `--no-compile` /
`--typ` with `cosam-layout` to emit `.typ` source only).

## The `generate` contract

Every format exposes:

```rust
pub fn generate(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
) -> Vec<(String, String)>
```

It returns a list of `(qualifier, typ_source)` pairs. Each pair becomes one
output document. The **qualifier** is a slug the caller appends to the base
stem; an **empty** qualifier means "this format is a single document, use the
stem as-is."

### Filename assembly

The caller joins the non-empty parts with `-`:

```
{stem}-{paper_dir}-{qualifier}
```

- `stem` — from `--stem`, the output override's file stem, or a per-format
  default (`schedule`, `desc`, `workshops`, `room-signs`, `postcards`, `flyer`).
- `paper_dir` — `letter`, `legal`, `tabloid`, `super-b`, `poster`, `postcard`.
- `qualifier` — the per-pair slug (e.g. `friday`, `friday-am`, `salon-a-friday`),
  or omitted when empty.

PDFs are written under a per-paper-size subdirectory of the output dir; all
`.typ` sources share a single `typ/` subdirectory.

## Configuration (`LayoutConfig`)

| Field          | Meaning                                                           |
| -------------- | ----------------------------------------------------------------- |
| `paper`        | `Letter`, `Legal`, `Tabloid`, `SuperB`, `Poster`, `Postcard4x6`   |
| `format`       | Which builder to run (see below)                                  |
| `split_by`     | `Day` or `HalfDay` (schedule grid only)                           |
| `orientation`  | `Landscape` or `Portrait`                                         |
| `filter`       | `room_uid` (room signs), `guest_name` (postcards), `premium_only` |
| `base_font_pt` | Override body font; defaults to the paper's base size             |
| `grid_font_pt` | Override grid event-text size                                     |

## Shared building blocks (`blocks/`)

- **`typst_gen::preamble`** — page setup, brand color variables, font specs.
- **`banner`** — the branded header bar and footer:
  - `page_header(left, right)` — static labels (single page documents).
  - `page_header_running(right)` / `page_header_running_split(left, right)` —
    per-page running headers resolved from `<…>` metadata markers via `context`
    + `query` (read-only, so layout converges). The `_split` variant fills both
    sides — used by room signs for room (left) / day (right).
  - `page_footer(timestamps, site)` — timestamps, centered `Page N of M`, site.
  - `footer_timestamps(modified, generated)` — formats the modified/generated
    stamps in local time, mirroring the widget footer.
- **`grid`** — `render_schedule_grid` plus `GridRenderConfig` (font scaling,
  per-room column highlight, corner label, optional max height).
- **`panels::render_time_grouped_panels`** — the description column flow:
  time-slot headings, per-panel accent bar, room/time/cost, credits, workshop
  notices, prerequisites, and part/rerun cross-references. Uses sticky headings
  and label-`query`-based "(continued)" headers across column/page breaks.

### Grid + column mixing (`place` + `colbreak`)

The flyer and room signs share a technique for setting a schedule grid beside a
reflowing description column flow. The grid is `place`d (which reserves no
space) over a box covering the left half of the columns, then a full-width
`#columns(N)` block emits leading `#colbreak()`s to skip the columns the grid
covers. Descriptions therefore start in the right-hand columns on the first page
and overflow continues full-width on following pages — unlike a fixed
`#grid(columns: (X%, 1fr))`, which cannot let text reflow past the grid.

## Formats

### `schedule`

The time × room grid. One document **per day** (`SplitMode::Day`) or **per
half-day** (`SplitMode::HalfDay`); qualifier is the day/half slug. Landscape on
tabloid is the common choice.

### `descriptions`

Full panel-description listing in a multi-column flow. One document **per day**;
qualifier is the day slug. Column count from `description_columns` (see table).

### `workshops_listing`

Like `descriptions` but filtered to workshop/premium panels and spanning **all
days in one document** (empty qualifier), with a day heading inserted when the
date changes. Returns empty when there are no workshops.

### `room_signs`

Door signs for every room. A **single multi-page document** (empty qualifier).
Each room/day starts on a fresh page laid out like the flyer's first page: the
full schedule grid is `place`d over the left half with this room's column
highlighted and the day label in the corner, while the room's own descriptions
flow through the right-hand columns and overflow full-width onto following
pages. Every page carries a running header (room left, day right, from
`<room-sign>` markers) and the timestamp/page-number footer. Honors
`filter.room_uid` to emit a single room's signs.

### `flyer`

Double-sided per-day booklet, **one multi-day document** (empty qualifier). Each
day begins on an odd page; the day's grid occupies the left half of the first
page (descriptions flow through the remaining columns and onto following
full-width pages), with a running per-day header and the timestamp/page-number
footer. Column count from `flyer_columns` (4 on letter, 6 on legal+, landscape).

### `guest_postcards`

A 4×6 postcard **per presenter per half-day** (qualifier
`{guest-slug}-{half-slug}`) listing only that presenter's panels. Limited to
guest/judge/staff/invited ranks (priority ≤ 3). Honors `filter.guest_name`.

## Column counts by paper

`description_columns` (descriptions, workshops, room signs):

| Paper            | Landscape | Portrait |
| ---------------- | --------- | -------- |
| Letter           | 4         | 3        |
| Legal            | 4         | 3        |
| Tabloid / SuperB | 5         | 4        |
| Poster           | 5         | 5        |
| Postcard 4×6     | 1         | 1        |

`flyer_columns` (flyer; total must be even-friendly for the half-split):

| Paper                             | Landscape | Portrait |
| --------------------------------- | --------- | -------- |
| Letter                            | 4         | 2        |
| Legal / Tabloid / SuperB / Poster | 6         | 4        |
| Postcard 4×6                      | 2         | 2        |

For the half-page grid formats (flyer, room signs) the grid spans the left
`ceil(total / 2)` columns; descriptions take the rest.
