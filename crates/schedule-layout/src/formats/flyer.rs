/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Double-sided flyer layout builder.
//!
//! Produces a *single* multi-day Typst document (unlike the per-day formats).
//! [`ContentMode`] selects what each day carries; this one format subsumes the
//! former standalone schedule-grid and descriptions listings:
//!
//! - [`ContentMode::Both`] (default): each section begins on an odd page so the
//!   booklet can be printed double-sided — ideally one sheet per section. The
//!   section's schedule grid occupies the left half of the columns (rounded up)
//!   and descriptions flow through the remaining right-hand columns and onto any
//!   following full-width pages. A blank page is inserted when needed so the
//!   next section lands on an odd page.
//! - [`ContentMode::GridOnly`]: only the schedule grid, full width, one section
//!   per page (replaces the former `Schedule` format).
//! - [`ContentMode::DescriptionOnly`]: only the descriptions, as one continuous
//!   multi-column flow across all days; `split_by` is ignored (replaces the
//!   former `Descriptions` format).
//!
//! [`SplitMode`] controls section granularity for the grid-bearing modes: one
//! section per day, or one per AM/PM half-day.
//!
//! Column counts default to [`PaperSize::flyer_columns`] (grid modes) or
//! [`PaperSize::description_columns`] (description-only), and can be overridden
//! via [`LayoutConfig::columns`]. Every page carries a branded running header
//! whose right label is the current section's day, plus a footer selected by
//! [`FooterMode`].

use crate::blocks::banner;
use crate::blocks::grid::{render_schedule_grid, GridRenderConfig};
use crate::blocks::panels::render_time_grouped_panels;
use crate::brand::BrandConfig;
use crate::grid::{ContentMode, FooterMode, GridLayout, LayoutConfig, SplitMode};
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
) -> Vec<(String, String)> {
    let panels = data.scheduled_panels();
    if panels.is_empty() {
        return vec![];
    }

    // Group all scheduled panels by calendar day, preserving first-seen order.
    let by_day = group_by_day(&panels);
    if by_day.is_empty() {
        return vec![];
    }

    let all_date_strs: Vec<String> = by_day.iter().map(|(d, _)| d.clone()).collect();
    let all_dates: Vec<&str> = all_date_strs.iter().map(String::as_str).collect();

    let color_mode = config.color_mode;
    let font_pt = config.effective_font_pt();

    let mut doc = preamble(config, brand);

    // The header bar is always present, so the top margin is fixed; widen the
    // bottom margin only when a footer is shown (the shared preamble uses a
    // near-zero bottom margin tuned for edge-to-edge grids).
    let bottom = if matches!(config.footer, FooterMode::None) {
        "0.125in"
    } else {
        "0.5in"
    };
    doc.push_str(&format!(
        "#set page(margin: (top: 0.625in, bottom: {bottom}, left: 0.125in, right: 0.125in), \
         footer-descent: 0.15in)\n",
    ));

    // Page footer (selected by FooterMode). Set before the header so both
    // `#set page` directives apply document-wide.
    let timestamps = banner::footer_timestamps(&data.meta.modified, &data.meta.generated);
    let site = brand
        .meta
        .site_url
        .as_deref()
        .or(brand.meta.name.as_deref())
        .unwrap_or_default();
    match config.footer {
        FooterMode::Full => doc.push_str(&banner::page_footer(brand, &timestamps, site)),
        FooterMode::TimestampOnly => {
            doc.push_str(&banner::page_footer_timestamps_only(&timestamps))
        }
        FooterMode::None => {}
    }

    // Page header: branded bar whose right label is the *current section's day*,
    // resolved per page from `<flyer-day>` markers emitted at each section start.
    // Filtering by page (read-only query, no `state.update`) keeps layout
    // convergent.
    doc.push_str(&banner::page_header_running(
        brand,
        "#context {\n    \
           let _days = query(<flyer-day>).filter(m => m.location().page() <= here().page())\n    \
           if _days.len() > 0 { _days.last().value }\n  \
         }",
    ));

    match config.content {
        ContentMode::DescriptionOnly => {
            // One continuous column flow across all days (no page splitting). Day
            // markers feed the running header; `render_time_grouped_panels`
            // inserts the visible day/time headings itself.
            let total_cols =
                config.effective_columns(config.paper.description_columns(config.orientation));
            doc.push_str(&format!("#columns({}, gutter: 0.2in)[\n", total_cols));
            for (date_str, day_panels) in &by_day {
                let day_label = make_day_label(date_str, &all_dates);
                doc.push_str(&day_marker(&day_label));
                doc.push_str(&render_time_grouped_panels(
                    data, color_mode, day_panels, font_pt,
                ));
            }
            doc.push_str("]\n");
        }
        ContentMode::GridOnly => {
            // Full-width grid, one section per page (no double-sided padding).
            let sections = build_sections(config.split_by, &by_day, &all_dates);
            for (i, (label, sec_panels)) in sections.iter().enumerate() {
                if i > 0 {
                    doc.push_str("#pagebreak()\n\n");
                }
                doc.push_str(&day_marker(label));

                let layout = GridLayout::compute(sec_panels, data);
                let mut grid_cfg = GridRenderConfig::full_page("", None)
                    .with_base_font(config.grid_font_value());
                grid_cfg.corner_label = label.clone();
                doc.push_str(&render_schedule_grid(&layout, data, color_mode, &grid_cfg));
            }
        }
        ContentMode::Both => {
            // Column split: left half (rounded up) is the grid, the rest are
            // descriptions. The total must stay even-friendly for a clean split.
            let total_cols =
                config.effective_columns(config.paper.flyer_columns(config.orientation));
            let grid_cols = total_cols.div_ceil(2);
            let grid_pct = grid_cols as f64 / total_cols as f64 * 100.0;

            let sections = build_sections(config.split_by, &by_day, &all_dates);
            for (i, (label, sec_panels)) in sections.iter().enumerate() {
                // Every section after the first starts on a fresh odd page (blank
                // padding page inserted automatically when the previous section
                // ended on an odd page).
                if i > 0 {
                    doc.push_str("#pagebreak(to: \"odd\")\n\n");
                }
                doc.push_str(&day_marker(label));

                // Schedule grid, placed over the left grid_cols columns of this
                // page. `place` reserves no space, so the description column flow
                // (below) keeps the full page height; we skip the grid's columns
                // with leading colbreaks. The section label goes in the grid's
                // top-left corner cell (no heading above the grid).
                let layout = GridLayout::compute(sec_panels, data);
                let mut grid_cfg = GridRenderConfig::full_page("", None)
                    .with_base_font(config.grid_font_value());
                grid_cfg.corner_label = label.clone();
                let grid_content = render_schedule_grid(&layout, data, color_mode, &grid_cfg);

                doc.push_str(&format!(
                    "#place(top + left, box(width: {:.2}%)[\n",
                    grid_pct
                ));
                doc.push_str(&grid_content);
                doc.push_str("])\n");

                // Descriptions: one continuous full-width column flow. Leading
                // colbreaks push the first page's text past the grid into the
                // right-hand columns; overflow continues full-width on the
                // following pages.
                doc.push_str(&format!("#columns({}, gutter: 0.2in)[\n", total_cols));
                for _ in 0..grid_cols {
                    doc.push_str("#colbreak()\n");
                }
                doc.push_str(&render_time_grouped_panels(
                    data, color_mode, sec_panels, font_pt,
                ));
                doc.push_str("]\n");
            }
        }
    }

    vec![(String::new(), doc)]
}

