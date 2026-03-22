# cosam_sched

Interactive event calendar for Cosplay America. This is a complete rewrite of the [schedule-to-html](https://github.com/daphnediane/schedule-to-html) project, adapted for modern web embedding with enhanced interactivity.

## License

Copyright (c) 2026 Daphne Pfister. Licensed under the [BSD-2-Clause License](LICENSE).

## Attribution

This project is a rewrite of and based on the original [schedule-to-html](https://github.com/daphnediane/schedule-to-html) project. Development assisted by [Windsurf](https://windsurf.com/) AI.

Two components:

## Repository Layout

- `crates/schedule-core/` — shared Rust library for schedule data models and import/export logic
- `apps/cosam-editor/` — Rust GUI application (`cosam-editor`)
- `apps/cosam-convert/` — Rust CLI application (`cosam-convert`)
- `widget/` — embeddable JavaScript/CSS calendar widget (source only)
- `output/` — generated files: JSON, embed HTML, test pages (gitignored)
- `docs/work-plan/` — individual work plan items
- `docs/WORK_PLAN.md` — generated combined work plan
- `scripts/` — project scripts, including work plan generation/formatting tools

Rust code is organized as a top-level Cargo workspace.

## Rust Apps + Core Library

Rust project with two binaries that share the same data import/export pipeline:

- `cosam-editor` — GUI editor for interactive schedule editing
- `cosam-convert` — command-line converter for XLSX/JSON to schedule JSON output

### Rust CLI usage

```bash
cargo run -p cosam-convert -- \
  --input path/to/schedule.xlsx \
  --export output/2026.json \
  --title "Cosplay America 2026"
```

#### Output Options

The CLI provides two different output options:

- `--output` / `-o` - **Full/Private Schedule**: Saves the complete schedule data including all panels, timeline entries, hidden types, and internal metadata. This is the raw data format used for editing and internal processing.
- `--export` / `-e` - **Public Schedule**: Exports a filtered, public-facing version with hidden panels removed, presenter privacy respected, and optimized for display/public consumption.

**Use `--output` for:**

- Full data backups
- Editor input files
- Internal processing
- When you need all timeline entries and hidden panels

**Use `--export` for:**

- Public display
- Widget data
- Web embedding
- When you need only visible, public-safe content

#### Multiple outputs with different settings

The new CLI supports multiple output files with different settings per output:

```bash
# Generate both minified and unminified versions
cargo run -p cosam-convert -- \
  --input schedule.xlsx \
  --minified --export-embed min.html \
  --no-minified --export-embed max.html

# Generate both full and public versions
cargo run -p cosam-convert -- \
  --input schedule.xlsx \
  --title "Cosplay America 2026 Schedule" \
  --output full-schedule.json \
  --export public.json

# Generate multiple outputs with different titles
cargo run -p cosam-convert -- \
  --input schedule.xlsx \
  --title "Public Schedule" --export public.json \
  --title "Internal Schedule" --output internal.json
```

#### Validation mode

Check schedule validity without generating output:

```bash
cargo run -p cosam-convert -- \
  --input schedule.xlsx \
  --check
```

#### Builtin resources

Use `--builtin-*` options to specify built-in CSS, JS, and templates:

```bash
# Use all builtin resources
cargo run -p cosam-convert -- \
  --input schedule.xlsx \
  --builtin \
  --export-embed embed.html

# Use builtin CSS only
cargo run -p cosam-convert -- \
  --input schedule.xlsx \
  --builtin-css \
  --widget-js custom.js \
  --export-embed embed.html

# Reset to defaults
cargo run -p cosam-convert -- \
  --input schedule.xlsx \
  --default \
  --export-embed embed.html
```

### Build for macOS and Windows

Use the helper script:

```bash
./scripts/build-rust-targets.sh
```

Prerequisites for Windows cross-build on macOS:

```bash
rustup target add x86_64-pc-windows-gnu
sudo port install mingw-w64
```

### Spreadsheet Format

Same format as [schedule-to-html](https://github.com/daphnediane/schedule-to-html):

- **Schedule sheet** (main): Uniq_ID, Name, Description, Start_Time, End_Time, Duration, Room, Cost, Difficulty, Capacity, Kind, Note, Prereq, Ticket_Sale, Full, plus presenter columns (g1, g2, j1, s1, p1, etc.)
- **Rooms sheet**: Sort_Key, Room_Name, Hotel_Room, Long_Name
- **PanelTypes sheet**: Prefix, Panel_Kind, Hidden, Is_Break, Is_Café, Is_Workshop, Color

## Widget (`widget/`)

Embeddable vanilla JS/CSS calendar widget. No framework dependencies — designed to work inside Squarespace Code Blocks.

### Files

- `cosam-calendar.js` — calendar logic (IIFE, exposes `CosAmCalendar.init()`)
- `cosam-calendar.css` — all styling (responsive, print-friendly, scoped under `.cosam-calendar`)
- `square-template.html` — Squarespace simulation template for test page generation

### Embedding

Upload `cosam-calendar.css`, `cosam-calendar.js`, and your `schedule.json` to a CDN or file host, then:

```html
<link rel="stylesheet" href="URL/cosam-calendar.css">
<div id="cosam-calendar"></div>
<script src="URL/cosam-calendar.js"></script>
<script>
  CosAmCalendar.init({
    el: '#cosam-calendar',
    dataUrl: 'URL/schedule.json'
  });
</script>
```

### Features

- **Two views**: switchable grid (rooms × time slots) and list (card-based by time)
- **Day tabs**: navigate between convention days
- **Filters**: room, panel type/kind, cost (free/paid/workshop), presenter, text search
- **"My Schedule" bookmarks**: star events, stored in localStorage + shareable via URL hash
- **Print support**: print button, clean `@media print` styles, starred-only option
- **Event detail modal**: full description, presenters, cost, prerequisites, notes, ticket link
- **Responsive**: list view on mobile, grid on desktop
- **Color-coded**: panel types distinguished by color from PanelTypes sheet
- **Theming**: CSS custom properties (accent color, fonts, etc.) for easy customization
- **No conflicts**: all styles scoped under `.cosam-calendar`

### Local Development

Generate a test page that simulates the widget inside the Squarespace site:

```bash
cargo run -p cosam-convert -- \
  --input "input/2026 Schedule.xlsx" \
  --output output/2026.json \
  --export-embed output/2026-embed.html \
  --export-test output/2026-test.html \
  --title "Cosplay America 2026 Schedule"
# Open output/2026-test.html in a browser
```

To iterate on widget CSS/JS without rebuilding the Rust binary:

```bash
cargo run -p cosam-convert -- \
  --input "input/2026 Schedule.xlsx" \
  --export-test output/2026-test.html \
  --widget widget/ \
  --no-minified \
  --title "Cosplay America 2026 Schedule"
```

#### Multiple outputs in a single command

The updated export script now uses the new multi-output functionality to generate all files in a single command:

```bash
# Old way (multiple calls)
./scripts/export-schedules.sh

# New way (single call with multiple outputs)
cargo run -p cosam-convert -- \
  --input schedule.xlsx \
  --title "Cosplay America 2026 Schedule" \
  --output full-schedule.json \
  --export public.json \
  --export-embed embed.html \
  --export-test test.html \
  --style-page \
  --export-embed style-embed.html \
  --export-test style-page.html
```

Rebuild all years at once:

```bash
./scripts/export-schedules.sh
```
