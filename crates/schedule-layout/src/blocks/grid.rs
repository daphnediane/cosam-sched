/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule grid Typst rendering using CSS-grid-style layout.
//!
//! Produces a `#grid()` with subtle gridlines, colored event boxes, and
//! light-grey empty slots.  Used by both the schedule format (full page)
//! and room signs (embedded in a side-by-side layout).

use std::collections::HashSet;

use crate::color::{ColorMode, PanelColor};
use crate::model::ScheduleData;
use crate::timegrid::GridLayout;
use crate::typst_gen::escape_typst;

/// Configuration for grid rendering.
#[derive(Debug, Clone)]
pub(crate) struct GridRenderConfig {
    /// If set, highlight this room's column with brand-primary tint.
    pub highlight_room_uid: Option<i64>,
    /// If set, highlight individual event cells whose `panel.id` is in this set
    /// (used for presenter schedules). Independent of the column highlight.
    pub highlight_panel_ids: Option<HashSet<String>>,
    /// Day heading printed above the grid (empty = suppressed).
    pub day_label: String,
    /// Label shown in the top-left corner cell, above the time column (empty =
    /// blank corner). When set, the time column widens to fit the larger of
    /// this label and `"Midnight"`.
    pub corner_label: String,
    /// Maximum height for the grid block (e.g. `"8.5in"`).
    /// If `None`, the grid flows naturally without a height constraint.
    pub max_height: Option<String>,
    /// Time column width (e.g. `"0.55in"` for compact, `"0.7in"` for full).
    pub time_col_width: String,
    /// Maximum characters before credits are truncated. `0` = no truncation.
    pub credits_max_chars: usize,
    /// Whether to show hotel room name below the short name in headers.
    pub show_hotel_room: bool,
    /// Whether to show cost in event cells.
    pub show_cost: bool,
    /// Fill for empty (no-event) cells as a Typst color expression. `None` uses
    /// the built-in light gray ([`EMPTY_SLOT_LUMA`]); set it to keep empty cells
    /// from blending into a tinted page background.
    pub empty_fill: Option<String>,
}

// Grid text-role sizes (`_name_size`, `_hdr_size`, …) are emitted globally by
// `fonts::typst_lets` in the preamble; the renderer just references them.

// --- Gridline / cell styling (emitted into the Typst `#grid`) ---
/// Grey level of the grid's hairline stroke.
const GRIDLINE_LUMA: u16 = 210;
/// Grid hairline thickness in points.
const GRIDLINE_THICKNESS_PT: f64 = 0.4;
/// Grey level of the per-cell bottom rule under event text.
const CELL_RULE_LUMA: u16 = 200;
/// Per-cell bottom rule thickness in points.
const CELL_RULE_PT: f64 = 0.3;
/// Width of the panel-type colour accent on the left of an event cell, in points.
const ACCENT_WIDTH_PT: f64 = 2.5;
/// Italic text size for a break row spanning all rooms, in points.
const BREAK_TEXT_PT: f64 = 5.5;
/// Grey level of an empty (no-event) slot.
const EMPTY_SLOT_LUMA: u16 = 245;
/// Grey level of a break row's fill.
const BREAK_FILL_LUMA: u16 = 235;
/// Lighten percentage for a highlighted event cell's fill.
const HIGHLIGHT_FILL_LIGHTEN: u16 = 90;
/// Lighten percentage for an empty slot in the highlighted room column.
const HIGHLIGHT_EMPTY_LIGHTEN: u16 = 78;

impl Default for GridRenderConfig {
    fn default() -> Self {
        Self::full_page("", None)
    }
}

impl GridRenderConfig {
    /// Full-page configuration for standalone schedule grids.
    pub fn full_page(day_label: &str, highlight_room_uid: Option<i64>) -> Self {
        Self {
            highlight_room_uid,
            highlight_panel_ids: None,
            day_label: day_label.to_string(),
            corner_label: String::new(),
            max_height: None,
            time_col_width: String::new(),
            credits_max_chars: 0,
            show_hotel_room: true,
            show_cost: true,
            empty_fill: None,
        }
    }
}

