# FEATURE-110: Add IDML export format option

## Summary

Add Adobe InDesign Markup Language (IDML) as an optional export format for schedule layouts.

## Status

Open

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
Add CLI option to cosam-layout: `--format idml`

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

## Acceptance Criteria

- `cargo test -p schedule-layout` passes with `idml` feature
- `IdmlBuilder` creates valid IDML package structure
- Generated IDML can be opened in Adobe InDesign without errors
- Basic schedule layout (panels, rooms, times) renders correctly
- Feature flag allows building without IDML dependencies
- CLI option `--format idml` produces .idml file

## Notes

IDML specification is available from Adobe. Initial implementation should focus on basic schedule layout (text frames, simple styles, single-page layouts). Advanced features (story threading, complex transforms, embedded images) can be deferred.

This is an exploratory feature - evaluate after initial implementation whether IDML provides sufficient value over Typst/PDF to justify ongoing maintenance.
