/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Typst `.typ` source generation helpers.

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
}

/// Generate the Typst document preamble with paper size, fonts, and brand colors.
pub fn preamble(config: &LayoutConfig, brand: &BrandConfig, landscape: bool) -> String {
    let paper = config.paper.typst_name();
    let orientation = if landscape { ", flipped: true" } else { "" };
    let heading_font = brand.fonts.heading_or_default();
    let body_font = brand.fonts.body_or_default();
    let primary = &brand.colors.primary;
    let dark_grey = &brand.colors.dark_grey;

    format!(
        r#"#set page(paper: "{paper}"{orientation}, margin: (top: 0.5in, bottom: 0.5in, left: 0.4in, right: 0.4in))
#set text(font: "{body_font}", size: 9pt)
#show heading: set text(font: "{heading_font}")

#let brand-primary = rgb("{primary}")
#let brand-dark = rgb("{dark_grey}")
"#,
        paper = paper,
        orientation = orientation,
        body_font = body_font,
        heading_font = heading_font,
        primary = primary,
        dark_grey = dark_grey,
    )
}

/// Generate a simple grid schedule Typst document for one day's panels.
pub fn schedule_grid(
    layout: &GridLayout,
    data: &ScheduleData,
    _brand: &BrandConfig,
    _config: &LayoutConfig,
    color_mode: ColorMode,
    day_label: &str,
) -> String {
    let mut out = String::new();

    // Day heading
    out.push_str(&format!("= {}\n\n", escape_typst(day_label)));

    if layout.room_order.is_empty() || layout.time_slots.is_empty() {
        out.push_str("_(No events scheduled)_\n");
        return out;
    }

    let col_count = layout.room_order.len() + 1; // +1 for time col

    // Table header row
    out.push_str(&format!(
        "#table(\n  columns: {},\n  align: left,\n",
        col_count
    ));

    // Column widths: time col fixed, room cols equal
    let time_col_w = "0.7in";
    let room_col_w = "1fr";
    let cols_spec: Vec<&str> = std::iter::once(time_col_w)
        .chain(std::iter::repeat(room_col_w).take(layout.room_order.len()))
        .collect();
    out.push_str(&format!("  columns: ({}),\n", cols_spec.join(", ")));

    // Header cells
    out.push_str(&format!(
        "  table.header([], {}),\n",
        layout
            .room_order
            .iter()
            .map(|uid| format!("[*{}*]", escape_typst(layout.room_name(*uid, &data.rooms))))
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
            let _room_id = layout.room_order[col_idx];
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

                let border_color = if color_str.is_empty() {
                    String::new()
                } else {
                    format!(", stroke: (left: 3pt + rgb(\"{}\"))", color_str)
                };

                let name = escape_typst(&panel.name);
                let dur = panel
                    .duration
                    .map(|d| format!(" _{} min_", d))
                    .unwrap_or_default();

                if rowspan > 1 {
                    out.push_str(&format!(
                        "  table.cell(rowspan: {}{})[*{}*{}],\n",
                        rowspan, border_color, name, dur
                    ));
                } else {
                    out.push_str(&format!(
                        "  table.cell({})[*{}*{}],\n",
                        if border_color.is_empty() {
                            String::new()
                        } else {
                            border_color[2..].to_string()
                        },
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
                            layout.room_order.len(), rowspan, name
                        ));
                    }
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
    let mut doc = preamble(config, brand, true);
    doc.push_str(&schedule_grid(
        &layout, data, brand, config, color_mode, day_label,
    ));
    doc
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
        let config = LayoutConfig::default();
        let brand = BrandConfig::default();
        let pre = preamble(&config, &brand, true);
        assert!(pre.contains("us-tabloid"));
        assert!(pre.contains("flipped: true"));
    }

    #[test]
    fn test_preamble_contains_brand_color() {
        let config = LayoutConfig::default();
        let brand = BrandConfig::default();
        let pre = preamble(&config, &brand, true);
        assert!(pre.contains("#00BCDD"));
    }
}