/// Render a schedule grid as Typst source.
///
/// Produces a `#grid()` element with:
/// - Subtle gridlines (thin, light stroke)
/// - Room header row with brand-primary fill
/// - Time label column with tinted fill
/// - Event boxes with panel-type color left accent, clipped text
/// - Empty cells with light grey fill so panels stand out
/// - Break rows spanning all room columns
pub(crate) fn render_schedule_grid(
    layout: &GridLayout,
    data: &ScheduleData,
    color_mode: ColorMode,
    config: &GridRenderConfig,
) -> String {
    let mut out = String::new();

    // Optional day heading
    if !config.day_label.is_empty() {
        out.push_str(&format!("= {}\n\n", escape_typst(&config.day_label)));
    }

    if layout.room_order.is_empty() || layout.time_slots.is_empty() {
        out.push_str("_(No events scheduled)_\n");
        return out;
    }

    let n_rooms = layout.room_order.len();
    let n_slots = layout.time_slots.len();

    // Optional height-constrained wrapper block
    if let Some(ref h) = config.max_height {
        out.push_str(&format!("#block(height: {}, clip: true)[\n", h));
    }

    // When using measured width, we wrap the entire grid in a `context`
    // block so that `measure()` resolves to a length value usable in the
    // column spec.  All variable definitions live inside this code block.
    let use_measured_width = config.time_col_width.is_empty();
    if use_measured_width {
        out.push_str("#context {\n");
    }

    // Grid-cell inset variables (defined once so values stay in sync).
    // Inside context we're in code mode (plain `let`), otherwise markup (`#let`).
    // The `_*_size` font variables are global `#let`s from the preamble
    // (`fonts::typst_lets`), in scope here and inside the `#context` block.
    let let_kw = if use_measured_width { "let" } else { "#let" };
    out.push_str(&format!(
        "{kw} _hdr_inset = (x: 2pt, y: 4pt)\n\
         {kw} _time_inset = (top: 2pt, bottom: 1pt, left: 2pt, right: 6pt)\n\
         {kw} _cell_inset = (x: 3pt, y: 2pt)\n",
        kw = let_kw,
    ));

    // Compute time-column width via measure (only when inside context). When a
    // corner label is present, the column must also fit it, so widen to the
    // larger of the "Midnight" time cell and the corner label.
    if use_measured_width {
        if config.corner_label.is_empty() {
            out.push_str(
                "let _time_col_w = {\n\
                 \x20 let sz = measure(text(size: _time_size, weight: \"bold\")[Midnight])\n\
                 \x20 sz.width + _time_inset.left + _time_inset.right\n\
                 }\n",
            );
        } else {
            out.push_str(&format!(
                "let _time_col_w = {{\n\
                 \x20 let mw = measure(text(size: _time_size, weight: \"bold\")[Midnight]).width \
                   + _time_inset.left + _time_inset.right\n\
                 \x20 let dw = measure(text(size: _hdr_size, weight: \"bold\")[{label}]).width \
                   + _hdr_inset.x * 2\n\
                 \x20 calc.max(mw, dw)\n\
                 }}\n",
                label = escape_typst(&config.corner_label),
            ));
        }
    }

    let time_col_expr = if use_measured_width {
        "_time_col_w".to_string()
    } else {
        config.time_col_width.clone()
    };

    // Column spec: time col + room cols
    let cols_spec: String = std::iter::once(time_col_expr)
        .chain(std::iter::repeat_n("1fr".to_string(), n_rooms))
        .collect::<Vec<_>>()
        .join(", ");

    // Row spec: header auto + slot rows equal
    let rows_spec: String = std::iter::once("auto".to_string())
        .chain(std::iter::repeat_n("1fr".to_string(), n_slots))
        .collect::<Vec<_>>()
        .join(", ");

    let grid_prefix = if use_measured_width { "grid" } else { "#grid" };
    out.push_str(&format!(
        "{prefix}(\n  columns: ({cols}),\n  rows: ({rows}),\n  \
         column-gutter: 0pt,\n  row-gutter: 0pt,\n  \
         stroke: (paint: luma({gridline_luma}), thickness: {gridline_pt}pt),\n",
        prefix = grid_prefix,
        cols = cols_spec,
        rows = rows_spec,
        gridline_luma = GRIDLINE_LUMA,
        gridline_pt = GRIDLINE_THICKNESS_PT,
    ));

    // --- Header row ---
    render_header_row(&mut out, layout, data, config);

    // --- Time slot rows ---
    for (row_idx, slot) in layout.time_slots.iter().enumerate() {
        render_time_cell(&mut out, slot, config);
        render_room_cells(&mut out, layout, data, color_mode, config, row_idx, n_rooms);
    }

    out.push_str(")\n"); // close grid

    if use_measured_width {
        out.push_str("}\n"); // close context
    }

    if config.max_height.is_some() {
        out.push_str("]\n"); // close block
    }

    out
}

