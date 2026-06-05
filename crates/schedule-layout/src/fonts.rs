/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Centralized typography: font-size constants, typeface specs, and the Typst
//! `#let` font block.
//!
//! Like [`crate::geometry`], every font value that used to be hard-coded inline
//! lives here, both as a documented Rust constant and as a `#let` variable
//! emitted into the document preamble by [`typst_lets`]. Generators reference
//! those names (`_body-font`, `_name_size`, `_desc-secondary-size`, …) instead
//! of repeating literals or recomputing sizes.
//!
//! Typefaces are emitted as Typst *dictionaries* (`(font: "…", weight: …)`) so a
//! caller can spread them into a text call: `#set text(.._body-font, size: …)`.
//!
//! [`typst_lets`] inspects the [`LayoutConfig`]'s [`ContentMode`] and only emits
//! the grid font sizes when a grid is drawn, the description font size when panel
//! text is drawn, or both — see [`ContentMode::shows_grid`] /
//! [`ContentMode::shows_text`].

use crate::brand::BrandConfig;
use crate::config::LayoutConfig;

// --- Grid font scaling (smallest text is "secondary"; others scale up) ---
/// Floor for the smallest grid text (secondary/duration), in points.
const GRID_MIN_SECONDARY_PT: f64 = 4.0;
/// Header text size as a multiple of the secondary size.
const GRID_HEADER_SCALE: f64 = 1.15;
/// Major (on-the-hour) time-label size as a multiple of the secondary size.
const GRID_TIME_MAJOR_SCALE: f64 = 1.1;
/// Cost-label size as a multiple of the secondary size.
const GRID_COST_SCALE: f64 = 1.05;

// --- Description font scaling ---
/// Secondary text (credits, metadata) size as a multiple of the base size.
const DESC_SECONDARY_SCALE: f64 = 0.9;
/// Floor for the description secondary text size, in points.
const DESC_MIN_SECONDARY_PT: f64 = 7.0;

// --- Banner / footer ---
/// Banner label text size (points).
pub const BANNER_TEXT_SIZE_PT: f64 = 28.0;
/// Footer text size (points).
pub const FOOTER_TEXT_SIZE_PT: f64 = 8.0;

/// Name of the preamble `#let` holding the description/secondary text size
/// (credits, the panel-list time/room text, "(continued)" tags). Emitted by
/// [`typst_lets`] when panel text is drawn; generators reference it by name.
pub const DESC_SECONDARY_SIZE_VAR: &str = "_desc-secondary-size";

/// Build a Typst `font:` argument fragment with optional style and weight.
///
/// Typst's `weight` parameter accepts either an integer (100–900) or a named
/// keyword (`"thin"`, `"extralight"`, `"light"`, `"regular"`, `"medium"`,
/// `"semibold"`, `"bold"`, `"extrabold"`, `"black"`).  Numeric strings from
/// `brand.toml` (e.g. `"200"`) are emitted without quotes; named strings are
/// quoted.  `style` is filtered to the three values Typst accepts
/// (`"normal"`, `"italic"`, `"oblique"`); anything else is ignored.
///
/// Returns a fragment like `font: "Trend Sans", weight: 200` that can be
/// embedded in any Typst `#text(…)` / `#set text(…)` call, or wrapped in
/// parentheses to form a spreadable dictionary (see [`font_dict`]).
pub(crate) fn build_font_spec(font: &str, style: Option<&str>, weight: Option<&str>) -> String {
    let style_part = style
        .filter(|s| matches!(*s, "normal" | "italic" | "oblique"))
        .map(|s| format!(", style: \"{s}\""))
        .unwrap_or_default();
    let weight_part = weight
        .map(|w| {
            if w.chars().all(|c| c.is_ascii_digit()) {
                format!(", weight: {w}")
            } else {
                format!(", weight: \"{w}\"")
            }
        })
        .unwrap_or_default();
    format!("font: \"{font}\"{style_part}{weight_part}")
}

/// Wrap a [`build_font_spec`] fragment as a Typst dictionary literal so it can be
/// spread into a text call, e.g. `(font: "Trend Sans", weight: 200)`.
fn font_dict(font: &str, style: Option<&str>, weight: Option<&str>) -> String {
    format!("({})", build_font_spec(font, style, weight))
}

/// The seven grid text-role sizes, derived from the grid font value.
struct GridFontSizes {
    name: f64,
    secondary: f64,
    header: f64,
    hotel: f64,
    time_major: f64,
    time_minor: f64,
    cost: f64,
}

/// Compute the grid text-role sizes from the smallest (secondary) size.
fn grid_font_sizes(grid_font_pt: f64) -> GridFontSizes {
    let secondary = grid_font_pt.max(GRID_MIN_SECONDARY_PT);
    GridFontSizes {
        name: secondary,
        secondary,
        header: secondary * GRID_HEADER_SCALE,
        hotel: secondary,
        time_major: secondary * GRID_TIME_MAJOR_SCALE,
        time_minor: secondary,
        cost: secondary * GRID_COST_SCALE,
    }
}

/// Compute the description secondary text size (points) from the base size.
fn desc_secondary_pt(base_pt: f64) -> f64 {
    (base_pt * DESC_SECONDARY_SCALE)
        .round()
        .max(DESC_MIN_SECONDARY_PT)
}

