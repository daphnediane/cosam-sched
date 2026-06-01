/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Room signs layout builder.
//!
//! Produces one landscape-tabloid Typst document per room per day.  Each
//! page has a full-width branded header (room name + date), then a side-by-side
//! layout: the full conference schedule grid on the left (with this room's
//! column highlighted in the brand primary color) and description blocks for
//! this room's own events on the right.
//!
//! Optionally filtered to a single room UID via `config.filter.room_uid`.

use crate::blocks::grid::{render_schedule_grid, GridRenderConfig};
use crate::blocks::{banner, panels::render_description_blocks};
use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::grid::{GridLayout, LayoutConfig};
use crate::model::ScheduleData;
use crate::typst_gen::{day_label_to_stem, make_day_label, preamble};

/// Generate Typst source for room door signs.
///
/// Returns `(split_qualifier, typ_source)` pairs, one per room per day.
/// The qualifier is `"{room-slug}-{day-slug}"` that the caller appends to its base stem.
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

    // Determine which rooms to generate signs for
    let rooms: Vec<&crate::model::Room> = data
        .sorted_rooms()
        .into_iter()
        .filter(|r| {
            !data
                .panel_types
                .values()
                .any(|_| false) // placeholder
                && config
                    .filter
                    .room_uid
                    .map(|uid| r.uid == uid)
                    .unwrap_or(true)
        })
        .collect();

    // Group ALL panels by calendar day — the full grid requires all rooms.
    let mut by_day: Vec<(String, Vec<&crate::model::Panel>)> = vec![];
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

    // Collect owned date strings for smart weekday-label computation
    let all_date_strs: Vec<String> = by_day.iter().map(|(d, _)| d.clone()).collect();
    let all_dates: Vec<&str> = all_date_strs.iter().map(String::as_str).collect();

    let mut out = vec![];

    for room in &rooms {
        let room_slug = room_name_slug(&room.short_name);

        for (date_str, day_panels) in &by_day {
            // Only generate a sign if this room has events on this day.
            let room_panels: Vec<&crate::model::Panel> = day_panels
                .iter()
                .copied()
                .filter(|p| p.room_ids.contains(&room.uid))
                .collect();
            if room_panels.is_empty() {
                continue;
            }

            let day_label = make_day_label(date_str, &all_dates);
            let qualifier = format!("{}-{}", room_slug, day_label_to_stem(&day_label));
            let source = generate_sign_typ(
                data,
                brand,
                config,
                color_mode,
                room,
                &day_label,
                date_str,
                day_panels,
                &room_panels,
            );
            out.push((qualifier, source));
        }
    }

    out
}

fn room_name_slug(short_name: &str) -> String {
    short_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[allow(clippy::too_many_arguments)]
fn generate_sign_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    room: &crate::model::Room,
    day_label: &str,
    day_date: &str,
    all_day_panels: &[&crate::model::Panel],
    room_panels: &[&crate::model::Panel],
) -> String {
    let mut doc = preamble(config, brand);

    let room_name = if !room.long_name.is_empty() {
        &room.long_name
    } else {
        &room.short_name
    };

    // Page header: room name on left, logo (center), day label on right
    doc.push_str(&banner::page_header(
        brand,
        Some(room_name),
        Some(day_label),
    ));

    // Column layout: use same total columns as descriptions/workshops.
    // Grid spans the left half (rounded up), descriptions fill the rest.
    let total_cols = config.paper.description_columns(config.orientation);
    let grid_cols = total_cols.div_ceil(2);
    let desc_cols = total_cols - grid_cols;

    // Proportional widths based on column split
    let grid_pct = (grid_cols as f64 / total_cols as f64 * 100.0).round() as u32;

    // Build the CSS-grid-style schedule
    let layout = GridLayout::compute(all_day_panels, data);
    let grid_cfg = GridRenderConfig::compact(room.uid).with_base_font(config.grid_font_value());
    let grid_content = render_schedule_grid(&layout, data, color_mode, &grid_cfg);

    let font_pt = config.effective_font_pt();
    let desc_content =
        render_description_blocks(data, color_mode, room_panels, day_date, day_label, font_pt);

    // Side-by-side layout using proportional widths
    doc.push_str(&format!(
        "#grid(columns: ({}%, 1fr), gutter: 0.2in,\n",
        grid_pct
    ));
    doc.push('['); // left cell: grid
    doc.push_str(&grid_content);
    doc.push_str("],\n");
    // Right cell: descriptions in sub-columns if >1 column allocated
    doc.push('['); // right cell: descriptions
    if desc_cols > 1 {
        doc.push_str(&format!("#columns({})[\n", desc_cols));
        doc.push_str(&desc_content);
        doc.push_str("]\n");
    } else {
        doc.push_str(&desc_content);
    }
    doc.push_str("]\n");
    doc.push_str(")\n");
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_name_slug() {
        assert_eq!(room_name_slug("Salon A"), "salon-a");
        assert_eq!(room_name_slug("Main Stage!"), "main-stage");
        assert_eq!(room_name_slug("  Room 101  "), "room-101");
    }

    #[test]
    fn test_generate_non_tabloid_returns_empty() {
        use crate::grid::{LayoutConfig, PaperSize};
        use crate::model::{Meta, ScheduleData};
        use std::collections::HashMap;
        let data = ScheduleData {
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
        };
        let config = LayoutConfig {
            paper: PaperSize::Letter,
            ..LayoutConfig::default()
        };
        let out = generate(&data, &BrandConfig::default(), &config, ColorMode::Color);
        assert!(out.is_empty());
    }
}
