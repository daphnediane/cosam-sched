# Print Layout Formats

The `schedule-layout` crate turns a schedule (the widget JSON data model) into
[Typst](https://typst.app/) source, which is then compiled to PDF. Every print
artifact — schedule grids, description booklets, workshop listings, room signs,
guest lists — is one configuration of a **single** builder, selected by a
[`ContentMode`](#contentmode--splits) (what to draw) plus a section/time split
(how to break it into pages). This document describes that builder, its
configuration, and the output-file conventions.

## Front-ends

Two binaries drive the layout engine; both call `document::generate` and share
the same filename assembly:

- **`cosam-layout`** — standalone CLI. Takes a schedule JSON plus repeatable
  per-job specs (`--content`, `--paper`, `--orientation`, `--split`,
  `--panel-filter`, `--footer`, `--columns`, `--stem`, `--header-text`), jobs
  separated by a bare `--`. See `apps/cosam-layout`.
- **`cosam-convert --export-layout <dir>`** — runs the jobs configured in
  `config/layout.toml` (falling back to the embedded `config/layout-default.toml`)
  end-to-end from an XLSX/JSON input. This is the path `scripts/sync-schedule.sh`
  uses for publishing.

Both require the `typst` binary on `PATH` to compile PDFs (`cosam-layout --typ`
also writes `.typ`; `--no-compile` emits `.typ` only). For reproducible test
output, `cosam-convert --stable-timestamps` pins the generated time to the
modified time so the footer no longer varies per run. See `.claude/rules/layout.md`
for the testing workflow.

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

- `stem` — from `--stem`, the output override's file stem, or a per-content
  default (`flyer`, `schedule`, `desc`, `list`).
- `paper_dir` — `letter`, `legal`, `tabloid`, `super-b`, `poster`, `postcard`.

PDFs are written under a per-paper-size subdirectory of the output dir; all
`.typ` sources share a single `typ/` subdirectory.

## Configuration (`LayoutConfig`)

| Field          | Meaning                                                             |
| -------------- | ------------------------------------------------------------------- |
| `paper`        | `Letter`, `Legal`, `Tabloid`, `SuperB`, `Poster`, `Postcard4x6`     |
| `content`      | [`ContentMode`](#contentmode--splits) — what to draw + how to split |
| `panel_filter` | `All`, `Workshops`, `Premium`                                       |
| `orientation`  | `Landscape` or `Portrait`                                           |
| `color_mode`   | Color or black-and-white output                                     |
| `columns`      | Override the per-content/per-paper default column count             |
| `footer`       | `Full` (timestamps + page number + site), `TimestampOnly`, `None`   |
| `double_sided` | Pad each section onto an odd page (booklet printing)                |
| `header_text`  | Optional banner label (left for 1-D splits, right for no split)     |
| `base_font_pt` | Override body font; defaults to the paper's base size               |
| `grid_font_pt` | Override grid event-text size                                       |

### Style options

These per-job keys are opt-in; unset, every job renders exactly as before.
Colors accept hex (`#f2f2f2`), `luma(95%)`, or a named Typst color (`white`,
`silver`, …); lengths accept `<number><unit>`. Invalid values fall back to the
default rather than emitting broken Typst.

| Field             | Meaning                   | Default       |
| ----------------- | ------------------------- | ------------- |
| `page_fill`       | Page background           | white         |
| `empty_grid_fill` | Empty grid-cell fill      | `luma(245)`   |
| `cards`           | Cards vs. left accent bar | `false`       |
| `card_fill`       | Card background           | `white`       |
| `column_gap`      | Body-column gutter        | `0.2in`       |
| `card_gap`        | Gap between cards         | column gutter |

Set `empty_grid_fill` when `page_fill` is tinted, so empty cells stay distinct
from the background. `card_gap` accepts a length or the literal `"column"`
(match the column gutter) and applies only when `cards` is set; the default
(bar) style keeps Typst's block spacing between panels.

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

## Crate modules

- **`config`** — `LayoutConfig` and the paper / orientation / content / split /
  footer / filter enums.
- **`timegrid`** — time-grid *computation*: `GridLayout`, `TimeSlot`, `GridCell`
  (time slots, room columns, cell spans). No Typst.
- **`geometry`** — page/banner/footer dimension constants and a `#let` emitter;
  the preamble defines `_content-top`, `_page-edge`, `_col-gutter`,
  `_banner-inset`, etc. (`_content-top = _page-edge + _banner-height +
  _banner-gap`), and the generators reference them instead of inline literals.
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
  fill override). Font sizes come from the global `#let`s emitted by `fonts`.
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
