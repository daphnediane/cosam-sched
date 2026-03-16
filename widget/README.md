# Widget (`widget/`)

Interactive event calendar for Cosplay America. This is a complete rewrite of the [schedule-to-html](https://github.com/daphnediane/schedule-to-html) project, adapted for modern web embedding with enhanced interactivity.

## License

Copyright (c) 2026 Daphne Pfister. Licensed under the [BSD-2-Clause License](LICENSE).

## Attribution

This project is a rewrite of and based on the original [schedule-to-html](https://github.com/daphnediane/schedule-to-html) project. Development assisted by [Windsurf](https://windsurf.com/) AI.

## Files

- `cosam-calendar.js` — calendar logic (IIFE, exposes `CosAmCalendar.init()`)
- `cosam-calendar.css` — all styling (responsive, print-friendly, scoped under `.cosam-calendar`)
- `embed.html` — demo/test page
- `sample-data.json` — sample schedule data for testing

## Embedding

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

```
cd widget
python3 -m http.server 8080
# Open http://localhost:8080/embed.html
```
