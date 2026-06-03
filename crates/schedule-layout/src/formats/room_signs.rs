/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Room signs layout builder.
//!
//! Produces a *single* multi-page Typst document holding every room's signs
//! (unlike a per-room/per-day file split).  Each room/day combination starts on
//! a fresh page laid out like the flyer's first page:
//!
//! - The full conference schedule grid is `place`d over the left half of the
//!   columns (rounded up), with this room's column highlighted in the brand
//!   primary color and the day label in the grid's corner.
//! - This room's own event descriptions flow through a full-width column block;
//!   leading column breaks push the first page's text past the reserved grid
//!   into the right-hand columns, and any overflow continues full-width on the
//!   following pages before the next room/day's page break.
//!
//! Every page carries a branded running header (room name left, day right,
//! resolved per page from `<room-sign>` markers) and a footer with the page
//! number plus the modified/generated timestamps.
//!
//! Optionally filtered to a single room UID via `config.filter.room_uid`.

use crate::blocks::banner;
use crate::blocks::grid::{render_schedule_grid, GridRenderConfig};
use crate::blocks::panels::render_time_grouped_panels;
use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::grid::{GridLayout, LayoutConfig};
use crate::model::{Panel, Room, ScheduleData};
use crate::typst_gen::{make_day_label, preamble};

/// Generate Typst source for room door signs.
///
/// Returns a single `(qualifier, typ_source)` pair with an empty qualifier, so
/// the caller's filename is just `{stem}-{paper}` — every room's signs live in
/// one document.  Returns an empty vec when there are no scheduled panels.
pub fn generate(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
) -> Vec<(String, String)> {
    let panels = data.scheduled_panels();
    if panels.is_empty() {
        return vec![];
    }

    // Determine which rooms to generate signs for (optionally filtered to one).
    let rooms: Vec<&Room> = data
        .sorted_rooms()
        .into_iter()
        .filter(|r| config.filter.room_uid.map(|uid| r.uid == uid).unwrap_or(true))
        .collect();

    // Group ALL panels by calendar day — the full grid requires all rooms.
    let mut by_day: Vec<(String, Vec<&Panel>)> = vec![];
    for panel in &panels {
        if let Some(start) = &panel.start_time {
            let date = start.get(..10).unwrap_or("unknown").to_string();
            if let Some(entry) = by_day.iter_mut().find(|(d, _)| d == &date) {
                entry.1.push(panel);
            } else {
                by_day.push((date, vec![panel]));
            }
        }
    }
    if by_day.is_empty() {
        return vec![];
    }

    // Collect owned date strings for smart weekday-label computation.
    let all_date_strs: Vec<String> = by_day.iter().map(|(d, _)| d.clone()).collect();
    let all_dates: Vec<&str> = all_date_strs.iter().map(String::as_str).collect();

    // Column split: left half (rounded up) is the grid, the rest are descriptions.
    let total_cols = config.paper.description_columns(config.orientation);
    let grid_cols = total_cols.div_ceil(2);
    let grid_pct = grid_cols as f64 / total_cols as f64 * 100.0;

    let font_pt = config.effective_font_pt();

    let mut doc = preamble(config, brand);

    // Widen the bottom margin so the footer has room to sit (the shared preamble
    // uses a near-zero bottom margin tuned for edge-to-edge grids).
    doc.push_str(
        "#set page(margin: (top: 0.625in, bottom: 0.5in, left: 0.125in, right: 0.125in), \
         footer-descent: 0.15in)\n",
    );

    // Page footer: timestamps + page number + site. Set before the header so
    // both `#set page` directives apply document-wide.
    let timestamps = banner::footer_timestamps(&data.meta.modified, &data.meta.generated);
    let site = brand
        .meta
        .site_url
        .as_deref()
        .or(brand.meta.name.as_deref())
        .unwrap_or_default();
    doc.push_str(&banner::page_footer(brand, &timestamps, site));

    // Running header: room name (left) and day label (right), resolved per page
    // from `<room-sign>` markers emitted at each room/day's start. Filtering by
    // page (read-only query, no `state.update`) keeps layout convergent.
    doc.push_str(&banner::page_header_running_split(
        brand,
        &running_field("room"),
        &running_field("day"),
    ));

    let mut first = true;
    for room in &rooms {
        let room_name = if !room.long_name.is_empty() {
            &room.long_name
        } else {
            &room.short_name
        };

        for (date_str, day_panels) in &by_day {
            // Only generate a sign if this room has events on this day.
            let room_panels: Vec<&Panel> = day_panels
                .iter()
                .copied()
                .filter(|p| p.room_ids.contains(&room.uid))
                .collect();
            if room_panels.is_empty() {
                continue;
            }

            // Every room/day after the first starts on a fresh page.
            if !first {
                doc.push_str("#pagebreak()\n\n");
            }
            first = false;

            let day_label = make_day_label(date_str, &all_dates);

            // Marker for the running header (no visible output): carries this
            // page's room and day so later pages of an overflowing sign keep the
            // same labels until the next marker.
            doc.push_str(&format!(
                "#metadata((room: \"{}\", day: \"{}\")) <room-sign>\n",
                typst_str(room_name),
                typst_str(&day_label),
            ));

            // Full schedule grid, this room's column highlighted, day label in
            // the corner cell. `place` reserves no space, so the description
            // column flow (below) keeps the full page height; we skip the grid's
            // columns with leading colbreaks.
            let layout = GridLayout::compute(day_panels, data);
            let mut grid_cfg = GridRenderConfig::full_page("", Some(room.uid))
                .with_base_font(config.grid_font_value());
            grid_cfg.corner_label = day_label.clone();
            let grid_content = render_schedule_grid(&layout, data, color_mode, &grid_cfg);

            doc.push_str(&format!("#place(top + left, box(width: {:.2}%)[\n", grid_pct));
            doc.push_str(&grid_content);
            doc.push_str("])\n");

            // Descriptions: one continuous full-width column flow. Leading
            // colbreaks push the first page's text past the grid into the
            // right-hand columns; overflow continues full-width on the following
            // pages.
            doc.push_str(&format!("#columns({}, gutter: 0.2in)[\n", total_cols));
            for _ in 0..grid_cols {
                doc.push_str("#colbreak()\n");
            }
            doc.push_str(&render_time_grouped_panels(
                data,
                color_mode,
                &room_panels,
                font_pt,
            ));
            doc.push_str("]\n");
        }
    }

    // If no room/day section was emitted (e.g. filtered to a room with no
    // events), there is nothing worth a document.
    if first {
        return vec![];
    }

    vec![(String::new(), doc)]
}

