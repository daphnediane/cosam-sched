/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget print-format configuration.
//!
//! Resolves the shipped default print formats the widget seeds its print
//! dropdown with. The committed `config/widget-default.toml` is embedded at
//! compile time; a user `config/widget.toml` (gitignored), when present,
//! replaces it wholesale.
//!
//! The TOML authoring shape uses `layout.toml`-style snake_case keys/values
//! (e.g. `content = "grid_only"`, `footer = "timestamp_only"`); this module
//! normalizes them to the widget's camelCase [`SchedulePrintFormat`] runtime
//! values (`"gridOnly"`, `"timestamp"`). It is only compiled with the `layout`
//! feature, since TOML parsing is gated there.

use std::path::{Path, PathBuf};

use schedule_core::widget_json::{SchedulePrintFontSizes, SchedulePrintFonts, SchedulePrintFormat};
use serde::Deserialize;

/// Embedded default print formats (committed, mirrors `layout-default.toml`).
const BUILTIN_WIDGET_DEFAULT: &str = include_str!("../../../config/widget-default.toml");

/// Default user-override path, relative to the working directory.
const DEFAULT_WIDGET_PATH: &str = "config/widget.toml";

/// Top-level `widget.toml` shape.
#[derive(Debug, Default, Deserialize)]
struct RawWidgetConfig {
    #[serde(default)]
    print_formats: Vec<RawPrintFormat>,
}

/// One `[[print_formats]]` entry, in author-facing snake_case.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawPrintFormat {
    name: String,
    content: String,
    color_mode: String,
    columns: u32,
    header_text: String,
    footer_text: String,
    footer: String,
    logo: String,
    page_fill: String,
    cards: bool,
    panel_filter: String,
    time_split: String,
    section_split: String,
    /// When true, apply brand fonts to all four roles (each role → its own name).
    /// Individual `[print_formats.fonts]` entries still override.
    brand_fonts: bool,
    fonts: RawFonts,
    base_font_pt: String,
    grid_font_pt: String,
    banner_text_pt: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawFonts {
    heading: Option<String>,
    banner: Option<String>,
    subheading: Option<String>,
    body: Option<String>,
}

/// Resolve the shipped default print formats.
///
/// Uses `path` if given, else `config/widget.toml`, else the embedded default.
/// A present-but-unparseable user file warns and falls back to the embedded
/// default. Returns an empty vec only if both fail to parse (never in practice,
/// since the embedded default is validated by a test).
#[must_use]
pub fn load_print_formats(path: Option<&Path>) -> Vec<SchedulePrintFormat> {
    let user_path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_WIDGET_PATH));

    if user_path.exists() {
        match std::fs::read_to_string(&user_path) {
            Ok(text) => match parse(&text) {
                Ok(formats) => return formats,
                Err(e) => eprintln!("warning: {user_path:?}: {e}; using built-in print formats"),
            },
            Err(e) => {
                eprintln!("warning: reading {user_path:?}: {e}; using built-in print formats")
            }
        }
    }

    parse(BUILTIN_WIDGET_DEFAULT).unwrap_or_default()
}

fn parse(text: &str) -> Result<Vec<SchedulePrintFormat>, toml::de::Error> {
    let raw: RawWidgetConfig = toml::from_str(text)?;
    Ok(raw.print_formats.into_iter().map(into_widget).collect())
}

fn into_widget(r: RawPrintFormat) -> SchedulePrintFormat {
    let mut fonts = SchedulePrintFonts::default();
    if r.brand_fonts {
        fonts.heading = "heading".to_string();
        fonts.banner = "banner".to_string();
        fonts.subheading = "subheading".to_string();
        fonts.body = "body".to_string();
    }
    if let Some(v) = r.fonts.heading {
        fonts.heading = v;
    }
    if let Some(v) = r.fonts.banner {
        fonts.banner = v;
    }
    if let Some(v) = r.fonts.subheading {
        fonts.subheading = v;
    }
    if let Some(v) = r.fonts.body {
        fonts.body = v;
    }

    SchedulePrintFormat {
        name: r.name,
        content_mode: normalize_content(&r.content),
        color_mode: normalize_color(&r.color_mode),
        columns: r.columns,
        header_text: r.header_text,
        footer_text: r.footer_text,
        footer_mode: normalize_footer(&r.footer),
        logo: if r.logo.is_empty() {
            "none".to_string()
        } else {
            r.logo
        },
        page_fill: r.page_fill,
        cards: r.cards,
        panel_filter: normalize_filter(&r.panel_filter),
        time_split: normalize_time_split(&r.time_split),
        section_split: normalize_section_split(&r.section_split),
        fonts,
        font_sizes: SchedulePrintFontSizes {
            base: r.base_font_pt,
            grid: r.grid_font_pt,
            banner: r.banner_text_pt,
        },
    }
}

