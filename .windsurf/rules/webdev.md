---
description: Frontend development practices for the calendar widget
trigger: glob
globs: widget/**/*.css,widget/**/*.js
---

# Frontend Development Practices

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

Open `output/<year>-test.html` in browser. Wraps widget in Squarespace-like layout.

### Iterate on widget CSS/JS

Use `--widget widget/` to read from disk instead of compiled-in builtins:

```bash
cargo run -p cosam-convert -- \
  --input "input/2026 Schedule.xlsx" \
  --export-test output/2026-test.html \
  --widget widget/ \
  --no-minified
```

Avoids recompiling when only widget files change.

### Batch rebuild

`./scripts/export-schedules.sh` processes all years from `input/` into `output/`.

### Minification

- `--minified` (default): uses `minify-html` for CSS/JS minification
- `--no-minified` (alias `--for-debug`): readable output

### Widget source overrides

- `--widget <dir>`: overrides both CSS and JS
- `--widget-css <path>` and `--widget-js <path>`: override individually
- Value `builtin`: reverts to compiled-in version

### Test template

`--test-template <file>` overrides Squarespace simulation template.
Builtin template: `widget/square-template.html` (compiled in via `include_str!`).

## CSS Development

### CSS Architecture

- `cosam-calendar.css`: Main widget stylesheet
- Use CSS custom properties for theming and configuration
- Follow mobile-first responsive design principles
- Ensure compatibility with Squarespace injection

### CSS Best Practices

- Use semantic class names that reflect purpose, not appearance
- Leverage CSS Grid for schedule layout with proper accessibility
- Implement smooth transitions for interactive elements
- Test across browsers and devices
- Maintain color contrast ratios per accessibility requirements

### CSS erformance

- Minimize CSS selector specificity
- Use efficient layout algorithms (Grid > Flexbox > Block)
- Avoid expensive animations and transforms
- Leverage browser caching with appropriate cache headers

## JavaScript Development

### JavaScript Architecture

- `cosam-calendar.js`: Main widget script
- Use modern ES6+ features with appropriate fallbacks
- Implement event delegation for dynamic content
- Separate data processing from DOM manipulation

### JavaScript Best Practices

- Use semantic HTML5 elements generated from JavaScript
- Implement proper error handling and graceful degradation
- Use async/await for asynchronous operations
- Maintain clean separation between widget logic and host page

### JavaScript Performance

- Minimize DOM manipulation and reflows
- Use event delegation instead of individual event listeners
- Implement lazy loading for large datasets
- Leverage browser APIs for optimal performance

## Integration Testing

### Browser Testing

- Test in major browsers: Chrome, Firefox, Safari, Edge
- Verify responsive behavior across device sizes
- Test widget injection into various host environments
- Validate accessibility with screen readers

### JavaScript Performance Testing

- Monitor load times and render performance
- Test with large schedule datasets
- Verify memory usage doesn't grow unbounded
- Test minified vs unminified builds

## Deployment

### Build Process

- Use `--minified` for production builds
- Validate generated HTML/CSS/JS before deployment
- Test embedded widget in target environment
- Verify all assets load correctly

### Version Management

- Tag widget releases with corresponding schedule versions
- Maintain backward compatibility where possible
- Document breaking changes in release notes
- Test upgrade paths from previous versions
