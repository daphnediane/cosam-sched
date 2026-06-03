/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Double-sided per-day flyer layout builder.
//!
//! Produces a *single* multi-day Typst document (unlike the per-day formats).
//! Each day begins on an odd page so the booklet can be printed double-sided
//! with — ideally — one sheet per day:
//!
//! - **First page of a day (odd):** the day's schedule grid occupies the left
//!   half of the columns (rounded up); descriptions flow through the remaining
//!   right-hand columns.
//! - **Following pages:** zero or more full-width pages carrying the remaining
//!   descriptions for that day.
//! - A blank page is inserted when needed so the next day lands on an odd page.
//!
//! Column counts: 4 on letter, 6 on legal and larger (landscape).  Every page
//! carries a branded header and a footer with the page number plus the modified
//! and generated timestamps (mirroring the widget's grid footer).

use crate::blocks::banner;
use crate::blocks::grid::{render_schedule_grid, GridRenderConfig};
use crate::blocks::panels::render_time_grouped_panels;
use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::grid::{GridLayout, LayoutConfig};
use crate::model::{Panel, ScheduleData};
use crate::typst_gen::{make_day_label, preamble};

/// Generate the flyer document.
///
/// Returns a single `(qualifier, typ_source)` pair with an empty qualifier, so
/// the caller's filename is just `{stem}-{paper}` — the whole convention lives
/// in one document.
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

    // Group all scheduled panels by calendar day, preserving first-seen order.
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

    let all_date_strs: Vec<String> = by_day.iter().map(|(d, _)| d.clone()).collect();
    let all_dates: Vec<&str> = all_date_strs.iter().map(String::as_str).collect();

    // Column split: left half (rounded up) is the grid, the rest are descriptions.
    let total_cols = config.paper.flyer_columns(config.orientation);
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

    // Page header: branded bar whose right label is the *current day*, resolved
    // per page from `<flyer-day>` markers emitted at each day's start. Filtering
    // by page (read-only query, no `state.update`) keeps layout convergent.
    doc.push_str(&banner::page_header_running(
        brand,
        "#context {\n    \
           let _days = query(<flyer-day>).filter(m => m.location().page() <= here().page())\n    \
           if _days.len() > 0 { _days.last().value }\n  \
         }",
    ));

    for (i, (date_str, day_panels)) in by_day.iter().enumerate() {
        // Every day after the first starts on a fresh odd page (blank padding
        // page inserted automatically when the previous day ended on an odd page).
        if i > 0 {
            doc.push_str("#pagebreak(to: \"odd\")\n\n");
        }

        let day_label = make_day_label(date_str, &all_dates);

        // Day marker for the running header (no visible output).
        doc.push_str(&format!(
            "#metadata(\"{}\") <flyer-day>\n",
            day_label.replace('\\', "\\\\").replace('"', "\\\"")
        ));

        // Schedule grid, placed over the left grid_cols columns of this page.
        // `place` reserves no space, so the description column flow (below) keeps
        // the full page height; we skip the grid's columns with leading colbreaks.
        // The day label goes in the grid's top-left corner cell (no heading above
        // the grid).
        let layout = GridLayout::compute(day_panels, data);
        let mut grid_cfg =
            GridRenderConfig::full_page("", None).with_base_font(config.grid_font_value());
        grid_cfg.corner_label = day_label.clone();
        let grid_content = render_schedule_grid(&layout, data, color_mode, &grid_cfg);

        doc.push_str(&format!(
            "#place(top + left, box(width: {:.2}%)[\n",
            grid_pct
        ));
        doc.push_str(&grid_content);
        doc.push_str("])\n");

        // Descriptions: one continuous full-width column flow. Leading colbreaks
        // push the first page's text past the grid into the right-hand columns;
        // overflow continues full-width on the following pages.
        doc.push_str(&format!("#columns({}, gutter: 0.2in)[\n", total_cols));
        for _ in 0..grid_cols {
            doc.push_str("#colbreak()\n");
        }
        doc.push_str(&render_time_grouped_panels(
            data, color_mode, day_panels, font_pt,
        ));
        doc.push_str("]\n");
    }

    vec![(String::new(), doc)]
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
    fn test_generate_empty() {
        let out = generate(
            &empty_schedule(),
            &BrandConfig::default(),
            &LayoutConfig::default(),
            ColorMode::Color,
        );
        assert!(out.is_empty());
    }

    #[test]
    fn test_flyer_columns_split() {
        // Letter landscape: 4 cols → grid 2, desc 2.
        assert_eq!(
            PaperSize::Letter.flyer_columns(crate::grid::Orientation::Landscape),
            4
        );
        // Legal+ landscape: 6 cols → grid 3, desc 3.
        assert_eq!(
            PaperSize::Legal.flyer_columns(crate::grid::Orientation::Landscape),
            6
        );
        assert_eq!(
            PaperSize::Tabloid.flyer_columns(crate::grid::Orientation::Landscape),
            6
        );
    }
}
