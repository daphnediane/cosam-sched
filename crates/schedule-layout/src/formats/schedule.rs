/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule grid layout builder.

use crate::blocks::banner;
use crate::blocks::grid::{render_schedule_grid, GridRenderConfig};
use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::grid::{GridLayout, LayoutConfig, SplitMode};
use crate::model::ScheduleData;
use crate::typst_gen::{day_label_to_stem, make_day_label, preamble};

/// Generate one or more Typst source documents for the full schedule.
///
/// Returns a `Vec` of `(split_qualifier, typ_source)` pairs.
/// The qualifier is a day or half-day slug (e.g. `"friday"`, `"friday-am"`)
/// that the caller appends to its chosen base stem.
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

    let day_dates = split_by_day(&panels);

    // Collect all date strings (owned) for smart weekday-label computation
    let all_date_strs: Vec<String> = day_dates.iter().map(|(d, _)| d.clone()).collect();
    let all_dates: Vec<&str> = all_date_strs.iter().map(String::as_str).collect();

    match config.split_by {
        SplitMode::Day => day_dates
            .into_iter()
            .map(|(date_str, day_panels)| {
                let day_label = make_day_label(&date_str, &all_dates);
                let source =
                    generate_schedule_typ(data, brand, config, color_mode, &day_label, &day_panels);
                let qualifier = day_label_to_stem(&day_label);
                (qualifier, source)
            })
            .collect(),
        SplitMode::HalfDay => day_dates
            .into_iter()
            .flat_map(|(date_str, day_panels)| {
                let day_label = make_day_label(&date_str, &all_dates);
                split_halves(&day_label, &day_panels)
                    .into_iter()
                    .map(|(label, half_panels)| {
                        let source = generate_schedule_typ(
                            data,
                            brand,
                            config,
                            color_mode,
                            &label,
                            &half_panels,
                        );
                        let qualifier = day_label_to_stem(&label);
                        (qualifier, source)
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
    }
}

/// Generate a full Typst document for a schedule grid.
fn generate_schedule_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    day_label: &str,
    panels: &[&crate::model::Panel],
) -> String {
    let layout = GridLayout::compute(panels, data);
    let grid_cfg = GridRenderConfig::full_page("", None).with_base_font(config.grid_font_value());
    let mut doc = preamble(config, brand);
    doc.push_str(&banner::page_header(brand, None, Some(day_label)));
    doc.push_str(&render_schedule_grid(&layout, data, color_mode, &grid_cfg));
    doc
}

/// Split panels by calendar day (YYYY-MM-DD prefix of startTime).
fn split_by_day<'a>(
    panels: &[&'a crate::model::Panel],
) -> Vec<(String, Vec<&'a crate::model::Panel>)> {
    let mut days: Vec<(String, Vec<&'a crate::model::Panel>)> = vec![];
    for panel in panels {
        if let Some(start) = &panel.start_time {
            let day = start.get(..10).unwrap_or("unknown").to_string();
            if let Some(entry) = days.iter_mut().find(|(d, _)| d == &day) {
                entry.1.push(panel);
            } else {
                days.push((day, vec![panel]));
            }
        }
    }
    days
}

/// Split a day's panels into AM and PM halves.
fn split_halves<'a>(
    day_label: &str,
    panels: &[&'a crate::model::Panel],
) -> Vec<(String, Vec<&'a crate::model::Panel>)> {
    let am: Vec<&'a crate::model::Panel> = panels
        .iter()
        .copied()
        .filter(|p| {
            p.start_time
                .as_ref()
                .and_then(|s| s.get(11..13))
                .and_then(|h| h.parse::<u32>().ok())
                .map(|h| h < 12)
                .unwrap_or(false)
        })
        .collect();
    let pm: Vec<&'a crate::model::Panel> = panels
        .iter()
        .copied()
        .filter(|p| {
            p.start_time
                .as_ref()
                .and_then(|s| s.get(11..13))
                .and_then(|h| h.parse::<u32>().ok())
                .map(|h| h >= 12)
                .unwrap_or(false)
        })
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

    #[test]
    fn test_day_label_to_stem() {
        assert_eq!(day_label_to_stem("Friday"), "friday");
        assert_eq!(day_label_to_stem("Friday AM"), "friday-am");
        assert_eq!(day_label_to_stem("Saturday 27"), "saturday-27");
    }
}