/// Invisible day marker that feeds the running page header.
fn day_marker(label: &str) -> String {
    format!(
        "#metadata(\"{}\") <flyer-day>\n",
        label.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

/// Group scheduled panels by calendar day (YYYY-MM-DD), preserving first-seen
/// order.
fn group_by_day<'a>(panels: &[&'a Panel]) -> Vec<(String, Vec<&'a Panel>)> {
    let mut by_day: Vec<(String, Vec<&'a Panel>)> = vec![];
    for panel in panels {
        if let Some(start) = &panel.start_time {
            let date = start.get(..10).unwrap_or("unknown").to_string();
            if let Some(entry) = by_day.iter_mut().find(|(d, _)| d == &date) {
                entry.1.push(panel);
            } else {
                by_day.push((date, vec![panel]));
            }
        }
    }
    by_day
}

/// Expand the per-day groups into labelled sections according to `split`.
fn build_sections<'a>(
    split: SplitMode,
    by_day: &[(String, Vec<&'a Panel>)],
    all_dates: &[&str],
) -> Vec<(String, Vec<&'a Panel>)> {
    match split {
        SplitMode::Day => by_day
            .iter()
            .map(|(date_str, day_panels)| {
                (make_day_label(date_str, all_dates), day_panels.clone())
            })
            .collect(),
        SplitMode::HalfDay => by_day
            .iter()
            .flat_map(|(date_str, day_panels)| {
                let day_label = make_day_label(date_str, all_dates);
                split_halves(&day_label, day_panels)
            })
            .collect(),
    }
}

/// Split a day's panels into AM and PM halves, dropping empty halves.
fn split_halves<'a>(
    day_label: &str,
    panels: &[&'a Panel],
) -> Vec<(String, Vec<&'a Panel>)> {
    let hour_of = |p: &&'a Panel| -> Option<u32> {
        p.start_time
            .as_ref()
            .and_then(|s| s.get(11..13))
            .and_then(|h| h.parse::<u32>().ok())
    };

    let am: Vec<&'a Panel> = panels
        .iter()
        .copied()
        .filter(|p| hour_of(p).map(|h| h < 12).unwrap_or(false))
        .collect();
    let pm: Vec<&'a Panel> = panels
        .iter()
        .copied()
        .filter(|p| hour_of(p).map(|h| h >= 12).unwrap_or(false))
        .collect();

    let mut out = vec![];
    if !am.is_empty() {
        out.push((format!("{} AM", day_label), am));
    }
    if !pm.is_empty() {
        out.push((format!("{} PM", day_label), pm));
    }
    out
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

    #[test]
    fn test_split_halves_partitions_by_noon() {
        let am = Panel {
            id: "A".into(),
            start_time: Some("2026-06-26T09:00:00".into()),
            ..Panel::default()
        };
        let pm = Panel {
            id: "B".into(),
            start_time: Some("2026-06-26T14:00:00".into()),
            ..Panel::default()
        };
        let refs = vec![&am, &pm];
        let halves = split_halves("Friday", &refs);
        assert_eq!(halves.len(), 2);
        assert_eq!(halves[0].0, "Friday AM");
        assert_eq!(halves[1].0, "Friday PM");
        assert_eq!(halves[0].1.len(), 1);
        assert_eq!(halves[1].1.len(), 1);
    }
}