/// Build the running-header `context` expression that reads `field` ("room" or
/// "day") from the most recent `<room-sign>` marker on or before the current
/// page.
fn running_field(field: &str) -> String {
    format!(
        "#context {{\n    \
           let _m = query(<room-sign>).filter(m => m.location().page() <= here().page())\n    \
           if _m.len() > 0 {{ _m.last().value.{field} }}\n  \
         }}",
    )
}

/// Escape a string for embedding inside a Typst double-quoted string literal.
fn typst_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{LayoutConfig, PaperSize};
    use crate::model::{Meta, ScheduleData};
    use std::collections::HashMap;

    fn empty_schedule() -> ScheduleData {
        ScheduleData {
            meta: Meta {
                title: "T".into(),
                version: 0,
                variant: String::new(),
                generator: String::new(),
                generated: String::new(),
                modified: String::new(),
                start_time: None,
                end_time: None,
            },
            panels: vec![],
            rooms: vec![],
            panel_types: HashMap::new(),
            timeline: vec![],
            presenters: vec![],
        }
    }

    #[test]
    fn test_generate_empty_returns_empty() {
        let config = LayoutConfig {
            paper: PaperSize::Tabloid,
            ..LayoutConfig::default()
        };
        let out = generate(
            &empty_schedule(),
            &BrandConfig::default(),
            &config,
            ColorMode::Color,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn test_typst_str_escapes() {
        assert_eq!(typst_str("Salon A"), "Salon A");
        assert_eq!(typst_str(r#"a "b" c"#), r#"a \"b\" c"#);
        assert_eq!(typst_str(r"a\b"), r"a\\b");
    }

    #[test]
    fn test_running_field_reads_named_field() {
        let room = running_field("room");
        assert!(room.contains("<room-sign>"));
        assert!(room.contains(".value.room"));
        assert!(room.contains("here().page()"));

        let day = running_field("day");
        assert!(day.contains(".value.day"));
    }
}
