/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Typst `.typ` source generation helpers.

use chrono::NaiveDate;

use crate::brand::BrandConfig;
use crate::color::{ColorMode, PanelColor};
use crate::grid::{GridLayout, LayoutConfig};
use crate::model::ScheduleData;

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
/// paper name.
pub fn preamble(config: &LayoutConfig, brand: &BrandConfig) -> String {
    use crate::grid::PaperSize;

    let heading_font = brand.fonts.heading_or_default();
    let body_font = brand.fonts.body_or_default();
    let primary = &brand.colors.primary;
    let dark_grey = &brand.colors.dark_grey;
    let font_size = config.paper.base_font_pt();

    let landscape = config.orientation.is_landscape();
    let page_spec = match config.paper {
        PaperSize::Poster => {
            // 30"×20" — dimensions encode landscape; orientation field is ignored.
            "width: 30in, height: 20in".to_string()
        }
        _ => {
            let name = config.paper.typst_name().unwrap_or("us-letter");
            let flip = if landscape { ", flipped: true" } else { "" };
            format!("paper: \"{name}\"{flip}")
        }
    };

    format!(
        r#"#set page({page_spec}, margin: (top: 0.5in, bottom: 0.5in, left: 0.4in, right: 0.4in))
#set text(font: "{body_font}", size: {font_size})
#show heading: set text(font: "{heading_font}")

#let brand-primary = rgb("{primary}")
#let brand-dark = rgb("{dark_grey}")
"#,
        page_spec = page_spec,
        body_font = body_font,
        font_size = font_size,
        heading_font = heading_font,
        primary = primary,
        dark_grey = dark_grey,
    )
}

