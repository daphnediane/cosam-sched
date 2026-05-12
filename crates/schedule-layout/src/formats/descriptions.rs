/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Panel descriptions listing layout builder.
//!
//! Produces one Typst document per day listing all scheduled panels with
//! their time, room, title, and description in a two-column layout.

use crate::brand::BrandConfig;
use crate::color::{ColorMode, PanelColor};
use crate::grid::LayoutConfig;
use crate::model::ScheduleData;
use crate::typst_gen::{escape_typst, preamble};

/// Generate Typst source for the full panel descriptions listing.
///
/// Returns `(filename_stem, typ_source)` pairs, one per day.
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

    // Group by day
    let mut days: Vec<(String, Vec<&crate::model::Panel>)> = vec![];
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

    days.into_iter()
        .map(|(day, day_panels)| {
            let stem = day
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>();
            let stem = format!("descriptions-{}", stem);
            let source = generate_day_typ(data, brand, config, color_mode, &day, &day_panels);
            (stem, source)
        })
        .collect()
}

fn generate_day_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    day_label: &str,
    panels: &[&crate::model::Panel],
) -> String {
    let mut doc = preamble(config, brand, false);

    // Two-column layout
    doc.push_str("#set columns(2)\n#columns(2)[\n");
    doc.push_str(&format!("= {}\n\n", escape_typst(day_label)));

    for panel in panels {
        let color_str = panel
            .panel_type
            .as_ref()
            .and_then(|pt| data.panel_types.get(pt.as_str()))
            .and_then(|pt| PanelColor::resolve(&pt.colors, color_mode))
            .map(|c| c.hex)
            .unwrap_or_default();

        let time = format_time_range(panel.start_time.as_deref(), panel.end_time.as_deref());
        let room = panel
            .room_ids
            .first()
            .and_then(|uid| data.rooms.iter().find(|r| r.uid == *uid))
            .map(|r| {
                if !r.long_name.is_empty() {
                    r.long_name.as_str()
                } else {
                    r.short_name.as_str()
                }
            })
            .unwrap_or("");

        let accent = if color_str.is_empty() {
            String::new()
        } else {
            format!(
                "#rect(fill: rgb(\"{}\"), width: 4pt, height: 0.8em)#h(4pt)",
                color_str
            )
        };

        doc.push_str(&format!(
            "#block(breakable: false)[{}*{}* #h(1fr) #text(size: 8pt)[{} · {}]\n",
            accent,
            escape_typst(&panel.name),
            escape_typst(&time),
            escape_typst(room),
        ));

        if let Some(desc) = &panel.description {
            if !desc.is_empty() {
                doc.push_str(&format!("#text(size: 8pt)[{}]\n", escape_typst(desc)));
            }
        }

        if !panel.credits.is_empty() {
            doc.push_str(&format!(
                "#text(size: 7pt, style: \"italic\")[{}]\n",
                escape_typst(&panel.credits.join(", "))
            ));
        }

        doc.push_str("]\n\n");
    }

    doc.push_str("]\n");
    doc
}

fn format_time_range(start: Option<&str>, end: Option<&str>) -> String {
    let fmt = |s: &str| -> String {
        let time_part = s.get(11..).unwrap_or(s);
        let parts: Vec<&str> = time_part.splitn(2, ':').collect();
        if parts.len() < 2 {
            return time_part.to_string();
        }
        let hour: u32 = parts[0].parse().unwrap_or(0);
        let min: u32 = parts[1].get(..2).unwrap_or("0").parse().unwrap_or(0);
        let (h12, suffix) = if hour == 0 {
            (12u32, "AM")
        } else if hour < 12 {
            (hour, "AM")
        } else if hour == 12 {
            (12, "PM")
        } else {
            (hour - 12, "PM")
        };
        if min == 0 {
            format!("{} {}", h12, suffix)
        } else {
            format!("{}:{:02} {}", h12, min, suffix)
        }
    };
    match (start, end) {
        (Some(s), Some(e)) => format!("{} – {}", fmt(s), fmt(e)),
        (Some(s), None) => fmt(s),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_range() {
        assert_eq!(
            format_time_range(Some("2026-06-26T14:00:00"), Some("2026-06-26T15:00:00")),
            "2 PM – 3 PM"
        );
    }

    #[test]
    fn test_generate_empty() {
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
        let out = generate(
            &data,
            &BrandConfig::default(),
            &LayoutConfig::default(),
            ColorMode::Color,
        );
        assert!(out.is_empty());
    }
}