/// Map content-mode aliases (snake_case or camelCase) to the widget value.
fn normalize_content(s: &str) -> String {
    match s {
        "grid_only" | "gridOnly" => "gridOnly",
        "description_only" | "descriptionOnly" => "descriptionOnly",
        "panel_list" | "panelList" => "panelList",
        "both" => "both",
        "" => "both",
        other => other,
    }
    .to_string()
}

fn normalize_color(s: &str) -> String {
    match s {
        "bw" => "bw",
        _ => "color",
    }
    .to_string()
}

/// Map footer aliases to the widget value (`full` | `timestamp` | `none`).
fn normalize_footer(s: &str) -> String {
    match s {
        "none" => "none",
        "timestamp" | "timestamp_only" => "timestamp",
        _ => "full",
    }
    .to_string()
}

fn normalize_filter(s: &str) -> String {
    match s {
        "workshops" => "workshops",
        "premium" => "premium",
        _ => "all",
    }
    .to_string()
}

/// Map time-split aliases to the widget value (`none` | `day` | `half_day` |
/// `timeline`). Empty maps to `none`; unknown values pass through unchanged
/// (the widget's `_coercePrintFormat` is the validation authority and falls
/// back to `none`), mirroring [`normalize_content`].
fn normalize_time_split(s: &str) -> String {
    match s {
        "" | "none" => "none",
        "halfDay" => "half_day",
        other => other,
    }
    .to_string()
}

/// Map section-split aliases to the widget value (`none` | `room` |
/// `presenter`). Empty maps to `none`; unknown values pass through unchanged.
fn normalize_section_split(s: &str) -> String {
    match s {
        "" => "none",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_default_parses() {
        let formats = parse(BUILTIN_WIDGET_DEFAULT).expect("embedded widget-default.toml valid");
        assert!(
            !formats.is_empty(),
            "default config defines at least one format"
        );
        // Every format needs a name.
        assert!(formats.iter().all(|f| !f.name.is_empty()));
    }

    #[test]
    fn test_normalization() {
        assert_eq!(normalize_content("grid_only"), "gridOnly");
        assert_eq!(normalize_content("descriptionOnly"), "descriptionOnly");
        assert_eq!(normalize_footer("timestamp_only"), "timestamp");
        assert_eq!(normalize_color("bw"), "bw");
        assert_eq!(normalize_color("anything"), "color");
        assert_eq!(normalize_filter("workshops"), "workshops");
    }

    #[test]
    fn test_split_normalization() {
        assert_eq!(normalize_time_split(""), "none");
        assert_eq!(normalize_time_split("none"), "none");
        assert_eq!(normalize_time_split("day"), "day");
        assert_eq!(normalize_time_split("half_day"), "half_day");
        assert_eq!(normalize_time_split("halfDay"), "half_day");
        assert_eq!(normalize_time_split("timeline"), "timeline");
        // Unknown values pass through (widget validates and falls back to none).
        assert_eq!(normalize_time_split("future_mode"), "future_mode");

        assert_eq!(normalize_section_split(""), "none");
        assert_eq!(normalize_section_split("room"), "room");
        assert_eq!(normalize_section_split("presenter"), "presenter");
    }

    #[test]
    fn test_time_split_round_trips_through_parse() {
        let toml = "\
            [[print_formats]]\n\
            name = \"Grid\"\n\
            content = \"grid_only\"\n\
            time_split = \"half_day\"\n\
            section_split = \"room\"\n";
        let formats = parse(toml).expect("valid");
        assert_eq!(formats[0].time_split, "half_day");
        assert_eq!(formats[0].section_split, "room");
    }

    #[test]
    fn test_unknown_toml_fields_are_ignored() {
        // widget.toml intentionally shares layout.toml-style keys; ones that do
        // not apply to browser print (e.g. `orientation`) are silently dropped.
        let toml = "\
            [[print_formats]]\n\
            name = \"Grid\"\n\
            content = \"grid_only\"\n\
            orientation = \"landscape\"\n";
        let formats = parse(toml).expect("unknown keys ignored, not an error");
        assert_eq!(formats[0].name, "Grid");
    }

    #[test]
    fn test_into_widget_brand_fonts_shorthand() {
        let raw = RawPrintFormat {
            name: "X".to_string(),
            content: "both".to_string(),
            brand_fonts: true,
            ..Default::default()
        };
        let w = into_widget(raw);
        assert_eq!(w.fonts.heading, "heading");
        assert_eq!(w.fonts.body, "body");
        assert_eq!(w.logo, "none"); // empty logo → "none"
    }

    #[test]
    fn test_into_widget_explicit_font_override() {
        let raw = RawPrintFormat {
            name: "X".to_string(),
            brand_fonts: true,
            fonts: RawFonts {
                heading: Some(String::new()), // explicitly clear heading
                ..Default::default()
            },
            ..Default::default()
        };
        let w = into_widget(raw);
        assert_eq!(w.fonts.heading, ""); // override wins
        assert_eq!(w.fonts.banner, "banner"); // shorthand retained
    }
}