// ---------------------------------------------------------------------------
// Internal rendering helpers
// ---------------------------------------------------------------------------

fn render_header_row(
    out: &mut String,
    layout: &GridLayout,
    data: &ScheduleData,
    config: &GridRenderConfig,
) {
    // Corner cell — blank, or the day label when configured.
    if config.corner_label.is_empty() {
        out.push_str("  grid.cell(fill: brand-primary, inset: _hdr_inset)[],\n");
    } else {
        out.push_str(&format!(
            "  grid.cell(fill: brand-primary, inset: _hdr_inset)\
             [#align(center + horizon)[#text(fill: white, size: _hdr_size, weight: \"bold\")[{label}]]],\n",
            label = escape_typst(&config.corner_label),
        ));
    }

    // Room header cells
    for &uid in &layout.room_order {
        let room = data.rooms.iter().find(|r| r.uid == uid);
        let short_name = room.map(|r| r.short_name.as_str()).unwrap_or("?");
        let hotel_room = room.map(|r| r.hotel_room.as_str()).unwrap_or("");
        let is_highlighted = config.highlight_room_uid == Some(uid);

        let fill = if is_highlighted {
            "brand-primary"
        } else {
            "brand-primary.lighten(15%)"
        };
        // Bold the highlighted room, or every room when none is focused.
        let weight = if is_highlighted || config.highlight_room_uid.is_none() {
            ", weight: \"bold\""
        } else {
            ""
        };

        // Build cell content: short name + optional hotel room name below
        let hotel_line = if config.show_hotel_room && !hotel_room.is_empty() {
            format!(
                " \\ #text(fill: white, size: _hotel_size, style: \"italic\")[{}]",
                escape_typst(hotel_room)
            )
        } else {
            String::new()
        };

        out.push_str(&format!(
            "  grid.cell(fill: {fill}, inset: _hdr_inset)\
             [#align(center)[#text(fill: white, size: _hdr_size{weight})[{name}]{hotel}]],\n",
            fill = fill,
            weight = weight,
            name = escape_typst(short_name),
            hotel = hotel_line,
        ));
    }
}

fn render_time_cell(
    out: &mut String,
    slot: &crate::timegrid::TimeSlot,
    _config: &GridRenderConfig,
) {
    let (time_var, time_weight) = if slot.is_major {
        ("_time_size", ", weight: \"bold\"")
    } else {
        ("_time_minor_size", "")
    };
    out.push_str(&format!(
        "  grid.cell(fill: brand-primary, inset: _time_inset)\
         [#align(right + top)\
         [#text(fill: white, size: {size}{weight})[{label}]]],\n",
        size = time_var,
        weight = time_weight,
        label = escape_typst(&slot.label)
    ));
}

fn render_room_cells(
    out: &mut String,
    layout: &GridLayout,
    data: &ScheduleData,
    color_mode: ColorMode,
    config: &GridRenderConfig,
    row_idx: usize,
    n_rooms: usize,
) {
    for col_idx in 0..n_rooms {
        let room_id = layout.room_order[col_idx];
        let is_highlighted = config.highlight_room_uid == Some(room_id);

        // Find cell starting at this slot and column
        let cell = layout
            .cells
            .iter()
            .find(|c| c.row_start == row_idx && c.col == col_idx);

        if let Some(cell) = cell {
            // An event cell is highlighted by its room column OR when its panel
            // id is in the per-panel highlight set (presenter schedules).
            let panel_highlighted = config
                .highlight_panel_ids
                .as_ref()
                .is_some_and(|ids| ids.contains(&cell.panel.id));
            render_event_cell(
                out,
                cell,
                data,
                color_mode,
                config,
                is_highlighted || panel_highlighted,
            );
        } else {
            render_empty_or_spanned_cell(
                out,
                layout,
                config,
                row_idx,
                col_idx,
                n_rooms,
                is_highlighted,
            );
        }
    }
}

