/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Panel descriptions listing layout builder.
//!
//! Produces one Typst document per day listing all scheduled panels with
//! their time, room, title, description, workshop notices, and cross-references
//! in a multi-column layout. Letter-sized output is portrait; all other paper
//! sizes are landscape.

use std::collections::{HashMap, HashSet};

use schedule_core::value::uniq_id::PanelUniqId;

use crate::brand::BrandConfig;
use crate::color::{ColorMode, PanelColor};
use crate::grid::{LayoutConfig, PaperSize};
use crate::model::{Panel, ScheduleData};
use crate::typst_gen::{day_label_to_stem, escape_typst, make_day_label, preamble};

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

    // Collect sorted unique day strings (YYYY-MM-DD)
    let mut days: Vec<(String, Vec<&Panel>)> = vec![];
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

    // Collect owned day strings for smart label logic before consuming `days`
    let all_day_strs: Vec<String> = days.iter().map(|(d, _)| d.clone()).collect();
    let all_day_refs: Vec<&str> = all_day_strs.iter().map(String::as_str).collect();

    days.into_iter()
        .map(|(day, day_panels)| {
            let label = make_day_label(&day, &all_day_refs);
            let stem = format!("description-{}", day_label_to_stem(&label));
            let source =
                generate_day_typ(data, brand, config, color_mode, &label, &day, &day_panels);
            (stem, source)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Per-day document
// ---------------------------------------------------------------------------

fn generate_day_typ(
    data: &ScheduleData,
    brand: &BrandConfig,
    config: &LayoutConfig,
    color_mode: ColorMode,
    heading: &str,
    day_date: &str,
    panels: &[&Panel],
) -> String {
    // Letter uses portrait; all other paper sizes use landscape.
    let landscape = config.paper != PaperSize::Letter;
    let num_cols = config.paper.description_columns(landscape);

    let mut doc = preamble(config, brand, landscape);

    // Build base_id → panels lookup for cross-reference resolution
    let mut by_base: HashMap<&str, Vec<&Panel>> = HashMap::new();
    for p in panels.iter().copied() {
        by_base.entry(p.base_id.as_str()).or_default().push(p);
    }

    // Banner: logo (if configured) + day heading
    let banner = if let Some(path) = brand.meta.logo_path.as_ref().and_then(|p| p.to_str()) {
        format!(
            "#grid(columns: (auto, 1fr), align: (left + horizon, right + horizon), \
             image(\"{path}\", height: 0.55in), [= {heading}])\n\n",
            path = path.replace('\\', "/"),
            heading = escape_typst(heading),
        )
    } else {
        format!("= {}\n\n", escape_typst(heading))
    };

    // Multi-column layout (count driven by paper size)
    doc.push_str(&format!(
        "#set columns({n})\n#columns({n})[\n",
        n = num_cols
    ));
    doc.push_str(&banner);

    // Group panels by start-time slot, deduplicate by id
    let mut time_groups: Vec<(String, Vec<&Panel>)> = vec![];
    let mut seen_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for panel in panels.iter().copied() {
        if !seen_ids.insert(panel.id.as_str()) {
            continue;
        }
        // Key on YYYY-MM-DDTHH:MM (first 16 chars)
        let key = panel
            .start_time
            .as_deref()
            .and_then(|s| s.get(..16))
            .unwrap_or("")
            .to_string();
        if let Some(group) = time_groups.iter_mut().find(|(k, _)| k == &key) {
            group.1.push(panel);
        } else {
            time_groups.push((key, vec![panel]));
        }
    }

    for (time_key, group) in &time_groups {
        let slot_label = format_time_only(time_key);
        if !slot_label.is_empty() {
            doc.push_str(&format!("== {}\n\n", escape_typst(&slot_label)));
        }
        for panel in group {
            doc.push_str(&panel_block(data, color_mode, panel, day_date, &by_base));
        }
    }

    doc.push_str("]\n");
    doc
}

// ---------------------------------------------------------------------------
// Shared block renderer (used by room_signs and other composing formats)
// ---------------------------------------------------------------------------

/// Generate Typst content for a slice of panels' description blocks.
///
/// Returns raw Typst markup (no document wrapper, no column or page setup)
/// suitable for embedding inside a `#grid()` cell or `#columns()` section.
/// Panels are deduplicated by `id` and grouped by time slot with a sub-heading.
///
/// `day_date` is the `YYYY-MM-DD` string for the day, used to suppress the
/// weekday prefix on same-day cross-references.
pub(crate) fn render_description_blocks<'a>(
    data: &'a ScheduleData,
    color_mode: ColorMode,
    panels: &[&'a Panel],
    day_date: &str,
) -> String {
    let mut by_base: HashMap<&'a str, Vec<&'a Panel>> = HashMap::new();
    for p in panels.iter().copied() {
        by_base.entry(p.base_id.as_str()).or_default().push(p);
    }

    let mut time_groups: Vec<(String, Vec<&Panel>)> = vec![];
    let mut seen_ids: HashSet<&str> = HashSet::new();
    for panel in panels.iter().copied() {
        if !seen_ids.insert(panel.id.as_str()) {
            continue;
        }
        let key = panel
            .start_time
            .as_deref()
            .and_then(|s| s.get(..16))
            .unwrap_or("")
            .to_string();
        if let Some(group) = time_groups.iter_mut().find(|(k, _)| k == &key) {
            group.1.push(panel);
        } else {
            time_groups.push((key, vec![panel]));
        }
    }

    let mut out = String::new();
    for (time_key, group) in &time_groups {
        let slot_label = format_time_only(time_key);
        if !slot_label.is_empty() {
            out.push_str(&format!("== {}\n\n", escape_typst(&slot_label)));
        }
        for panel in group {
            out.push_str(&panel_block(data, color_mode, panel, day_date, &by_base));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Panel block
// ---------------------------------------------------------------------------

fn panel_block<'a>(
    data: &'a ScheduleData,
    color_mode: ColorMode,
    panel: &'a Panel,
    day_date: &str,
    by_base: &HashMap<&'a str, Vec<&'a Panel>>,
) -> String {
    let color_str = panel
        .panel_type
        .as_ref()
        .and_then(|pt| data.panel_types.get(pt.as_str()))
        .and_then(|pt| PanelColor::resolve(&pt.colors, color_mode))
        .map(|c| c.hex)
        .unwrap_or_default();

    let time_range = format_time_range(panel.start_time.as_deref(), panel.end_time.as_deref());

    let room_str = panel
        .room_ids
        .iter()
        .filter_map(|uid| data.rooms.iter().find(|r| r.uid == *uid))
        .map(|r| {
            if !r.long_name.is_empty() {
                r.long_name.as_str()
            } else {
                r.short_name.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    let accent = if color_str.is_empty() {
        String::new()
    } else {
        format!(
            "#rect(fill: rgb(\"{}\"), width: 4pt, height: 0.8em)#h(4pt)",
            color_str
        )
    };

    // Right column: room \ time \ cost (Typst line-break inside cell)
    let right_items = build_right_column(&room_str, &time_range, panel.cost.as_deref());

    // Left column second row: credits (italic)
    let credits_line = if !panel.credits.is_empty() {
        format!(
            "\n  #text(size: 8pt, style: \"italic\")[{}]",
            escape_typst(&panel.credits.join(", "))
        )
    } else {
        String::new()
    };

    // Header grid: 1fr left (name + credits), auto right (room/time/cost stacked)
    let mut block = format!(
        "#block(breakable: false)[\n\
         #grid(columns: (1fr, auto), align: (top + left, top + right),\n\
           [{accent}*{name}*{credits}],\n\
           [#text(size: 8pt)[{right}]],\n\
         )\n",
        accent = accent,
        name = escape_typst(&panel.name),
        credits = credits_line,
        right = right_items,
    );

    // Description
    let desc_text = panel
        .description
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("Description pending");
    block.push_str(&format!(
        "\n#text(size: 8pt)[{}]\n",
        escape_typst(desc_text)
    ));

    // Notes / workshop notice block
    let notice = workshop_cap_notice(data, panel);
    let has_notice = notice.is_some()
        || panel.note.as_deref().is_some_and(|n| !n.is_empty())
        || panel.is_full
        || panel.difficulty.as_deref().is_some_and(|d| !d.is_empty());

    if has_notice {
        let mut note_parts: Vec<String> = vec![];
        if let Some(n) = notice {
            note_parts.push(n);
        }
        if let Some(note) = panel.note.as_deref().filter(|n| !n.is_empty()) {
            note_parts.push(format!("#text(style: \"italic\")[{}]", escape_typst(note)));
        }
        if panel.is_full {
            note_parts.push(escape_typst("This workshop is full."));
        }
        if let Some(diff) = panel.difficulty.as_deref().filter(|d| !d.is_empty()) {
            note_parts.push(escape_typst(&format!("Difficulty level: {}", diff)));
        }
        block.push_str(&format!("\n#text(size: 8pt)[{}]\n", note_parts.join(" ")));
    }

    // Prereq block
    if let Some(prereq) = panel.prereq.as_deref().filter(|p| !p.is_empty()) {
        let prereq_content = resolve_prereq(prereq, day_date, &data.panels);
        block.push_str(&format!("\n#text(size: 8pt)[{}]\n", prereq_content));
    }

    // Cross-references (parts and reruns)
    let xrefs = build_cross_refs(panel, by_base);
    for xref in &xrefs {
        block.push_str(&format!("\n#text(size: 8pt)[{}]\n", escape_typst(xref)));
    }

    block.push_str("]\n\n");
    block
}

// ---------------------------------------------------------------------------
// Header right-column builder
// ---------------------------------------------------------------------------

/// Build the stacked right-column content for the panel header grid.
///
/// Items are joined with Typst's `\ ` line-break so they stack vertically.
fn build_right_column(room: &str, time_range: &str, cost: Option<&str>) -> String {
    let mut parts: Vec<String> = vec![];
    if !room.is_empty() {
        parts.push(escape_typst(room));
    }
    if !time_range.is_empty() {
        parts.push(escape_typst(time_range));
    }
    if let Some(c) = cost.filter(|c| !c.is_empty()) {
        parts.push(format!("*{}*", escape_typst(c)));
    }
    parts.join(" \\ \n")
}

// ---------------------------------------------------------------------------
// Workshop / capacity notice
// ---------------------------------------------------------------------------

/// Generate the bold workshop/premium/capacity notice string, or `None`.
fn workshop_cap_notice(data: &ScheduleData, panel: &Panel) -> Option<String> {
    let cap_suffix = panel
        .capacity
        .as_deref()
        .filter(|c| !c.is_empty())
        .map(|c| format!(" (Capacity: {})", c))
        .unwrap_or_default();

    let is_workshop = panel
        .panel_type
        .as_ref()
        .and_then(|pt| data.panel_types.get(pt.as_str()))
        .is_some_and(|pt| pt.is_workshop);

    if panel.is_premium {
        Some(format!(
            "*Premium workshop:*{} Requires a separate purchase.",
            cap_suffix
        ))
    } else if is_workshop {
        Some(format!("*Workshop:*{}", cap_suffix))
    } else if panel.capacity.as_deref().is_some_and(|c| !c.is_empty()) {
        Some(format!("*Limited space:*{}", cap_suffix))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Prereq resolution
// ---------------------------------------------------------------------------

/// Resolve the `prereq` field into a Typst-safe string.
///
/// Tokens that parse as a valid `PanelUniqId` and match a panel are shown as
/// `"Prereq: Panel Name: Saturday 4:00 PM"`.  Unresolved tokens are shown as
/// italic text.
fn resolve_prereq(prereq: &str, day_date: &str, all_panels: &[Panel]) -> String {
    let tokens: Vec<&str> = prereq
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let mut resolved: Vec<String> = vec![];
    let mut unresolved: Vec<&str> = vec![];

    for token in &tokens {
        if let Some(uid) = PanelUniqId::parse(token) {
            let base = uid.base_id();
            let full = uid.full_id();
            let found = all_panels
                .iter()
                .find(|p| p.id == full)
                .or_else(|| all_panels.iter().find(|p| p.base_id == base));
            if let Some(p) = found {
                let time_label = p
                    .start_time
                    .as_deref()
                    .map(|t| format_weekday_time(t, day_date))
                    .unwrap_or_default();
                resolved.push(escape_typst(&format!("Prereq: {}: {}", p.name, time_label)));
            } else {
                unresolved.push(token);
            }
        } else {
            unresolved.push(token);
        }
    }

    let mut parts: Vec<String> = resolved;
    if !unresolved.is_empty() {
        parts.push(format!(
            "#text(style: \"italic\")[{}]",
            escape_typst(&unresolved.join("; "))
        ));
    }
    parts.join(" ")
}

// ---------------------------------------------------------------------------
// Cross-references (parts & reruns)
// ---------------------------------------------------------------------------

/// Build cross-reference lines for a panel (parts or rerun sessions).
fn build_cross_refs<'a>(
    panel: &'a Panel,
    by_base: &HashMap<&'a str, Vec<&'a Panel>>,
) -> Vec<String> {
    let related: &[&Panel] = by_base
        .get(panel.base_id.as_str())
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let others: Vec<&Panel> = related
        .iter()
        .copied()
        .filter(|p| p.id != panel.id)
        .collect();

    if others.is_empty() {
        return vec![];
    }

    let mut refs: Vec<String> = vec![];

    if panel.part_num.is_some() {
        // Group other panels by part_num
        let mut by_part: HashMap<i32, Vec<&Panel>> = HashMap::new();
        for p in &others {
            by_part.entry(p.part_num.unwrap_or(1)).or_default().push(p);
        }
        let mut part_keys: Vec<i32> = by_part.keys().copied().collect();
        part_keys.sort_unstable();
        for part in part_keys {
            let mut sessions = by_part[&part].clone();
            sessions.sort_by_key(|p| p.start_time.as_deref().unwrap_or(""));
            let mut first = true;
            for p in sessions {
                let label = if first {
                    format!("Part {}", part)
                } else {
                    format!("or Part {}", part)
                };
                first = false;
                let time_str = p
                    .start_time
                    .as_deref()
                    .map(|t| format_weekday_time(t, ""))
                    .unwrap_or_default();
                refs.push(format!("{}: {}", label, time_str));
            }
        }
    } else if panel.session_num.is_some() {
        let mut sorted = others.clone();
        sorted.sort_by_key(|p| p.start_time.as_deref().unwrap_or(""));
        for p in sorted {
            let time_str = p
                .start_time
                .as_deref()
                .map(|t| format_weekday_time(t, ""))
                .unwrap_or_default();
            refs.push(format!("Rerun at: {}", time_str));
        }
    }

    refs
}

// ---------------------------------------------------------------------------
// Time formatting helpers
// ---------------------------------------------------------------------------

/// Format a datetime or time string's time component as `"5 PM"` or `"5:30 PM"`.
fn format_time_only(datetime_str: &str) -> String {
    let time_part = datetime_str.get(11..).unwrap_or(datetime_str);
    let parts: Vec<&str> = time_part.splitn(2, ':').collect();
    if parts.len() < 2 {
        return String::new();
    }
    let hour: u32 = parts[0].parse().unwrap_or(0);
    let min: u32 = parts[1].get(..2).unwrap_or("0").parse().unwrap_or(0);
    let (h12, suffix) = if hour == 0 {
        (12u32, "AM")
    } else if hour < 12 {
        (hour, "AM")
    } else if hour == 12 {
        (12u32, "PM")
    } else {
        (hour - 12, "PM")
    };
    if min == 0 {
        format!("{} {}", h12, suffix)
    } else {
        format!("{}:{:02} {}", h12, min, suffix)
    }
}

/// Format start–end as `"5 PM – 6 PM"` or `"5:30 PM – 7 PM"`.
fn format_time_range(start: Option<&str>, end: Option<&str>) -> String {
    match (start, end) {
        (Some(s), Some(e)) => format!("{} – {}", format_time_only(s), format_time_only(e)),
        (Some(s), None) => format_time_only(s),
        _ => String::new(),
    }
}

/// Format a datetime as `"Saturday 4:00 PM"` for cross-reference labels.
///
/// When the cross-reference is on the same day as `current_day_date`, the
/// weekday is omitted (returns just the time).
fn format_weekday_time(datetime_str: &str, current_day_date: &str) -> String {
    use chrono::NaiveDate;

    let date_str = datetime_str.get(..10).unwrap_or("");
    let time_str = format_time_only(datetime_str);

    if date_str.is_empty() || date_str == current_day_date {
        return time_str;
    }

    let weekday = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map(|d| d.format("%A").to_string())
        .unwrap_or_default();

    if weekday.is_empty() {
        time_str
    } else {
        format!("{} {}", weekday, time_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Meta, ScheduleData};

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
            panel_types: std::collections::HashMap::new(),
            timeline: vec![],
            presenters: vec![],
        }
    }

    // -- time formatting --

    #[test]
    fn test_format_time_range() {
        assert_eq!(
            format_time_range(Some("2026-06-25T17:00:00"), Some("2026-06-25T18:00:00")),
            "5 PM – 6 PM"
        );
        assert_eq!(
            format_time_range(Some("2026-06-25T21:30:00"), Some("2026-06-25T23:00:00")),
            "9:30 PM – 11 PM"
        );
        assert_eq!(
            format_time_range(Some("2026-06-26T14:00:00"), Some("2026-06-26T15:00:00")),
            "2 PM – 3 PM"
        );
        assert_eq!(
            format_time_range(Some("2026-06-25T00:00:00"), None),
            "12 AM"
        );
    }

    #[test]
    fn test_format_weekday_time_cross_day() {
        assert_eq!(
            format_weekday_time("2026-06-27T14:00:00", "2026-06-26"),
            "Saturday 2 PM"
        );
    }

    #[test]
    fn test_format_weekday_time_same_day() {
        assert_eq!(
            format_weekday_time("2026-06-27T14:00:00", "2026-06-27"),
            "2 PM"
        );
    }

    // -- day label --

    #[test]
    fn test_day_label_single_week() {
        let days = ["2026-06-25", "2026-06-26", "2026-06-27", "2026-06-28"];
        assert_eq!(make_day_label("2026-06-25", &days), "Thursday");
        assert_eq!(make_day_label("2026-06-27", &days), "Saturday");
    }

    #[test]
    fn test_day_label_multi_week_same_month() {
        // Two weekends in June, different ISO weeks
        let days = ["2026-06-19", "2026-06-20", "2026-06-26", "2026-06-27"];
        assert_eq!(make_day_label("2026-06-26", &days), "Friday 26");
        assert_eq!(make_day_label("2026-06-19", &days), "Friday 19");
    }

    #[test]
    fn test_day_label_cross_month() {
        let days = ["2026-06-27", "2026-07-04"];
        assert_eq!(make_day_label("2026-06-27", &days), "Saturday Jun 27");
        assert_eq!(make_day_label("2026-07-04", &days), "Saturday Jul 4");
    }

    // -- workshop notice --

    #[test]
    fn test_workshop_notice_premium() {
        let data = empty_schedule();
        let panel = Panel {
            id: "WS001P1".into(),
            base_id: "WS001".into(),
            name: "Test Workshop".into(),
            is_premium: true,
            capacity: Some("12".into()),
            ..Panel::default()
        };
        let notice = workshop_cap_notice(&data, &panel);
        assert!(notice.is_some());
        let n = notice.unwrap();
        assert!(n.contains("Premium workshop:"));
        assert!(n.contains("Capacity: 12"));
        assert!(n.contains("Requires a separate purchase."));
    }

    #[test]
    fn test_workshop_notice_none_for_free_panel() {
        let data = empty_schedule();
        let panel = Panel {
            id: "GP001".into(),
            base_id: "GP001".into(),
            name: "Free Panel".into(),
            ..Panel::default()
        };
        assert!(workshop_cap_notice(&data, &panel).is_none());
    }

    // -- generate empty --

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
