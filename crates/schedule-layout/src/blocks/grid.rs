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
use crate::time_fmt;
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
/* Zig-zag constants - currently unused, reserved for future torn-edge effect:
const ZIGZAG_TOOTH_PT: f64 = 8.0;
const ZIGZAG_HEIGHT_PT: f64 = 6.0;
const ZIGZAG_LUMA: u16 = 140;
const ZIGZAG_STROKE_PT: f64 = 1.2;
*/
/// Dotted stroke width (pt) for truncated cell borders (top/bottom).
const TRUNC_STROKE_PT: f64 = 1.5;
/// Grey level of the dotted truncation border.
const TRUNC_STROKE_LUMA: u16 = 140;

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

    // The global-max time key is an event END that nothing extends past (e.g.
    // the last event's end or an overnight break's next-day end). A full `1fr`
    // track for such trailing end-only slots would render as an empty row at the
    // foot of the grid. Cells occupy rows `[row_start, row_end)`, so any slot at
    // or beyond the deepest `row_end` is touched by no event or break; drop those
    // trailing rows entirely so the grid ends flush with the last event (matching
    // the widget grid).
    let body_slots = layout
        .cells
        .iter()
        .chain(layout.break_cells.iter())
        .map(|c| c.row_end)
        .max()
        .unwrap_or(n_slots);

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

    // Zig-zag / torn-edge logic for panels truncated at time-split boundaries.
    // Currently using a simple dotted border stroke instead of the full zig-zag
    // polygon overlay (which had issues with polygon closing edges and background
    // fill not matching the torn shape).
    //
    // To re-enable zig-zag in the future, uncomment below and update the
    // truncated border logic in render_event_cell to call _zigzag().
    /*
    // Zig-zag constants: tooth=8pt, height=6pt, stroke=1.2pt, luma=140
    out.push_str(&format!(
        "{kw} _zz_tooth = 8pt\n\
         {kw} _zz_h     = 6pt\n\
         {kw} _zz_sw    = 1.2pt\n\
         {kw} _zz_seed  = 1234.5678\n\
         {kw} _zigzag = (at-top: true, col: luma(140)) => layout(avail => {{\n\
         \x20 let w = avail.width\n\
         \x20 let n = calc.max(1, calc.ceil(w / _zz_tooth))\n\
         \x20 let actual_tooth = w / n\n\
         \x20 let pts = ()\n\
         \x20 let dir = if at-top {{ 1 }} else {{ -1 }}\n\
         \x20 for i in range(n) {{\n\
         \x20   let x0 = i * actual_tooth\n\
         \x20   let x1 = (i + 0.25) * actual_tooth\n\
         \x20   let x2 = (i + 0.5) * actual_tooth\n\
         \x20   let x3 = (i + 0.75) * actual_tooth\n\
         \x20   let x4 = (i + 1) * actual_tooth\n\
         \x20   let r1 = calc.sin(i * 123.456 + _zz_seed) * 0.5 + 0.5\n\
         \x20   let r2 = calc.sin(i * 789.012 + _zz_seed) * 0.5 + 0.5\n\
         \x20   let r3 = calc.sin(i * 345.678 + _zz_seed) * 0.5 + 0.5\n\
         \x20   let h_up = _zz_h * r1 * dir\n\
         \x20   let h_dn = -_zz_h * r2 * dir\n\
         \x20   let h_mid = _zz_h * (r3 - 0.5) * dir * 2\n\
         \x20   pts.push((x0, 0pt))\n\
         \x20   pts.push((x1, h_up))\n\
         \x20   pts.push((x2, h_mid))\n\
         \x20   pts.push((x3, h_dn))\n\
         \x20   pts.push((x4, 0pt))\n\
         \x20 }}\n\
         \x20 place(\n\
         \x20   if at-top {{ top }} else {{ bottom }},\n\
         \x20   polygon(fill: none, stroke: _zz_sw + col, ..pts)\n\
         \x20 )\n\
         }})\n",
        kw = let_kw,
    ));
    */

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

    // Row spec: header auto + one equal (1fr) row per rendered slot (trailing
    // end-only slots are dropped).
    let rows_spec: String = std::iter::once("auto".to_string())
        .chain(std::iter::repeat_n("1fr".to_string(), body_slots))
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

    // --- Time slot rows (trailing end-only slots dropped) ---
    for (row_idx, slot) in layout.time_slots.iter().take(body_slots).enumerate() {
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

    // Cost suffix (inline after title on the first line). For a multi-part
    // series the price is rendered plainly on the lead part and faded, italic,
    // and parenthesized on continuation parts so it never reads as an extra
    // per-part charge while still showing what the series covers.
    let cost_suffix = if !config.show_cost {
        String::new()
    } else if panel.is_premium && panel.is_series_continuation() {
        panel
            .cost
            .as_deref()
            .filter(|c| !c.is_empty())
            .map(|c| {
                format!(
                    " #h(1fr) #text(size: _cost_size, fill: luma(150), style: \"italic\")[({})]",
                    escape_typst(c)
                )
            })
            .unwrap_or_default()
    } else {
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

    // Zig-zag overlays for panels truncated at a time-split boundary are placed
    // outside the block (see top_zz_outside / bot_zz_outside below) so they sit at
    // the cell border rather than inside the inset content area.

    // "↑ cont from X PM" label shown as the first text line of a top-truncated cell.
    let cont_from_str = if cell.truncated_start {
        let orig_start = panel
            .start_time
            .as_deref()
            .map(time_fmt::format_time)
            .unwrap_or_default();
        if orig_start.is_empty() {
            String::new()
        } else {
            format!(
                "#text(size: _secondary_size, style: \"italic\")[\u{2191} cont from {}]\n",
                escape_typst(&orig_start)
            )
        }
    } else {
        String::new()
    };

    // Build stroke for truncated edges (top/bottom) using dotted line.
    // The left accent stroke is handled separately in left_stroke.
    let trunc_stroke = if cell.truncated_start || cell.truncated_end {
        let mut parts = vec![];
        if cell.truncated_start {
            parts.push(format!(
                "top: (thickness: {STROKE_PT}pt, paint: luma({STROKE_LUMA}), dash: \"dotted\")",
                STROKE_PT = TRUNC_STROKE_PT,
                STROKE_LUMA = TRUNC_STROKE_LUMA
            ));
        }
        if cell.truncated_end {
            parts.push(format!(
                "bottom: (thickness: {STROKE_PT}pt, paint: luma({STROKE_LUMA}), dash: \"dotted\")",
                STROKE_PT = TRUNC_STROKE_PT,
                STROKE_LUMA = TRUNC_STROKE_LUMA
            ));
        }
        // Add the left accent if present
        if !color_str.is_empty() {
            parts.push(format!(
                "left: {accent}pt + rgb(\"{color}\")",
                accent = ACCENT_WIDTH_PT,
                color = color_str
            ));
        }
        format!(", stroke: ({})", parts.join(", "))
    } else {
        left_stroke
    };

    out.push_str(&format!(
        "  grid.cell(fill: {fill}{rowspan}{stroke})\
         [#block(clip: true, width: 100%, height: 100%, inset: _cell_inset, \
         stroke: (bottom: {rule_pt}pt + luma({rule_luma})))[\
         {cont_from}\
         #text(size: _name_size, weight: \"bold\")[{name}]{cost}\n{credits}\n{dur}]],\n",
        fill = fill,
        rowspan = rowspan_arg,
        stroke = trunc_stroke,
        rule_pt = CELL_RULE_PT,
        rule_luma = CELL_RULE_LUMA,
        cont_from = cont_from_str,
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
