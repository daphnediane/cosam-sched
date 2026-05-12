/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Workshop poster layout builder.
//!
//! Produces one Typst poster per premium or workshop panel, including title,
//! time, room, description, credits, cost, and a QR code if `ticket_url` is set.

use crate::brand::BrandConfig;
use crate::color::{ColorMode, PanelColor};
use crate::grid::LayoutConfig;
use crate::model::ScheduleData;
use crate::typst_gen::{escape_typst, preamble};

/// Generate Typst source for workshop posters.
///
/// Returns `(filename_stem, typ_source)` pairs, one per workshop panel.
pub fn generate(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
) -> Vec<(String, String)> {
    let panels = data.scheduled_panels();

    panels
        .iter()
        .filter(|p| {
            let is_workshop = p
                .panel_type
                .as_ref()
                .and_then(|pt| data.panel_types.get(pt.as_str()))
                .map(|pt| pt.is_workshop || pt.is_cafe)
                .unwrap_or(false);
            let is_premium = p.is_premium;
            let premium_only = config.filter.premium_only;
            is_workshop && (!premium_only || is_premium)
        })
        .map(|panel| {
            let stem = poster_stem(panel);
            let source = generate_poster_typ(data, brand, config, color_mode, panel);
            (stem, source)
        })
        .collect()
}

fn poster_stem(panel: &crate::model::Panel) -> String {
    let base = panel
        .name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();
    let base = base
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!("poster-{}", base)
}

fn generate_poster_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    panel: &crate::model::Panel,
) -> String {
    let mut doc = preamble(config, brand, false);

    let color_str = panel
        .panel_type
        .as_ref()
        .and_then(|pt| data.panel_types.get(pt.as_str()))
        .and_then(|pt| PanelColor::resolve(&pt.colors, color_mode))
        .map(|c| c.hex)
        .unwrap_or_else(|| brand.colors.primary.clone());

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

    let time = format_time_range(panel.start_time.as_deref(), panel.end_time.as_deref());

    // Header bar
    doc.push_str(&format!(
        "#rect(fill: rgb(\"{color}\"), width: 100%, inset: 12pt)[\n\
         #text(size: 20pt, fill: white, font: \"{heading}\")[*{title}*]\n\
         #v(4pt)\n\
         #text(size: 12pt, fill: white)[{time} · {room}]\n\
         ]\n\
         #v(1em)\n",
        color = color_str,
        heading = brand.fonts.heading_or_default(),
        title = escape_typst(&panel.name),
        time = escape_typst(&time),
        room = escape_typst(room),
    ));

    if let Some(cost) = &panel.cost {
        doc.push_str(&format!(
            "#align(right)[#text(size: 14pt, fill: rgb(\"{color}\"))[*{cost}*]]\n#v(0.5em)\n",
            color = color_str,
            cost = escape_typst(cost),
        ));
    }

    if let Some(desc) = &panel.description {
        if !desc.is_empty() {
            doc.push_str(&format!(
                "#text(size: 11pt)[{}]\n#v(1em)\n",
                escape_typst(desc)
            ));
        }
    }

    if !panel.credits.is_empty() {
        doc.push_str(&format!(
            "#text(size: 10pt, style: \"italic\")[{}]\n#v(0.5em)\n",
            escape_typst(&panel.credits.join(", "))
        ));
    }

    if let Some(url) = &panel.ticket_url {
        if !url.is_empty() {
            doc.push_str(&format!(
                "#v(1fr)\n#align(right)[\n\
                 #text(size: 9pt)[Register: {}]\n\
                 ]\n",
                escape_typst(url)
            ));
        }
    }

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
    fn test_poster_stem() {
        let panel = crate::model::Panel {
            id: "WS001".into(),
            base_id: "WS001".into(),
            name: "Intro to Sewing!".into(),
            panel_type: None,
            room_ids: vec![],
            start_time: None,
            end_time: None,
            duration: None,
            description: None,
            note: None,
            prereq: None,
            cost: None,
            capacity: None,
            difficulty: None,
            ticket_url: None,
            is_premium: false,
            is_full: false,
            is_kids: false,
            credits: vec![],
            presenters: vec![],
        };
        assert_eq!(poster_stem(&panel), "poster-intro-to-sewing");
    }
}
