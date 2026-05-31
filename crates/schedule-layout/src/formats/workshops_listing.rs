/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Combined workshops listing layout builder.
//!
//! Produces a single Typst document containing all workshop and premium panels
//! across every day of the convention. The layout is identical to the per-day
//! descriptions listing but spans all days, with a day heading inserted whenever
//! the calendar date changes.

use crate::blocks::{banner, panels};
use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::grid::LayoutConfig;
use crate::model::{Panel, ScheduleData};
use crate::typst_gen::{escape_typst, make_day_label, preamble};

/// Generate Typst source for the combined workshops listing.
///
/// Returns a single `(split_qualifier, typ_source)` pair with an empty qualifier.
/// The caller uses its base stem directly as the filename.
/// Returns an empty `Vec` if there are no workshop panels.
pub fn generate(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
) -> Vec<(String, String)> {
    let panels = data.scheduled_panels();

    // Filter to workshops (and cafe/premium types)
    let workshop_panels: Vec<&Panel> = panels
        .into_iter()
        .filter(|p| {
            p.panel_type
                .as_ref()
                .and_then(|pt| data.panel_types.get(pt.as_str()))
                .map(|pt| pt.is_workshop || pt.is_cafe)
                .unwrap_or(false)
        })
        .collect();

    if workshop_panels.is_empty() {
        return vec![];
    }

    // Collect unique day strings (YYYY-MM-DD) in order
    let mut day_strs: Vec<String> = vec![];
    for p in &workshop_panels {
        if let Some(start) = &p.start_time {
            let day = start.get(..10).unwrap_or("unknown").to_string();
            if !day_strs.contains(&day) {
                day_strs.push(day);
            }
        }
    }

    let all_day_refs: Vec<&str> = day_strs.iter().map(String::as_str).collect();
    let num_cols = config.paper.description_columns(config.orientation);

    let source = generate_listing_typ(
        data,
        brand,
        config,
        color_mode,
        &workshop_panels,
        &day_strs,
        &all_day_refs,
        num_cols,
    );

    vec![(String::new(), source)]
}

#[allow(clippy::too_many_arguments)]
fn generate_listing_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    workshop_panels: &[&Panel],
    day_strs: &[String],
    all_day_refs: &[&str],
    num_cols: u32,
) -> String {
    let mut doc = preamble(config, brand);
    doc.push_str(&banner::page_header(brand, "Workshops"));
    doc.push_str(&format!("#columns({n})[\n", n = num_cols));

    let mut state_counter = 0u32;

    for day_str in day_strs {
        let day_panels: Vec<&Panel> = workshop_panels
            .iter()
            .copied()
            .filter(|p| {
                p.start_time
                    .as_deref()
                    .and_then(|s| s.get(..10))
                    .map(|d| d == day_str.as_str())
                    .unwrap_or(false)
            })
            .collect();

        if day_panels.is_empty() {
            continue;
        }

        let day_label = make_day_label(day_str, all_day_refs);
        doc.push_str(&format!("= {}\n\n", escape_typst(&day_label)));
        doc.push_str(&panels::render_time_grouped_panels(
            data,
            color_mode,
            &day_panels,
            day_str,
            &day_label,
            &mut state_counter,
        ));
    }

    doc.push_str("]\n");
    doc
}

#[cfg(test)]
mod tests {
    /*
     * Copyright (c) 2026 Daphne Pfister
     * SPDX-License-Identifier: BSD-2-Clause
     * See LICENSE file for full license text
     */

    use super::*;
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
