/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Guest postcard layout builder.
//!
//! Produces one 4×6 postcard per presenter (guest) per half-day listing
//! only the panels they appear in.  Optionally filtered to a single guest
//! via `config.filter.guest_name`.

use crate::brand::BrandConfig;
use crate::color::{ColorMode, PanelColor};
use crate::grid::LayoutConfig;
use crate::model::ScheduleData;
use crate::typst_gen::{escape_typst, preamble};

/// Generate Typst source for guest personal schedule postcards.
///
/// Returns `(filename_stem, typ_source)` pairs, one per guest per half-day.
pub fn generate(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
) -> Vec<(String, String)> {
    let panels = data.scheduled_panels();
    if panels.is_empty() || data.presenters.is_empty() {
        return vec![];
    }

    let mut out = vec![];

    for presenter in &data.presenters {
        if let Some(filter) = &config.filter.guest_name {
            if !presenter.name.eq_ignore_ascii_case(filter) {
                continue;
            }
        }

        // Collect panels this presenter appears in
        let guest_panels: Vec<&crate::model::Panel> = panels
            .iter()
            .copied()
            .filter(|p| p.presenters.iter().any(|n| n == &presenter.name))
            .collect();

        if guest_panels.is_empty() {
            continue;
        }

        // Group by half-day (day + AM/PM)
        let mut halves: Vec<(String, Vec<&crate::model::Panel>)> = vec![];
        for panel in &guest_panels {
            if let Some(start) = &panel.start_time {
                let day = start.get(..10).unwrap_or("unknown");
                let hour: u32 = start.get(11..13).and_then(|h| h.parse().ok()).unwrap_or(0);
                let half = if hour < 12 {
                    format!("{} AM", day)
                } else {
                    format!("{} PM", day)
                };
                if let Some(entry) = halves.iter_mut().find(|(h, _)| h == &half) {
                    entry.1.push(panel);
                } else {
                    halves.push((half, vec![panel]));
                }
            }
        }

        for (half_label, half_panels) in &halves {
            let guest_slug = presenter
                .name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-");
            let half_slug = half_label
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>();
            let stem = format!("postcard-{}-{}", guest_slug, half_slug);
            let source = generate_postcard_typ(
                data,
                brand,
                config,
                color_mode,
                presenter,
                half_label,
                half_panels,
            );
            out.push((stem, source));
        }
    }

    out
}

fn generate_postcard_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    presenter: &crate::model::Presenter,
    half_label: &str,
    panels: &[&crate::model::Panel],
) -> String {
    let mut doc = preamble(config, brand, false);

    // Header
    doc.push_str(&format!(
        "#rect(fill: rgb(\"{color}\"), width: 100%, inset: 8pt)[\n\
         #text(size: 14pt, fill: white, font: \"{heading}\")[*{name}*]\n\
         #v(2pt)\n\
         #text(size: 9pt, fill: white)[{half}]\n\
         ]\n\
         #v(0.5em)\n",
        color = brand.colors.primary,
        heading = brand.fonts.heading_or_default(),
        name = escape_typst(&presenter.name),
        half = escape_typst(half_label),
    ));

    for panel in panels {
        let time = panel
            .start_time
            .as_deref()
            .map(format_time_short)
            .unwrap_or_default();
        let room = panel
            .room_ids
            .first()
            .and_then(|uid| data.rooms.iter().find(|r| r.uid == *uid))
            .map(|r| r.short_name.as_str())
            .unwrap_or("");
        let color_str = panel
            .panel_type
            .as_ref()
            .and_then(|pt| data.panel_types.get(pt.as_str()))
            .and_then(|pt| PanelColor::resolve(&pt.colors, color_mode))
            .map(|c| c.hex)
            .unwrap_or_default();
        let accent = if color_str.is_empty() {
            String::new()
        } else {
            format!(
                "#rect(fill: rgb(\"{}\"), width: 3pt, height: 0.6em)#h(3pt)",
                color_str
            )
        };

        doc.push_str(&format!(
            "#block(breakable: false)[{}*{}* #h(1fr) #text(size: 8pt)[{} · {}]]\n#v(0.3em)\n",
            accent,
            escape_typst(&panel.name),
            escape_typst(&time),
            escape_typst(room),
        ));
    }

    if let Some(url) = &brand.meta.site_url {
        doc.push_str(&format!(
            "#v(1fr)\n#align(center)[#text(size: 7pt)[{}]]\n",
            escape_typst(url)
        ));
    }

    doc
}

fn format_time_short(s: &str) -> String {
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
        format!("{}:{:02}", h12, min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_no_presenters_empty() {
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