/// Emit the `#let` typography block for the document preamble.
///
/// Always defines the typefaces (`_body-font`, `_heading-font`, `_banner-font`),
/// the document base size (`_body-size`), and the banner/footer sizes. The grid
/// text-role sizes (`_name_size`, `_secondary_size`, `_hdr_size`, `_hotel_size`,
/// `_time_size`, `_time_minor_size`, `_cost_size`) are emitted only when the
/// content draws a grid; the description size (`_desc-secondary-size`) only when
/// it draws panel text.
///
/// Must be emitted inside the preamble (before the `#set text` / generators that
/// reference these).
pub(crate) fn typst_lets(config: &LayoutConfig, brand: &BrandConfig) -> String {
    let mut out = String::new();

    // Typefaces (spreadable dictionaries).
    out.push_str(&format!(
        "#let _body-font = {}\n",
        font_dict(
            brand.fonts.body_or_default(),
            brand.fonts.body_style(),
            brand.fonts.body_weight(),
        ),
    ));
    out.push_str(&format!(
        "#let _heading-font = {}\n",
        font_dict(
            brand.fonts.heading_or_default(),
            brand.fonts.heading_style(),
            brand.fonts.heading_weight(),
        ),
    ));
    out.push_str(&format!(
        "#let _banner-font = {}\n",
        font_dict(
            brand.fonts.banner_or_default(),
            brand.fonts.banner_style(),
            Some(brand.fonts.banner_weight_or_default()),
        ),
    ));

    // Document base size and banner/footer sizes (always present).
    out.push_str(&format!(
        "#let _body-size = {}\n",
        config.effective_font_pt()
    ));
    // Banner text size: use the job override if set, otherwise the built-in default.
    let banner_size_default = format!("{BANNER_TEXT_SIZE_PT}pt");
    let banner_size = config
        .banner_text_pt
        .as_deref()
        .unwrap_or(&banner_size_default);
    out.push_str(&format!("#let _banner-text-size = {banner_size}\n"));
    out.push_str(&format!(
        "#let _footer-text-size = {}pt\n",
        FOOTER_TEXT_SIZE_PT
    ));

    // Grid text-role sizes — only when a grid is drawn.
    if config.content.shows_grid() {
        let g = grid_font_sizes(config.grid_font_value());
        out.push_str(&format!(
            "#let _name_size = {:.1}pt\n\
             #let _secondary_size = {:.1}pt\n\
             #let _hdr_size = {:.1}pt\n\
             #let _hotel_size = {:.1}pt\n\
             #let _time_size = {:.1}pt\n\
             #let _time_minor_size = {:.1}pt\n\
             #let _cost_size = {:.1}pt\n",
            g.name, g.secondary, g.header, g.hotel, g.time_major, g.time_minor, g.cost,
        ));
    }

    // Description secondary size — only when panel text is drawn.
    if config.content.shows_text() {
        out.push_str(&format!(
            "#let _desc-secondary-size = {}pt\n",
            desc_secondary_pt(config.base_font_value()),
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ContentMode;

    #[test]
    fn test_build_font_spec_numeric_weight() {
        assert_eq!(
            build_font_spec("Trend Sans", None, Some("200")),
            "font: \"Trend Sans\", weight: 200"
        );
    }

    #[test]
    fn test_build_font_spec_named_weight_and_style() {
        assert_eq!(
            build_font_spec("Trend Sans", Some("italic"), Some("bold")),
            "font: \"Trend Sans\", style: \"italic\", weight: \"bold\""
        );
    }

    #[test]
    fn test_font_dict_wraps_in_parens() {
        assert_eq!(
            font_dict("Trend Sans", None, Some("200")),
            "(font: \"Trend Sans\", weight: 200)"
        );
    }

    #[test]
    fn test_desc_secondary_floor() {
        // 9pt base → 8.1 → round 8; tiny base clamps to the 7pt floor.
        assert_eq!(desc_secondary_pt(9.0), 8.0);
        assert_eq!(desc_secondary_pt(5.0), 7.0);
    }

    #[test]
    fn test_typst_lets_grid_only_omits_description() {
        let config = LayoutConfig {
            content: ContentMode::GridOnly {
                section: None,
                time: crate::config::TimeSplit::Day,
            },
            ..LayoutConfig::default()
        };
        let lets = typst_lets(&config, &BrandConfig::default());
        assert!(lets.contains("#let _name_size"));
        assert!(!lets.contains("_desc-secondary-size"));
        assert!(lets.contains("#let _body-font = (font:"));
    }

    #[test]
    fn test_typst_lets_description_only_omits_grid() {
        let config = LayoutConfig {
            content: ContentMode::DescriptionOnly {
                section: None,
                time: None,
            },
            ..LayoutConfig::default()
        };
        let lets = typst_lets(&config, &BrandConfig::default());
        assert!(lets.contains("_desc-secondary-size"));
        assert!(!lets.contains("#let _name_size"));
    }

    #[test]
    fn test_typst_lets_both_emits_grid_and_description() {
        let config = LayoutConfig {
            content: ContentMode::Both {
                section: None,
                time: crate::config::TimeSplit::Day,
            },
            ..LayoutConfig::default()
        };
        let lets = typst_lets(&config, &BrandConfig::default());
        assert!(lets.contains("#let _name_size"));
        assert!(lets.contains("_desc-secondary-size"));
    }
}
