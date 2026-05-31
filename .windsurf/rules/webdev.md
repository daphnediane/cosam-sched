---
description: Frontend development practices for the calendar widget
trigger: glob
globs: widget/**/*.css,widget/**/*.js
---

# Frontend Development Practices

## Build System

Uses **esbuild** via `widget/build.mjs`.

```bash
npm run build   # One-shot
npm run watch   # Rebuild on change
npm run serve   # localhost:8000
```

**Outputs:** `cosam-calendar.min.js/css`, `load-{json,html,data}.min.js`

**Deps:** `qrcode` (runtime), `esbuild` (dev only) — keep minimal.

## Security

### XSS Prevention

- **Never use `innerHTML`** with schedule JSON — use `textContent` and `document.createElement()`

### URL Handling

- Validate URLs; use `URL()` constructor; never `eval()` or `new Function()`

### CSP & Data

- No inline event handlers, no `javascript:` URLs
- Validate localStorage on read; don't store sensitive data
- QR codes: only encode same-origin URLs, never user-controlled text

## JavaScript

- ES6+, IIFE format, strict mode, prefer `const`/`let`
- Use `document.createElement()`, event delegation, scoped queries (`rootEl.querySelector()`)
- Validate JSON structure, handle missing fields defensively

## CSS

- Scope all styles under `.cosam-calendar`
- Mobile-first, semantic class names, CSS Grid
- Print-friendly `@media print` styles, WCAG contrast

## Testing

### Quick Start

```bash
cargo run -p cosam-convert -- \
  --input "input/2026 Schedule.xlsx" \
  --export-test output/2026-test.html \
  --widget widget/ --no-minified \
  --title "Cosplay America 2026 Schedule"
```

Open `output/2026-test.html` or use `browser_preview` on `http://localhost:8000`.

### Workflow

- `--widget widget/` iterates without Rust rebuild
- Test files go in `output/` (gitignored)
- `browser_preview` for layout/rendering debugging

### Pre-Commit Checklist

- [ ] Chrome, Firefox, Safari, Edge
- [ ] Mobile responsive, print styles
- [ ] localStorage, URL sharing, QR codes
- [ ] Keyboard accessibility, load times

## Deployment

- `npm run build` generates minified files (gitignored, Rust rebuilds as needed)
- Verify `CosAmCalendar.init()` backward compatibility

### Security Checklist

- [ ] No `innerHTML` with external data
- [ ] No `eval()`/`new Function()`
- [ ] No inline handlers/`javascript:` URLs
- [ ] QR codes: same-origin URLs only
- [ ] localStorage validation
