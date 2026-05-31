/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Panel descriptions listing layout builder.
//!
//! Produces one Typst document per day listing all scheduled panels with
//! their time, room, title, description, workshop notices, and cross-references
//! in a multi-column layout. Orientation is determined by `LayoutConfig::orientation`.

use crate::blocks::{banner, panels};
use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::grid::LayoutConfig;
use crate::model::{Panel, ScheduleData};
use crate::typst_gen::{day_label_to_stem, make_day_label, preamble};

/// Generate Typst source for the full panel descriptions listing.
///
/// Returns `(split_qualifier, typ_source)` pairs, one per day.
/// The qualifier is a day slug (e.g. `"friday"`) that the caller appends
/// to its chosen base stem.
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

    // Collect sorted unique day strings (YYYY-MM-DD)
    let mut days: Vec<(String, Vec<&Panel>)> = vec![];
    for panel in &panels {
        if let Some(start) = &panel.start_time {
            let day = start.get(..10).unwrap_or("unknown").to_string();
            if let Some(entry) = days.iter_mut().find(|(d, _)| d == &day) {
                entry.1.push(panel);
            } else {
                days.push((day, vec![panel]));
            }
        }
    }

    // Collect owned day strings for smart label logic before consuming `days`
    let all_day_strs: Vec<String> = days.iter().map(|(d, _)| d.clone()).collect();
    let all_day_refs: Vec<&str> = all_day_strs.iter().map(String::as_str).collect();

    days.into_iter()
        .map(|(day, day_panels)| {
            let label = make_day_label(&day, &all_day_refs);
            let qualifier = day_label_to_stem(&label);
            let source = generate_day_typ(data, brand, config, color_mode, &label, &day_panels);
            (qualifier, source)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Per-day document
// ---------------------------------------------------------------------------

fn generate_day_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    heading: &str,
    day_panels: &[&Panel],
) -> String {
    let num_cols = config.paper.description_columns(config.orientation);

    let mut doc = preamble(config, brand);
    doc.push_str(&banner::page_header(brand, heading));
    doc.push_str("\n#v(0.25in)\n");
    doc.push_str(&format!("#columns({n})[\n", n = num_cols));

    let font_pt = config.effective_font_pt();
    doc.push_str(&panels::render_time_grouped_panels(
        data, color_mode, day_panels, font_pt,
    ));

    doc.push_str("]\n");
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Meta, ScheduleData};

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
            panel_types: std::collections::HashMap::new(),
            timeline: vec![],
            presenters: vec![],
        }
    }

    // -- day label --

    #[test]
    fn test_day_label_single_week() {
        let days = ["2026-06-25", "2026-06-26", "2026-06-27", "2026-06-28"];
        assert_eq!(make_day_label("2026-06-25", &days), "Thursday");
        assert_eq!(make_day_label("2026-06-27", &days), "Saturday");
    }

    #[test]
    fn test_day_label_multi_week_same_month() {
        // Two weekends in June, different ISO weeks
        let days = ["2026-06-19", "2026-06-20", "2026-06-26", "2026-06-27"];
        assert_eq!(make_day_label("2026-06-26", &days), "Friday 26");
        assert_eq!(make_day_label("2026-06-19", &days), "Friday 19");
    }

    #[test]
    fn test_day_label_cross_month() {
        let days = ["2026-06-27", "2026-07-04"];
        assert_eq!(make_day_label("2026-06-27", &days), "Saturday Jun 27");
        assert_eq!(make_day_label("2026-07-04", &days), "Saturday Jul 4");
    }

    // -- generate empty --

    #[test]
    fn test_generate_empty() {
        let out = generate(
            &empty_schedule(),
            &BrandConfig::default(),
            &LayoutConfig::default(),
            ColorMode::Color,
        );
        assert!(out.is_empty());
    }
}
