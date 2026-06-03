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
use crate::typst_gen::preamble;

/// Generate Typst source for the combined workshops listing.
///
/// Returns a single `(split_qualifier, typ_source)` pair with an empty qualifier.
/// The caller uses its base stem directly as the filename.
/// Returns an empty `Vec` if there are no workshop panels.
pub fn generate(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
) -> Vec<(String, String)> {
    let color_mode = config.color_mode;
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

    let num_cols = config.effective_columns(config.paper.description_columns(config.orientation));

    let source = generate_listing_typ(data, brand, config, color_mode, &workshop_panels, num_cols);

    vec![(String::new(), source)]
}

fn generate_listing_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    workshop_panels: &[&Panel],
    num_cols: u32,
) -> String {
    let mut doc = preamble(config, brand);
    doc.push_str(&banner::page_header(brand, None, Some("Workshops")));
    doc.push_str(&format!("#columns({n})[\n", n = num_cols));

    let font_pt = config.effective_font_pt();
    doc.push_str(&panels::render_time_grouped_panels(
        data,
        color_mode,
        workshop_panels,
        font_pt,
    ));

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
        );
        assert!(out.is_empty());
    }
}
