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
- `widget/` — embeddable JavaScript/CSS calendar widget
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
  --output widget/2026.json \
  --title "Cosplay America 2026"
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
- `embed.html` — demo/test page
- `sample-data.json` — sample schedule data for testing

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

```bash
cd widget
python3 -m http.server 8080
# Open http://localhost:8080/embed.html
```
