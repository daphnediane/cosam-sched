# FEATURE-110: Add IDML export format option

## Summary

Add Adobe InDesign Markup Language (IDML) as an optional export format for schedule layouts.

## Status

Completed

## Priority

Low

## Blocked By

- FEATURE-106: schedule-layout crate must be implemented first

## Description

Add IDML export as an optional output format in the schedule-layout crate. IDML is Adobe's XML-based format for InDesign documents, packaged as a ZIP archive containing XML files and assets. This would provide an alternative to the Typst/PDF workflow for users who need editable InDesign files or require InDesign-specific features.

IDML is significantly more complex than the current Typst approach, requiring XML generation for multiple components (Stories, Spreads, MasterSpreads, Styles, Resources) and ZIP packaging. This feature should be implemented as an optional format behind a feature flag.

## Implementation Details

### Dependencies

- `quick-xml` or `serde-xml-rs` for XML generation
- `zip` crate for packaging IDML archive
- Feature-gated behind `idml` feature flag in schedule-layout

### Module Structure

Add `schedule-layout/src/idml_gen.rs` module with:

- `IdmlBuilder` struct for constructing IDML package
- Functions for generating designmap.xml
- Functions for generating Stories/ XML files (text content)
- Functions for generating Spreads/ XML files (page layouts)
- Functions for generating Styles/ XML files (paragraph, character styles)
- Functions for generating MasterSpreads/ XML files (master pages)
- ZIP packaging function to create final .idml file

### Integration

Add to `LayoutFormat` enum: `Idml`
Add to `formats/` subdirectory: `idml.rs` submodule
Expose via `cosam-convert` layout config (e.g. a `format = "idml"` job key,
selectable on the command line as `--layout.format=idml`); `cosam-layout` was
removed in CLI-139

### IDML Structure

IDML package contains:

- `META-INF/container.xml` - package manifest
- `META-INF/manifest.xml` - file list
- `designmap.xml` - document overview
- `Stories/*.xml` - text content and formatting
- `Spreads/*.xml` - page/spread layouts
- `MasterSpreads/*.xml` - master page templates
- `Styles/*.xml` - style definitions
- `Resources/` - images, fonts, color profiles

### Complexity Considerations

IDML requires understanding of:

- Text frame positioning and transforms
- Character and paragraph style hierarchies
- Story threading (linked text frames)
- Color space definitions (CMYK, RGB)
- Page geometry and master page application

## v1 Implementation (delivered)

Shipped a **threaded text listing** export (the work item's "initial
implementation focused on text frames, simple styles"):

- `schedule-layout/src/idml.rs` — `generate_idml(data, brand, config) -> Vec<u8>`
  produces the full `.idml` ZIP. Parts are emitted as hand-built XML strings
  (no `quick-xml`/`serde-xml` dependency), modeled on a real InDesign-authored
  package: `mimetype` (stored, first), `designmap.xml`, `META-INF/`,
  `Resources/{Fonts,Styles,Graphic,Preferences}.xml`, one `MasterSpreads/` part,
  N `Spreads/` (one page + one text frame each, threaded), one `Stories/` part,
  and `XML/{BackingStory,Tags}.xml`. Output is deterministic.
- Panels are grouped day → time slot; `panel_list` content is compact, other
  modes full. Paragraph styles (Day/Slot/Title/Meta/Body) are driven by the
  brand config; `heading_idml_style` / `body_idml_style` give the font's exact
  InDesign style name (overriding the numeric Typst weight).
- `LayoutFormat { Typst, Idml }` on `LayoutConfig`; wired through `cosam-convert`
  as `format = "idml"` / `--layout.format=idml`, behind the `idml` cargo feature.

### Deferred to follow-up work items

- The schedule **grid** as an InDesign `<Table>` (built from `GridLayout`), for
  `grid_only` / `both`, with room/presenter section splits.
- Embedded fonts/logos and per-panel-type CMYK swatches.
- True overflow-driven pagination (v1 estimates page count heuristically).

## Acceptance Criteria

- [x] `cargo test -p schedule-layout` passes with `idml` feature
- [x] `generate_idml` creates a valid IDML package structure (ZIP + parts,
      verified well-formed in tests)
- [x] Generated IDML opens in Adobe InDesign (confirmed by the user; only a
      normal missing-font *substitution* prompt when brand fonts aren't installed
      — not a document error)
- [x] Basic schedule layout (panels, rooms, times) renders correctly
- [x] Feature flag allows building without IDML dependencies (`idml` not in
      `default` for either crate)
- [x] `cosam-convert` layout option `format = "idml"` (`--layout.format=idml`)
      produces a `.idml` file

## Notes

IDML specification is available from Adobe. Initial implementation should focus on basic schedule layout (text frames, simple styles, single-page layouts). Advanced features (story threading, complex transforms, embedded images) can be deferred.

This is an exploratory feature - evaluate after initial implementation whether IDML provides sufficient value over Typst/PDF to justify ongoing maintenance.
