/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Room signs layout builder.
//!
//! Produces one Typst document per room per day listing that room's panels
//! in chronological order.  Optionally filtered to a single room UID via
//! `config.filter.room_uid`.

use crate::brand::BrandConfig;
use crate::color::{ColorMode, PanelColor};
use crate::grid::LayoutConfig;
use crate::model::ScheduleData;
use crate::typst_gen::{escape_typst, preamble};

/// Generate Typst source for room door signs.
///
/// Returns `(filename_stem, typ_source)` pairs, one per room per day.
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

    // Determine which rooms to generate signs for
    let rooms: Vec<&crate::model::Room> = data
        .sorted_rooms()
        .into_iter()
        .filter(|r| {
            !data
                .panel_types
                .values()
                .any(|_| false) // placeholder
                && config
                    .filter
                    .room_uid
                    .map(|uid| r.uid == uid)
                    .unwrap_or(true)
        })
        .collect();

    let mut out = vec![];

    for room in &rooms {
        // Panels in this room, grouped by day
        let mut days: Vec<(String, Vec<&crate::model::Panel>)> = vec![];
        for panel in &panels {
            if !panel.room_ids.contains(&room.uid) {
                continue;
            }
            if let Some(start) = &panel.start_time {
                let day = start.get(..10).unwrap_or("unknown").to_string();
                if let Some(entry) = days.iter_mut().find(|(d, _)| d == &day) {
                    entry.1.push(panel);
                } else {
                    days.push((day, vec![panel]));
                }
            }
        }

        for (day, day_panels) in &days {
            let room_slug = room
                .short_name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-");
            let day_slug = day
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>();
            let stem = format!("room-sign-{}-{}", room_slug, day_slug);
            let source = generate_sign_typ(data, brand, config, color_mode, room, day, day_panels);
            out.push((stem, source));
        }
    }

    out
}

fn generate_sign_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    room: &crate::model::Room,
    day_label: &str,
    panels: &[&crate::model::Panel],
) -> String {
    let mut doc = preamble(config, brand, false);

    let room_name = if !room.long_name.is_empty() {
        &room.long_name
    } else {
        &room.short_name
    };

    // Room header
    doc.push_str(&format!(
        "#rect(fill: rgb(\"{color}\"), width: 100%, inset: 10pt)[\n\
         #text(size: 18pt, fill: white, font: \"{heading}\")[*{room}*]\n\
         #v(2pt)\n\
         #text(size: 11pt, fill: white)[{day}]\n\
         ]\n\
         #v(1em)\n",
        color = brand.colors.primary,
        heading = brand.fonts.heading_or_default(),
        room = escape_typst(room_name),
        day = escape_typst(day_label),
    ));

    // Panel table
    doc.push_str("#table(columns: (0.8in, 1fr), align: left,\n");
    doc.push_str("  table.header([*Time*], [*Event*]),\n");

    for panel in panels {
        let time = panel
            .start_time
            .as_deref()
            .map(format_time_short)
            .unwrap_or_default();

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
                "#rect(fill: rgb(\"{}\"), width: 3pt, height: 0.7em)#h(3pt)",
                color_str
            )
        };

        doc.push_str(&format!(
            "  [{}], [{}*{}*],\n",
            escape_typst(&time),
            accent,
            escape_typst(&panel.name),
        ));
    }

    doc.push_str(")\n");
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
    fn test_format_time_short_noon() {
        assert_eq!(format_time_short("2026-06-26T12:00:00"), "12 PM");
    }

    #[test]
    fn test_format_time_short_half() {
        assert_eq!(format_time_short("2026-06-26T14:30:00"), "2:30");
    }
}
