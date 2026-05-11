# IDEA-109: Color science library for accessibility and contrast

## Summary

Color science library for accessibility and contrast

## Status

Open

## Priority

Medium

## Description

`crates/schedule-layout` currently uses hand-rolled ITU-R BT.601 luminance math
(`LUMA_R/G/B` constants in `color.rs`) to derive grayscale fallbacks for BW
print output. This works for the current use case but leaves several gaps:

### Gaps

- No WCAG 2.1 contrast ratio calculation (4.5:1 for normal text, 3:1 large)
- No perceptual color distance (CIEDE2000) for accessible palette generation
- No color-blindness simulation (Deuteranopia, Protanopia, Tritanopia) for
  test/preview output
- No sRGB ↔ linear RGB ↔ CIELAB conversion for accurate luminance on non-linear
  inputs (BT.601 on raw hex values is approximate for sRGB gamma-encoded inputs)

### Candidate crates

- **`palette`** (0.7) — full color space conversions, sRGB↔Lab, WCAG helpers;
  well-maintained, no unsafe
- **`colorsys`** — lighter weight, fewer conversions
- **`colorimetry`** — newer, more focused on CIE standards

`palette` is the most comprehensive and widely used; worth evaluating when
accessibility-aware PDF output is needed (e.g., verifying panel type colors meet
contrast requirements against white/dark-grey brand backgrounds).

### When to act

When implementing full `cosam-layout` PDF output or when adding
accessibility-verified palette enforcement to `schedule-layout`. Not needed for
the current Typst codegen work.
