# Print Layout Formats

The `schedule-layout` crate turns a schedule (the widget JSON data model) into
[Typst](https://typst.app/) source, which is then compiled to PDF. Every print
artifact — schedule grids, description booklets, workshop listings, room signs,
guest lists — is one configuration of a **single** builder, selected by a
[`ContentMode`](#contentmode--splits) (what to draw) plus a section/time split
(how to break it into pages). This document describes that builder, its
configuration, and the output-file conventions.

## Front-end

`cosam-convert` drives the layout engine end-to-end from an XLSX/JSON input,
calling `document::generate` and handling the filename assembly. It has two
modes:

- **`--export-layout <dir>`** — renders the jobs configured in
  `config/layout.toml` (falling back to the embedded `config/layout-default.toml`)
  into per-paper-size subdirectories of `<dir>`. This is the path
  `scripts/sync-schedule.sh` uses for publishing.
- **`--layout.<key>[=<value>]` … `--export-layout <file>`** — defines a single
  layout job on the command line and renders it to `<file>`. Keys mirror the
  TOML field names (`--layout.paper=letter`, `--layout.content=grid_only`,
  `--layout.cards`, …); repeat `--layout.import=<preset>` to stack presets, which
  resolve against the presets from **both** `layout-default.toml` and the user's
  `layout.toml` (the user's win on name clashes). A boolean key with no `=value`
  (e.g. `--layout.cards`) means `true`. When `<file>` has an extension it is
  written verbatim; otherwise its stem and parent directory seed the output
  name. `--layout-config <file>`, `--default-layouts`, and `--default` discard
  any accumulated `--layout.*` and revert to the TOML jobs.

It requires the `typst` binary on `PATH` to compile PDFs; the `.typ` source is
written either way. For reproducible test output, `cosam-convert
--stable-timestamps` pins the generated time to the modified time so the footer
no longer varies per run. See `.claude/rules/layout.md` for the testing
workflow.

## The `generate` contract

The builder exposes a single entry point:

```rust
pub fn generate(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
) -> Vec<(String, String)>
```

It returns **one** `(qualifier, typ_source)` pair — the whole multi-section
document (all days/rooms/presenters, separated by page breaks) lives in a single
file — or an empty vec when no panels match. The qualifier is always empty; pages
are extracted from the compiled PDF afterward if individual sheets are needed.

### Filename assembly

The caller joins the non-empty parts with `-`:

```text
{stem}-{paper_dir}
```

- `stem` — the job's `stem` (TOML jobs), or the `--export-layout` path's file
  stem (a command-line `--layout.*` job).
- `paper_dir` — `letter`, `legal`, `tabloid`, `super-b`, `poster`, `postcard`,
  `quarter`.

For TOML jobs, PDFs are written under a per-paper-size subdirectory of the output
dir and all `.typ` sources share a single `typ/` subdirectory. A command-line
`--layout.*` job writes its PDF and `.typ` next to the `--export-layout` path; if
that path has an extension, the PDF is written to it verbatim.

## Configuration (`LayoutConfig`)

| Field             | Meaning                                                                                                                                                                                                                  |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `paper`           | `Letter`, `Legal`, `Tabloid`, `SuperB`, `Poster`, `Postcard4x6` (4×6), `Quarter` (4.25×5.5)                                                                                                                              |
| `format`          | `Typst` (default, → PDF) or `Idml` (Adobe InDesign; see [IDML export](#idml-export))                                                                                                                                     |
| `content`         | [`ContentMode`](#contentmode--splits) — what to draw + how to split                                                                                                                                                      |
| `panel_filter`    | `All`, `Workshops`, `Premium`                                                                                                                                                                                            |
| `include_private` | Render the private view: private panels + unlisted presenters in per-presenter splits (byline credits stay credited-only). Default `false`                                                                               |
| `orientation`     | `Landscape` or `Portrait`                                                                                                                                                                                                |
| `color_mode`      | Color or black-and-white output                                                                                                                                                                                          |
| `columns`         | Override the per-content/per-paper default column count                                                                                                                                                                  |
| `footer`          | `Full` (timestamps + page number + site), `TimestampOnly`, `SectionPages` (per-section `Label: Page X of Y`), `None`                                                                                                     |
| `matching_only`   | Presenter×day grids: show only a guest's scheduled days (default `true`); `false` also emits their off days (full grid, none highlighted)                                                                                |
| `double_sided`    | Pad each section onto an odd page (booklet printing)                                                                                                                                                                     |
| `header_text`     | Optional banner label (left for 1-D splits, right for no split)                                                                                                                                                          |
| `base_font_pt`    | Override body font; defaults to the paper's base size                                                                                                                                                                    |
| `grid_font_pt`    | Override grid event-text size                                                                                                                                                                                            |
| `fit_grid`        | Fit a full-page grid onto one page when it would overflow: compress rows so it fits one page. Default on for `grid_only`, off otherwise; `false` flows/paginates naturally |
| `fit_text`        | How panel text fits a compressed cell: `shrink`/`all` (default — scale the font down so all content fits), `name` (keep the name readable, hide overflowing secondary lines from the bottom), `clip`/`none` (no resize; clip) |
| `show_duration`   | Show each event's duration line in grid cells. Default off on compact (4×6/quarter) papers (the time column conveys it); a panel split across a time-split boundary always shows its full duration |
| `show_cost`       | Show event cost (e.g. a workshop price) in grid cells (default on; guest schedules usually set `false`) |
| `logo`            | Banner logo alias/filename from `[logos]`; `none` suppresses it (default `brand`)                                                                                                                                        |
| `banner_text_pt`  | Override banner label size (default 28pt, 13pt on compact papers)                                                                                                                                                        |
| `banner_size`     | Banner bar height: `auto` (compact on 4×6/quarter, full otherwise), `compact`, `full`, a length (`0.5in`), or a `%` of page height (`4%`)                                                                                |
| `footer_size`     | Reserved footer height, same options as `banner_size`                                                                                                                                                                    |

### Style options

These per-job keys are opt-in; unset, every job renders exactly as before.
Colors accept hex (`#f2f2f2`), `luma(95%)`, or a named Typst color (`white`,
`silver`, …); lengths accept `<number><unit>`. Invalid values fall back to the
default rather than emitting broken Typst.

| Field             | Meaning                   | Default       |
| ----------------- | ------------------------- | ------------- |
| `page_fill`       | Page background           | white         |
| `empty_grid_fill` | Empty grid-cell fill      | `luma(245)`   |
| `dim_conflict`    | Fade conflicting panels   | `false`       |
| `cards`           | Cards vs. left accent bar | `false`       |
| `card_fill`       | Card background           | `white`       |
| `column_gap`      | Body-column gutter        | `0.2in`       |
| `card_gap`        | Gap between cards         | column gutter |

Set `empty_grid_fill` when `page_fill` is tinted, so empty cells stay distinct
from the background. `dim_conflict` applies only to presenter schedules: a
non-highlighted panel whose time overlaps one of the guest's own (highlighted)
panels is faded, surfacing the "you're booked elsewhere" conflicts the way the
old schedule-to-html did. `card_gap` accepts a length or the literal `"column"`
(match the column gutter) and applies only when `cards` is set; the default
(bar) style keeps Typst's block spacing between panels.

### Micro font

| Field          | Meaning                                                      | Default              |
| -------------- | ------------------------------------------------------------ | -------------------- |
| `micro`        | Font family for small text; `none` disables the substitution | brand `micro`        |
| `micro_style`  | Micro font style                                             | brand `micro_style`  |
| `micro_weight` | Micro font weight                                            | brand `micro_weight` |
| `micro_max_pt` | Size (pt) below which text switches to the micro font        | `8.0`                |

The micro font keeps small grid/description text legible where the body face
gets spindly. It is applied via a `context`-gated `show` rule so it fires even
for sizes computed at layout time — including the per-cell font condensing that
`fit_grid` performs.

### QR code

| Field           | Meaning                                                          | Default   |
| --------------- | ---------------------------------------------------------------- | --------- |
| `qr_url`        | URL encoded as a QR code in the bottom-right corner of each page | (omitted) |
| `qr_msg`        | Caption above the QR (heading font); the URL shows below         | (none)    |
| `qr_size`       | QR code size                                                     | `0.75in`  |
| `qr_caption_pt` | Caption text size                                                | `9pt`     |
| `qr_url_pt`     | URL text size                                                    | `7pt`     |

The QR keys take effect only when `qr_url` is set.

### `ContentMode` + splits

`ContentMode` chooses what each section draws; a `SectionSplit`
(`Room` / `Presenter`) and `TimeSplit` (`Day` / `HalfDay`) choose how the content
is broken into sections (one page-break-separated section per split value):

| `ContentMode`     | Draws                                                                  |
| ----------------- | ---------------------------------------------------------------------- |
| `Both`            | schedule grid on the left half + descriptions in the remaining columns |
| `GridOnly`        | full-width schedule grid                                               |
| `DescriptionOnly` | multi-column panel descriptions                                        |
| `PanelList`       | compact name + time + room list                                        |

Grid-bearing content (`Both`, `GridOnly`) requires a time split and renders the
**full day's** grid per section: a `Room` section highlights its room column, a
`Presenter` section highlights that guest's own cells. Text-only content
(`DescriptionOnly`, `PanelList`) may use either, both, or no split (`None` =
one continuous flow).

The former hard-coded formats are now these recipes (see
`config/layout-default.toml`):

| Former format     | `content`          | `split`     | notes                    |
| ----------------- | ------------------ | ----------- | ------------------------ |
| schedule grid     | `grid_only`        | `half_day`  |                          |
| descriptions      | `description_only` | `day`       |                          |
| workshops listing | `description_only` | `none`      | `panel_filter=workshops` |
| room signs        | `both`             | `room_day`  |                          |
| flyer             | `both`             | `day`       | `double_sided=true`      |
| guest postcards   | `panel_list`       | `presenter` | `paper=postcard`         |

## IDML export

The `idml` module (feature-gated behind the `idml` crate feature) emits an Adobe
**IDML** package (`.idml`) — a ZIP of XML parts InDesign can open and hand-edit —
as an alternative to the Typst/PDF pipeline. Select it per job with
`format = "idml"` (TOML) or `--layout.format=idml` (CLI). The package is written
as `<stem>.idml` instead of a `.typ`/`.pdf` pair; no `typst` binary is needed.

Build with the feature enabled (it is **not** in the default build, so the
standard binary needs no IDML/`zip` dependencies):

```bash
cargo run --release -p cosam-convert --features idml -- \
  --input "input/2026 Schedule.xlsx" \
  --layout.content=description_only --layout.split=none \
  --layout.format=idml --export-layout scratch/sched.idml
```

A job that requests `format=idml` from a binary built without the feature prints
a warning and is skipped.

### Scope (v1) and limitations

- Renders a **threaded text listing**: panels grouped by day → time slot, flowed
  through linked text frames across as many pages as needed. `panel_list` content
  is compact (name / time / room); other modes are full (title, time · room(s) ·
  presenters, description).
- The schedule **grid** is not yet emitted — `grid_only` / `both` produce only
  the text portion. The grid maps onto an InDesign `<Table>` (built from
  `GridLayout`) and is planned as a follow-up.
- Section splits beyond day/time (room/presenter) are not yet specialized.
- **Fonts** are referenced, not embedded (IDML never embeds fonts). Headings use
  the brand heading font; set `heading_idml_style` / `body_idml_style` in
  `brand.toml` to the font's exact InDesign style name (e.g. Trend Sans's
  `"One"`) — these override the numeric `*_weight` mapping used for Typst. When a
  font is not installed, InDesign substitutes it on open (a normal warning, not a
  document error).
- Page count is estimated heuristically (with slack), not driven by true text
  overflow.

## Crate modules

- **`config`** — `LayoutConfig` and the paper / orientation / content / split /
  footer / filter enums.
- **`timegrid`** — time-grid *computation*: `GridLayout`, `TimeSlot`, `GridCell`
  (time slots, room columns, cell spans). No Typst.
- **`geometry`** — page/banner/footer dimension constants and a `#let` emitter;
  the preamble defines `_content-top`, `_page-edge`, `_col-gutter`,
  `_banner-inset`, etc. (`_content-top = _page-edge + _banner-height +
  _banner-gap`), and the generators reference them instead of inline literals.
  The banner is a fixed-height bar (`_banner-height`), so the reserved margin and
  the visible colored bar always agree. `typst_lets` takes independent
  banner/footer compact flags and optional explicit heights, selected per job by
  `banner_size`/`footer_size` (compact defaults on 4×6/quarter papers).
- **`fonts`** — font sizes and typeface specs plus a `#let` emitter. Typefaces
  are dictionaries (`_body-font`, `_heading-font`, `_banner-font`) spread into
  text calls; grid text-role sizes (`_name_size`, …) are emitted only when a
  grid is drawn, the description size (`_desc-secondary-size`) only when panel
  text is drawn, both for the side-by-side layout.
- **`typst_gen`** — `preamble` (page setup, geometry/font `#let`s, brand colors),
  `escape_typst`, and the day-label helpers.
- **`document`** — the unified builder (`generate`): assembles the preamble,
  footer, running header, and one section per split value.

### Building blocks (`blocks/`)

- **`banner`** — the branded header bar and footer:
  - `page_header(left, right)` — static labels (no split).
  - `page_header_running(right)` / `page_header_running_split(left, right)` —
    per-page running headers resolved from `<section>` metadata markers via
    `context` + `query` (read-only, so layout converges). The `_split` variant
    fills both sides — entity (left) / day (right) for 2-D splits.
    `page_header_running` auto-shrinks its label (via `layout` + `measure`) so a
    long running value (e.g. a guest name on a postcard) stays on one line.
  - `page_footer(timestamps, site)` — timestamps, centered `Page N of M`, site;
    `page_footer_timestamps_only` for `FooterMode::TimestampOnly`.
  - `footer_timestamps(modified, generated)` — formats the modified/generated
    stamps in local time, mirroring the widget footer.
- **`grid`** — `render_schedule_grid` plus `GridRenderConfig` (per-room column
  highlight, per-panel highlight set, corner label, optional max height, empty-cell
  fill override, `show_cost`/`show_duration`, `fit_text`). Font sizes come from the
  global `#let`s emitted by `fonts`. When the grid is compressed to one page
  (`fit_to_page`), `fit_text` decides how a cell's text fits its shortened row:
  `Shrink` scales the whole cell font; `Name` keeps the name and drops overflowing
  secondary lines bottom-up (a Typst loop measuring each appended line); `Clip`
  leaves it to the cell's `clip`. The corner cell is labelled "Time" (see
  `document::TIME_CORNER_LABEL`).
- **`panels`** — `render_time_grouped_panels` (the description column flow:
  time-slot headings, the per-panel left accent bar or bordered card (`PanelStyle`),
  room/time/cost, credits, workshop notices, prerequisites, part/rerun
  cross-references; sticky headings and label-`query`-based "(continued)" headers
  across breaks) and `render_panel_list` (the compact `PanelList` flow — one
  shared grid of `time range | accent bar | name | room`, with day headings as
  full-width spanning rows so every name aligns).

### Grid + column mixing (`place` + `colbreak`)

`Both` content sets a schedule grid beside a reflowing description column flow.
The grid is `place`d (which reserves no space) over a box covering the left half
of the columns, then a full-width `#columns(N)` block emits leading `#colbreak()`s
to skip the columns the grid covers. Descriptions therefore start in the
right-hand columns on the first page and overflow continues full-width on
following pages — unlike a fixed `#grid(columns: (X%, 1fr))`, which cannot let
text reflow past the grid.

## Column counts by paper

`description_columns` (`DescriptionOnly`, `PanelList`, and the descriptions half
of `Both` when no override is given):

| Paper            | Landscape | Portrait |
| ---------------- | --------- | -------- |
| Letter           | 4         | 3        |
| Legal            | 4         | 3        |
| Tabloid / SuperB | 5         | 4        |
| Poster           | 5         | 5        |
| Postcard 4×6     | 1         | 1        |

`flyer_columns` (`Both`; total must be even-friendly for the half-split):

| Paper                             | Landscape | Portrait |
| --------------------------------- | --------- | -------- |
| Letter                            | 4         | 2        |
| Legal / Tabloid / SuperB / Poster | 6         | 4        |
| Postcard 4×6                      | 2         | 2        |

For `Both` content the grid spans the left `ceil(total / 2)` columns;
descriptions take the rest.