/// Generate a schedule grid table for one day's panels.
///
/// When `highlight_room_uid` is `Some(uid)`, that room's column header is
/// rendered with the brand-primary fill and its body cells get a light tint,
/// making it easy to spot on a room door sign.
///
/// If `day_label` is empty the heading line is suppressed (useful when the
/// grid is embedded inside a larger layout that already has a header).
pub fn schedule_grid(
    layout: &GridLayout,
    data: &ScheduleData,
    _brand: &BrandConfig,
    _config: &LayoutConfig,
    color_mode: ColorMode,
    day_label: &str,
    highlight_room_uid: Option<i64>,
) -> String {
    let mut out = String::new();

    // Day heading (suppressed when label is empty)
    if !day_label.is_empty() {
        out.push_str(&format!("= {}\n\n", escape_typst(day_label)));
    }

    if layout.room_order.is_empty() || layout.time_slots.is_empty() {
        out.push_str("_(No events scheduled)_\n");
        return out;
    }

    // Table header row
    out.push_str("#table(\n  align: left,\n");

    // Column widths: time col fixed, room cols equal
    let time_col_w = "0.7in";
    let room_col_w = "1fr";
    let cols_spec: Vec<&str> = std::iter::once(time_col_w)
        .chain(std::iter::repeat_n(room_col_w, layout.room_order.len()))
        .collect();
    out.push_str(&format!("  columns: ({}),\n", cols_spec.join(", ")));

    // Header cells — highlighted room gets brand-primary fill with white text
    out.push_str(&format!(
        "  table.header([], {}),\n",
        layout
            .room_order
            .iter()
            .map(|uid| {
                let name = escape_typst(layout.room_name(*uid, &data.rooms));
                if highlight_room_uid == Some(*uid) {
                    format!(
                        "[#table.cell(fill: brand-primary)[#text(fill: white)[*{}*]]]",
                        name
                    )
                } else {
                    format!("[*{}*]", name)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    ));

    // Time slot rows
    for (row_idx, slot) in layout.time_slots.iter().enumerate() {
        let time_cell = if slot.is_major {
            format!("[*{}*]", escape_typst(&slot.label))
        } else {
            format!("[#text(size: 7pt)[{}]]", escape_typst(&slot.label))
        };
        out.push_str(&format!("  {},\n", time_cell));

        for col_idx in 0..layout.room_order.len() {
            let room_id = layout.room_order[col_idx];
            let is_highlighted = highlight_room_uid == Some(room_id);

            // Find cell starting at this slot and column
            let cell = layout
                .cells
                .iter()
                .find(|c| c.row_start == row_idx && c.col == col_idx);

            if let Some(cell) = cell {
                let panel = &cell.panel;
                let rowspan = (cell.row_end - cell.row_start).max(1);
                let color_str = panel
                    .panel_type
                    .as_ref()
                    .and_then(|pt| data.panel_types.get(pt.as_str()))
                    .and_then(|pt| PanelColor::resolve(&pt.colors, color_mode))
                    .map(|c| c.hex)
                    .unwrap_or_default();

                let stroke = if color_str.is_empty() {
                    String::new()
                } else {
                    format!("stroke: (left: 3pt + rgb(\"{}\"))", color_str)
                };
                let fill = if is_highlighted {
                    "fill: brand-primary.lighten(85%)".to_string()
                } else {
                    String::new()
                };
                let cell_args = [fill, stroke]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>();

                let name = escape_typst(&panel.name);
                let dur = panel
                    .duration
                    .map(|d| format!(" _{} min_", d))
                    .unwrap_or_default();

                let rowspan_arg = if rowspan > 1 {
                    format!("rowspan: {}", rowspan)
                } else {
                    String::new()
                };
                let all_args = std::iter::once(rowspan_arg)
                    .chain(cell_args)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>();

                if all_args.is_empty() {
                    out.push_str(&format!("  [*{}*{}],\n", name, dur));
                } else {
                    out.push_str(&format!(
                        "  table.cell({})[*{}*{}],\n",
                        all_args.join(", "),
                        name,
                        dur
                    ));
                }
            } else {
                // Check if this cell is occupied by a rowspan from above
                let is_spanned = layout
                    .cells
                    .iter()
                    .any(|c| c.col == col_idx && c.row_start < row_idx && c.row_end > row_idx);

                // Check break
                let break_here = layout.break_cells.iter().find(|c| c.row_start == row_idx);

                if is_spanned {
                    // Typst handles rowspan automatically — don't emit a cell
                } else if let Some(brk) = break_here {
                    if col_idx == 0 {
                        let rowspan = (brk.row_end - brk.row_start).max(1);
                        let name = escape_typst(&brk.panel.name);
                        out.push_str(&format!(
                            "  table.cell(colspan: {}, rowspan: {}, fill: luma(240))[#align(center)[_{}_ ]],\n",
                            layout.room_order.len() + 1,
                            rowspan,
                            name
                        ));
                    }
                } else if is_highlighted {
                    out.push_str("  table.cell(fill: brand-primary.lighten(85%))[],\n");
                } else {
                    out.push_str("  [],\n");
                }
            }
        }
    }

    out.push_str(")\n\n");
    out
}

/// Generate a full Typst document string for a schedule layout.
pub fn generate_schedule_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    day_label: &str,
    panels: &[&crate::model::Panel],
) -> String {
    let layout = GridLayout::compute(panels, data);
    let mut doc = preamble(config, brand);
    doc.push_str(&schedule_grid(
        &layout, data, brand, config, color_mode, day_label, None,
    ));
    doc
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
        use crate::grid::{Orientation, PaperSize};
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
    fn test_preamble_poster_custom_dimensions() {
        use crate::grid::{Orientation, PaperSize};
        let config = LayoutConfig {
            paper: PaperSize::Poster,
            orientation: Orientation::Landscape,
            ..LayoutConfig::default()
        };
        let brand = BrandConfig::default();
        let pre = preamble(&config, &brand);
        assert!(pre.contains("width: 30in"), "should use custom width");
        assert!(pre.contains("height: 20in"), "should use custom height");
        assert!(!pre.contains("paper:"), "should not use paper: key");
        assert!(pre.contains("10pt"), "poster should use 10pt font");
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
