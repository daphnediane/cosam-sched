---
description: How to test Rust code and the embedded calendar widget
---

# Testing

## Rust

- `cargo test` at workspace root runs all crate tests.
- `cargo clippy` before committing; treat warnings as errors.
- `cargo build -p cosam-convert` and `cargo build -p cosam-editor` to verify compilation.

## Widget Testing

Generated test files go in `output/` (gitignored). The `widget/` directory is source-only.

### Generate test pages

```bash
cargo run -p cosam-convert -- \
  --input "input/<year> Schedule.xlsx" \
  --export output/<year>.json \
  --export-embed output/<year>-embed.html \
  --export-test output/<year>-test.html \
  --title "Cosplay America <year> Schedule"
```

Open `output/<year>-test.html` in a browser. It wraps the widget in a Squarespace-like layout.

### Iterate on widget CSS/JS

Use `--widget widget/` to read CSS/JS from disk instead of the compiled-in builtins:

```bash
cargo run -p cosam-convert -- \
  --input "input/2026 Schedule.xlsx" \
  --export-test output/2026-test.html \
  --widget widget/ \
  --no-minified
```

This avoids recompiling when only widget files change.

### Batch rebuild

`./scripts/export-schedules.sh` processes all years from `input/` into `output/`.

### Minification

`--minified` (default) uses `minify-html` (lightningcss + oxc) for CSS/JS minification.
`--no-minified` (alias `--for-debug`) produces readable output.

### Widget source overrides

- `--widget <dir>` overrides both CSS and JS (looks for `cosam-calendar.css`/`.js` in dir).
- `--widget-css <path>` and `--widget-js <path>` override individually.
- Value `builtin` reverts to the compiled-in version.

### Test template

`--test-template <file>` overrides the Squarespace simulation template.
The builtin template is `widget/square-template.html` (compiled in via `include_str!`).
