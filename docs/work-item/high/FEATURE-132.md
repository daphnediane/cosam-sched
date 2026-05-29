# FEATURE-132: HTML-embedded schedule format (widget-html)

## Summary

Add a hybrid "widget-html" format where structural schedule data (meta, rooms, panelTypes, timeline, presenters) is kept as a compact JSON block but panels are rendered as semantic HTML, enabling SEO crawlability and a no-JS fallback while preserving full widget functionality.

## Status

In Progress

## Priority

High

## Description

The current embed format stores all schedule data as a gzip+base64 JSON blob that is invisible to search engines and shows only "Please enable JavaScript" without JS. The widget-html format replaces the panels JSON array with semantic HTML elements — each panel as an `<article class="cosam-panel">` with `data-*` attributes for machine-readable scalar fields and visible HTML text for names, descriptions, and credits. The remaining structural data (rooms, panelTypes, presenters, timeline) is kept in a compact `<script type="application/json" data-cosam="schedule">` block.

## Implementation Details

- **Phase 1**: `docs/widget-html-format.md` — full format spec
- **Phase 2** ✓: `apps/cosam-convert/src/static_html.rs` — Rust HTML generator
- **Phase 3** ✓: `apps/cosam-convert/src/embed.rs` — dual-format: `--embed-as-json` (default, gzip+base64) and `--embed-as-html` (widget-html)
- **Phase 4**: `widget/cosam-calendar.js` — `_parseHtmlData()` + auto-detection
- **Phase 5**: Build, test, verify
- **Phase 6**: Remove old embed format infrastructure and dual-format flags

## Acceptance Criteria

- `embed.html` contains plain-text panel names/descriptions readable by crawlers
- Widget renders the full interactive view when JS is enabled
- A readable list of panels is shown when JS is disabled
- All widget features work (filtering, presenter lookup, starring, QR sharing, print)
