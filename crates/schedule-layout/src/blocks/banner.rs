/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Branded page-header bar used by all layout formats.

use crate::brand::BrandConfig;
use crate::typst_gen::{build_font_spec, escape_typst};

/// Generate a `#set page(header: …)` Typst directive for all layout formats.
///
/// Logo placement follows these rules (logo is used when `brand.meta.logo_path`
/// is configured):
///
/// | `left`  | `right` | logo placement              |
/// |---------|---------|-----------------------------|
/// | Some    | Some    | center (between the labels) |
/// | Some    | None    | right (opposite left)       |
/// | None    | Some    | left (opposite right)       |
/// | None    | None    | center                      |
///
/// When no logo is configured: both labels are shown; a single label is
/// centered in the bar.
///
/// Text is rendered ALL CAPS in the banner font (falling back to the heading
/// font) at 28 pt.
///
/// Must be emitted after `preamble()` so that `brand-primary` is already
/// defined in the document scope.
pub(crate) fn page_header(
    brand: &BrandConfig,
    left: Option<&str>,
    right: Option<&str>,
) -> String {
    let logo_path = brand
        .meta
        .logo_path
        .as_ref()
        .and_then(|p| p.to_str())
        .map(|p| p.replace('\\', "/"));

    let inner = build_inner(brand, left, right, logo_path.as_deref());

    format!(
        "#set page(header: block(fill: brand-primary, width: 100%, \
         inset: (x: 10pt, y: 5pt))[\n  {inner}\n])\n",
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Bare Typst content for a banner label (no surrounding brackets).
fn banner_text(brand: &BrandConfig, escaped: &str) -> String {
    let font_spec = build_font_spec(
        brand.fonts.banner_or_default(),
        brand.fonts.banner_style(),
        Some(brand.fonts.banner_weight_or_default()),
    );
    format!("#text(fill: white, size: 28pt, {font_spec})[#upper[{escaped}]]")
}

/// Banner label wrapped in a grid-cell content block.
fn banner_cell(brand: &BrandConfig, raw: &str) -> String {
    format!("[{}]", banner_text(brand, &escape_typst(raw)))
}

fn build_inner(
    brand: &BrandConfig,
    left: Option<&str>,
    right: Option<&str>,
    logo_path: Option<&str>,
) -> String {
    let logo = logo_path.map(|p| format!("image(\"{p}\", height: 0.3in)"));

    match (left, right, logo.as_deref()) {
        // Both labels + logo → L | logo | R
        (Some(l), Some(r), Some(img)) => format!(
            "#grid(columns: (1fr, auto, 1fr), \
             align: (left + horizon, center + horizon, right + horizon), \
             {}, {img}, {})",
            banner_cell(brand, l),
            banner_cell(brand, r),
        ),
        // Both labels, no logo → L | R
        (Some(l), Some(r), None) => format!(
            "#grid(columns: (1fr, auto), \
             align: (left + horizon, right + horizon), \
             {}, {})",
            banner_cell(brand, l),
            banner_cell(brand, r),
        ),
        // Only left + logo → L | logo
        (Some(l), None, Some(img)) => format!(
            "#grid(columns: (1fr, auto), \
             align: (left + horizon, right + horizon), \
             {}, {img})",
            banner_cell(brand, l),
        ),
        // Only right + logo → logo | R
        (None, Some(r), Some(img)) => format!(
            "#grid(columns: (auto, 1fr), \
             align: (left + horizon, right + horizon), \
             {img}, {})",
            banner_cell(brand, r),
        ),
        // Logo only → centered
        (None, None, Some(img)) => format!("#align(center)[{img}]"),
        // Only left, no logo → centered
        (Some(l), None, None) => {
            format!("#align(center)[{}]", banner_text(brand, &escape_typst(l)))
        }
        // Only right, no logo → centered
        (None, Some(r), None) => {
            format!("#align(center)[{}]", banner_text(brand, &escape_typst(r)))
        }
        // Nothing → empty bar
        (None, None, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brand::BrandConfig;

    #[test]
    fn test_banner_weight_fallback() {
        let brand = BrandConfig::default();
        // No banner_weight or heading_weight set → defaults to "bold"
        assert_eq!(brand.fonts.banner_weight_or_default(), "bold");
    }

    #[test]
    fn test_banner_font_fallback() {
        let brand = BrandConfig::default();
        // No banner or heading set → "Liberation Sans"
        assert_eq!(brand.fonts.banner_or_default(), "Liberation Sans");
    }

    #[test]
    fn test_page_header_both_no_logo() {
        let brand = BrandConfig::default();
        let out = page_header(&brand, Some("Room A"), Some("Friday"));
        assert!(out.contains("grid"));
        assert!(out.contains("ROOM A") || out.contains("upper"));
        assert!(out.contains("brand-primary"));
    }

    #[test]
    fn test_page_header_single_centered() {
        let brand = BrandConfig::default();
        let out = page_header(&brand, None, Some("Friday"));
        assert!(out.contains("align(center)"));
        assert!(!out.contains("grid"));
    }
}
