/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule grid layout builder.

use crate::brand::BrandConfig;
use crate::color::ColorMode;
use crate::grid::{LayoutConfig, SplitMode};
use crate::model::ScheduleData;
use crate::typst_gen;

/// Generate one or more Typst source documents for the full schedule.
///
/// Returns a `Vec` of `(filename_stem, typ_source)` pairs.
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

    let days = split_by_day(&panels);

    match config.split_by {
        SplitMode::Day => days
            .into_iter()
            .map(|(day_label, day_panels)| {
                let source = typst_gen::generate_schedule_typ(
                    data,
                    brand,
                    config,
                    color_mode,
                    &day_label,
                    &day_panels,
                );
                (day_label_to_stem(&day_label), source)
            })
            .collect(),
        SplitMode::HalfDay => days
            .into_iter()
            .flat_map(|(day_label, day_panels)| {
                split_halves(&day_label, &day_panels)
                    .into_iter()
                    .map(|(label, half_panels)| {
                        let source = typst_gen::generate_schedule_typ(
                            data,
                            brand,
                            config,
                            color_mode,
                            &label,
                            &half_panels,
                        );
                        (day_label_to_stem(&label), source)
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
    }
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

fn day_label_to_stem(label: &str) -> String {
    label
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_label_to_stem() {
        assert_eq!(day_label_to_stem("2026-06-26"), "2026-06-26");
        assert_eq!(day_label_to_stem("2026-06-26 AM"), "2026-06-26-am");
    }
}
