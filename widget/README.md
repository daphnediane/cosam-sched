# Widget (`widget/`)

Embeddable vanilla JS/CSS calendar widget. No framework dependencies — designed to work inside Squarespace Code Blocks.

Interactive event calendar for Cosplay America. This is a complete rewrite of the [schedule-to-html](https://github.com/daphnediane/schedule-to-html) project, adapted for modern web embedding with enhanced interactivity.

## License

Copyright (c) 2026 Daphne Pfister. Licensed under the [BSD-2-Clause License](LICENSE).

## Attribution

This project is a rewrite of and based on the original [schedule-to-html](https://github.com/daphnediane/schedule-to-html) project. Development assisted by [Windsurf](https://windsurf.com/) AI.

## Files

- `cosam-calendar.js` — calendar logic (IIFE, exposes `CosAmCalendar.init()`)
- `cosam-calendar.css` — all styling (responsive, print-friendly, scoped under `.cosam-calendar`)
- `load-json-embed.js` — loader for gzip+base64 JSON embedded via `cosam-convert --embed-as-json`
- `load-html-embed.js` — loader for widget-html format embedded via `cosam-convert` (default)
- `load-data-url.js` — loader factory for fetching JSON from a URL
- `square-template.html` — Squarespace simulation template for test page generation

## Embedding

The recommended path is to generate a self-contained HTML snippet with
`cosam-convert --export-embed` (widget-html format, default). For custom
hosting, upload `cosam-calendar.css`, `cosam-calendar.js`,
`load-data-url.js`, and `schedule.json` to a CDN or file host, then:

```html
<link rel="stylesheet" href="URL/cosam-calendar.css">
<div id="cosam-calendar"></div>
<script src="URL/cosam-calendar.js"></script>
<script src="URL/load-data-url.js"></script>
<script>
  CosAmCalendar.init({
    el: '#cosam-calendar',
    loader: CosAmCalendar.DataUrlLoader({ url: 'URL/schedule.json' })
  });
</script>
```

### `CosAmCalendar.init(opts)` options

| Option               | Description                                                                            |
| -------------------- | -------------------------------------------------------------------------------------- |
| `opts.el`            | CSS selector string or element reference for the widget root.                          |
| `opts.loader`        | Loader object with `load(rootEl): Promise<data>` and optional `watch(rootEl, reload)`. |
| `opts.data`          | Raw schedule data object — skips the loader entirely (useful for testing).             |
| `opts.stylePageBody` | Boolean — apply Squarespace-compatible body styles.                                    |

### Loader factories

| Factory                                | File                 | Description                                                                                                             |
| -------------------------------------- | -------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `CosAmCalendar.JsonEmbedLoader(opts?)` | `load-json-embed.js` | Reads gzip+base64 JSON from `#cosam-schedule-data`. `opts.dataId` overrides the element ID.                             |
| `CosAmCalendar.HtmlEmbedLoader(opts?)` | `load-html-embed.js` | Reads widget-html format: structural JSON from `#cosam-schedule-data` and panel articles from `.cosam-static-schedule`. |
| `CosAmCalendar.DataUrlLoader(opts?)`   | `load-data-url.js`   | Fetches JSON from `opts.url` (default `schedule.json`).                                                                 |

## Features

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

## Local Development

Generate a test page that simulates the widget inside the Squarespace site:

```bash
cargo run -p cosam-convert -- \
  --input "input/2026 Schedule.xlsx" \
  --export output/2026.json \
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

Rebuild all years at once:

```bash
./scripts/export-schedules.sh
```