fn render_event_cell(
    out: &mut String,
    cell: &crate::timegrid::GridCell,
    data: &ScheduleData,
    color_mode: ColorMode,
    config: &GridRenderConfig,
    is_highlighted: bool,
) {
    let panel = &cell.panel;
    let rowspan = (cell.row_end - cell.row_start).max(1);
    let color_str = panel
        .panel_type
        .as_ref()
        .and_then(|pt| data.panel_types.get(pt.as_str()))
        .and_then(|pt| PanelColor::resolve(&pt.colors, color_mode))
        .map(|c| c.hex)
        .unwrap_or_default();

    let fill = if is_highlighted {
        format!("brand-primary.lighten({}%)", HIGHLIGHT_FILL_LIGHTEN)
    } else {
        "white".to_string()
    };

    let left_stroke = if color_str.is_empty() {
        String::new()
    } else {
        format!(
            ", stroke: (left: {accent}pt + rgb(\"{color}\"), rest: none)",
            accent = ACCENT_WIDTH_PT,
            color = color_str,
        )
    };

    let rowspan_arg = if rowspan > 1 {
        format!(", rowspan: {}", rowspan)
    } else {
        String::new()
    };

    let name = escape_typst(&panel.name);

    // Cost suffix (inline after title on the first line)
    let cost_suffix = if config.show_cost {
        panel
            .cost
            .as_deref()
            .filter(|c| !c.is_empty())
            .map(|c| {
                format!(
                    " #h(1fr) #text(size: _cost_size, fill: luma(100))[{}]",
                    escape_typst(c)
                )
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Presenters / credits line
    let credits_str = if !panel.credits.is_empty() {
        let joined = panel.credits.join(", ");
        let display = if config.credits_max_chars > 0 {
            truncate_str(&joined, config.credits_max_chars)
        } else {
            joined
        };
        format!(
            " \\ #text(size: _secondary_size, style: \"italic\")[{}]",
            escape_typst(&display)
        )
    } else {
        String::new()
    };

    // Duration line
    let dur_label = panel
        .duration
        .map(|d| {
            if d >= 60 && d % 60 == 0 {
                format!("{} hr", d / 60)
            } else if d >= 60 {
                format!("{} hr {} min", d / 60, d % 60)
            } else {
                format!("{} min", d)
            }
        })
        .unwrap_or_default();
    let dur_str = if dur_label.is_empty() {
        String::new()
    } else {
        format!(" \\ #text(size: _secondary_size)[{}]", dur_label)
    };

    out.push_str(&format!(
        "  grid.cell(fill: {fill}{rowspan}{stroke})\
         [#block(clip: true, width: 100%, height: 100%, inset: _cell_inset, \
         stroke: (bottom: {rule_pt}pt + luma({rule_luma})))[\
         #text(size: _name_size, weight: \"bold\")[{name}]{cost}\n{credits}\n{dur}]],\n",
        fill = fill,
        rowspan = rowspan_arg,
        stroke = left_stroke,
        rule_pt = CELL_RULE_PT,
        rule_luma = CELL_RULE_LUMA,
        name = name,
        cost = cost_suffix,
        credits = credits_str,
        dur = dur_str,
    ));
}

fn render_empty_or_spanned_cell(
    out: &mut String,
    layout: &GridLayout,
    config: &GridRenderConfig,
    row_idx: usize,
    col_idx: usize,
    n_rooms: usize,
    is_highlighted: bool,
) {
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
                "  grid.cell(colspan: {}, rowspan: {}, fill: luma({break_luma}))\
                 [#align(center + horizon)\
                 [#text(size: {break_pt}pt, style: \"italic\")[{}]]],\n",
                n_rooms + 1,
                rowspan,
                name,
                break_luma = BREAK_FILL_LUMA,
                break_pt = BREAK_TEXT_PT,
            ));
        }
    } else if is_highlighted {
        // Empty slot in highlighted room — darker muted tint to fade behind panels
        out.push_str(&format!(
            "  grid.cell(fill: brand-primary.lighten({}%))[],\n",
            HIGHLIGHT_EMPTY_LIGHTEN
        ));
    } else {
        // Empty slot — configurable fill, defaulting to the built-in light grey.
        let fill = config
            .empty_fill
            .clone()
            .unwrap_or_else(|| format!("luma({})", EMPTY_SLOT_LUMA));
        out.push_str(&format!("  grid.cell(fill: {fill})[],\n"));
    }
}

/// Truncate a string to a maximum byte length, adding "..." if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len.saturating_sub(3);
        // Don't split in the middle of a multi-byte char
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_short() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_str_exact() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_long() {
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }

    #[test]
    fn test_grid_render_config_full_page() {
        let cfg = GridRenderConfig::full_page("Friday", None);
        assert_eq!(cfg.day_label, "Friday");
        assert!(cfg.max_height.is_none());
        assert_eq!(cfg.credits_max_chars, 0);
    }
}
