/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Typst `.typ` source generation helpers.

use chrono::NaiveDate;

use crate::brand::BrandConfig;
use crate::config::LayoutConfig;

/// Escape a string for use as Typst content (inside `[]` or `""`).
pub fn escape_typst(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('#', "\\#")
        .replace('@', "\\@")
        .replace('$', "\\$")
        .replace('*', "\\*")
        .replace('<', "\\<")
        .replace('>', "\\>")
}

/// Generate the Typst document preamble with paper size, fonts, and brand colors.
///
/// For the `Poster` paper size the page is set with explicit `width`/`height`
/// instead of a named `paper:` key, because 30"×20" is not a standard Typst
/// paper name. Both portrait (20"×30") and landscape (30"×20") are supported.
pub fn preamble(config: &LayoutConfig, brand: &BrandConfig) -> String {
    use crate::config::PaperSize;

    let primary = &brand.colors.primary;
    let dark_grey = &brand.colors.dark_grey;

    let landscape = config.orientation.is_landscape();
    let page_spec = match config.paper {
        PaperSize::Poster => {
            // 30"×20" base dimensions — swap for landscape orientation.
            if landscape {
                "width: 30in, height: 20in".to_string()
            } else {
                "width: 20in, height: 30in".to_string()
            }
        }
        _ => {
            let name = config.paper.typst_name().unwrap_or("us-letter");
            let flip = if landscape { ", flipped: true" } else { "" };
            format!("paper: \"{name}\"{flip}")
        }
    };

    // Geometry and font `#let`s come first so `#set page` margins and `#set text`
    // can reference them. Typefaces are dicts spread into the text calls.
    let geometry_lets = crate::geometry::typst_lets();
    let font_lets = crate::fonts::typst_lets(config, brand);

    format!(
        r#"{geometry_lets}{font_lets}#set page({page_spec}, margin: (top: _content-top, bottom: _page-edge, left: _page-edge, right: _page-edge), header-ascent: _header-ascent)
#set text(.._body-font, size: _body-size)
#show heading: set text(.._heading-font)

#let brand-primary = rgb("{primary}")
#let brand-dark = rgb("{dark_grey}")
"#,
        geometry_lets = geometry_lets,
        font_lets = font_lets,
        page_spec = page_spec,
        primary = primary,
        dark_grey = dark_grey,
    )
}

// ---------------------------------------------------------------------------
// Day label helpers (shared by descriptions, schedule, and other formats)
// ---------------------------------------------------------------------------

/// Compute a human-friendly day label from a `YYYY-MM-DD` string.
///
/// Chooses the most compact representation that still unambiguously identifies
/// the day among `all_days`:
///
/// - All days in the same ISO week → `"Thursday"`
/// - Multiple weeks, same calendar month → `"Thursday 25"`
/// - Spans multiple months → `"Thursday Jun 25"`
pub fn make_day_label(date_str: &str, all_days: &[&str]) -> String {
    use chrono::Datelike;

    let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") else {
        return date_str.to_string();
    };
    let weekday = date.format("%A").to_string();

    let parsed: Vec<NaiveDate> = all_days
        .iter()
        .filter_map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .collect();

    let min_date = parsed.iter().copied().min().unwrap_or(date);
    let max_date = parsed.iter().copied().max().unwrap_or(date);

    let same_week = min_date.iso_week() == max_date.iso_week();
    let same_month = min_date.year() == max_date.year() && min_date.month() == max_date.month();

    if same_week {
        weekday
    } else if same_month {
        format!("{} {}", weekday, date.day())
    } else {
        format!("{} {} {}", weekday, date.format("%b"), date.day())
    }
}

/// Convert a day label (e.g. `"Thursday 25"`) to a file-stem slug
/// (e.g. `"thursday-25"`).
pub fn day_label_to_stem(label: &str) -> String {
    label
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_typst_brackets() {
        assert_eq!(escape_typst("Hello [world]"), "Hello \\[world\\]");
    }

    #[test]
    fn test_escape_typst_hash() {
        assert_eq!(escape_typst("#Heading"), "\\#Heading");
    }

    #[test]
    fn test_preamble_contains_paper() {
        let config = LayoutConfig::default(); // default orientation is Landscape
        let brand = BrandConfig::default();
        let pre = preamble(&config, &brand);
        assert!(pre.contains("us-tabloid"));
        assert!(pre.contains("flipped: true"));
    }

    #[test]
    fn test_preamble_portrait_no_flip() {
        use crate::config::{Orientation, PaperSize};
        let config = LayoutConfig {
            paper: PaperSize::Letter,
            orientation: Orientation::Portrait,
            ..LayoutConfig::default()
        };
        let brand = BrandConfig::default();
        let pre = preamble(&config, &brand);
        assert!(pre.contains("us-letter"));
        assert!(!pre.contains("flipped"), "portrait should not add flipped");
    }

    #[test]
    fn test_preamble_contains_brand_color() {
        let config = LayoutConfig::default();
        let brand = BrandConfig::default();
        let pre = preamble(&config, &brand);
        assert!(pre.contains("#00BCDD"));
    }

    #[test]
    fn test_preamble_poster_landscape() {
        use crate::config::{Orientation, PaperSize};
        let config = LayoutConfig {
            paper: PaperSize::Poster,
            orientation: Orientation::Landscape,
            ..LayoutConfig::default()
        };
        let brand = BrandConfig::default();
        let pre = preamble(&config, &brand);
        assert!(
            pre.contains("width: 30in"),
            "landscape: wide dimension first"
        );
        assert!(
            pre.contains("height: 20in"),
            "landscape: short dimension second"
        );
        assert!(!pre.contains("paper:"), "should not use paper: key");
        assert!(pre.contains("10pt"), "poster should use 10pt font");
    }

    #[test]
    fn test_preamble_poster_portrait() {
        use crate::config::{Orientation, PaperSize};
        let config = LayoutConfig {
            paper: PaperSize::Poster,
            orientation: Orientation::Portrait,
            ..LayoutConfig::default()
        };
        let brand = BrandConfig::default();
        let pre = preamble(&config, &brand);
        assert!(
            pre.contains("width: 20in"),
            "portrait: narrow dimension first"
        );
        assert!(
            pre.contains("height: 30in"),
            "portrait: tall dimension second"
        );
        assert!(!pre.contains("paper:"), "should not use paper: key");
    }

    #[test]
    fn test_make_day_label_single_week() {
        let days = ["2026-06-26", "2026-06-27", "2026-06-28"];
        let label = make_day_label("2026-06-27", &days);
        assert_eq!(label, "Saturday");
    }

    #[test]
    fn test_make_day_label_multi_week_same_month() {
        let days = ["2026-06-20", "2026-06-27"];
        let label = make_day_label("2026-06-27", &days);
        assert_eq!(label, "Saturday 27");
    }

    #[test]
    fn test_day_label_to_stem() {
        assert_eq!(day_label_to_stem("Saturday"), "saturday");
        assert_eq!(day_label_to_stem("Saturday 27"), "saturday-27");
    }
}
