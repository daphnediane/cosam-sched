/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Static HTML generation for the widget-html embedded format.
//!
//! Produces the static schedule fragments for the widget-html embed format,
//! documented in `docs/widget-html-format.md`. Both fragments are placed
//! **outside** `#cosam-calendar-root` so they survive the widget's initial
//! render (which clears `rootEl.innerHTML`):
//! - A compact `<script id="cosam-schedule-data" data-cosam="schedule">` block
//!   carrying structural data (meta, rooms, panelTypes, timeline, presenters)
//!   with `meta.variant` set to `"html-embedded"`.
//! - A `<section class="cosam-static-schedule">` containing one
//!   `<article class="cosam-panel">` per panel, with `data-*` attributes for
//!   machine-readable scalar fields and visible HTML children for text content.
//!   Hidden by CSS (`.cosam-calendar ~ .cosam-static-schedule { display:none }`)
//!   once the widget JS initializes.

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use schedule_core::widget_json::{WidgetExport, WidgetMeta, WidgetRoom};

const HTML_EMBEDDED_VARIANT: &str = "html-embedded";

/// Generate the static schedule fragments for the widget-html embed format.
///
/// Returns a string containing the structural JSON script block followed by
/// the `<section>` of panel articles. Both elements are placed **outside**
/// `#cosam-calendar-root` by the caller (`embed.rs`).
#[must_use = "generated HTML must be embedded in the output document"]
pub fn generate_static_schedule_html(export: &WidgetExport) -> Result<String> {
    let json_block = build_structural_json(export)?;
    let panels_html = build_panels_html(export)?;

    Ok(format!(
        "<script type=\"application/json\" id=\"cosam-schedule-data\" data-cosam=\"schedule\">\n\
         {json_block}\n</script>\n\
         <section class=\"cosam-static-schedule\" aria-label=\"Schedule\">\n{panels_html}</section>"
    ))
}

// ── Structural JSON block ──────────────────────────────────────────────────────

fn build_structural_json(export: &WidgetExport) -> Result<String> {
    let meta = WidgetMeta {
        variant: HTML_EMBEDDED_VARIANT.to_string(),
        ..export.meta.clone()
    };

    let mut obj = serde_json::Map::new();
    obj.insert(
        "meta".to_string(),
        serde_json::to_value(&meta).context("Failed to serialize meta")?,
    );
    obj.insert(
        "rooms".to_string(),
        serde_json::to_value(&export.rooms).context("Failed to serialize rooms")?,
    );
    obj.insert(
        "panelTypes".to_string(),
        serde_json::to_value(&export.panel_types).context("Failed to serialize panelTypes")?,
    );
    obj.insert(
        "timeline".to_string(),
        serde_json::to_value(&export.timeline).context("Failed to serialize timeline")?,
    );
    obj.insert(
        "presenters".to_string(),
        serde_json::to_value(&export.presenters).context("Failed to serialize presenters")?,
    );

    serde_json::to_string_pretty(&serde_json::Value::Object(obj))
        .context("Failed to serialize structural JSON block")
}

// ── Panel HTML elements ────────────────────────────────────────────────────────

fn build_panels_html(export: &WidgetExport) -> Result<String> {
    let mut html = String::new();
    for panel in &export.panels {
        let attrs = build_panel_attrs(panel);
        let children = build_panel_children(panel, &export.rooms);
        html.push_str(&format!(
            "  <article class=\"cosam-panel\"{attrs}>\n{children}  </article>\n"
        ));
    }
    Ok(html)
}

fn build_panel_attrs(panel: &schedule_core::widget_json::WidgetPanel) -> String {
    let mut attrs = String::new();

    attrs.push_str(&format!(" data-id=\"{}\"", escape_attr(&panel.id)));
    attrs.push_str(&format!(
        " data-base-id=\"{}\"",
        escape_attr(&panel.base_id)
    ));

    if let Some(pt) = &panel.panel_type {
        attrs.push_str(&format!(" data-panel-type=\"{}\"", escape_attr(pt)));
    }

    let room_ids_str = panel
        .room_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    attrs.push_str(&format!(" data-room-ids=\"{room_ids_str}\""));

    if let Some(st) = &panel.start_time {
        attrs.push_str(&format!(" data-start-time=\"{}\"", escape_attr(st)));
    }
    if let Some(et) = &panel.end_time {
        attrs.push_str(&format!(" data-end-time=\"{}\"", escape_attr(et)));
    }
    attrs.push_str(&format!(" data-duration=\"{}\"", panel.duration));
    attrs.push_str(&format!(" data-is-premium=\"{}\"", panel.is_premium));
    attrs.push_str(&format!(" data-is-full=\"{}\"", panel.is_full));
    attrs.push_str(&format!(" data-is-kids=\"{}\"", panel.is_kids));

    if let Some(pn) = panel.part_num {
        attrs.push_str(&format!(" data-part-num=\"{pn}\""));
    }
    if let Some(sn) = panel.session_num {
        attrs.push_str(&format!(" data-session-num=\"{sn}\""));
    }
    if let Some(cost) = &panel.cost {
        attrs.push_str(&format!(" data-cost=\"{}\"", escape_attr(cost)));
    }
    if let Some(cap) = &panel.capacity {
        attrs.push_str(&format!(" data-capacity=\"{}\"", escape_attr(cap)));
    }
    if let Some(diff) = &panel.difficulty {
        attrs.push_str(&format!(" data-difficulty=\"{}\"", escape_attr(diff)));
    }
    if let Some(url) = &panel.ticket_url {
        attrs.push_str(&format!(" data-ticket-url=\"{}\"", escape_attr(url)));
    }

    attrs
}

fn build_panel_children(
    panel: &schedule_core::widget_json::WidgetPanel,
    rooms: &[WidgetRoom],
) -> String {
    let mut children = String::new();

    // Header: name, time, rooms
    let time_html = build_time_element(&panel.start_time, &panel.end_time);
    let rooms_text = build_rooms_text(&panel.room_ids, rooms);

    children.push_str("    <header>\n");
    children.push_str(&format!(
        "      <h3 class=\"cosam-panel-name\">{}</h3>\n",
        escape_html(&panel.name)
    ));
    children.push_str(&time_html);
    if !rooms_text.is_empty() {
        children.push_str(&format!(
            "      <p class=\"cosam-panel-rooms\">{}</p>\n",
            escape_html(&rooms_text)
        ));
    }
    children.push_str("    </header>\n");

    // Description, note, prereq
    if let Some(desc) = &panel.description {
        if !desc.is_empty() {
            children.push_str(&format!(
                "    <p class=\"cosam-panel-desc\">{}</p>\n",
                escape_html(desc)
            ));
        }
    }
    if let Some(note) = &panel.note {
        if !note.is_empty() {
            children.push_str(&format!(
                "    <p class=\"cosam-panel-note\">{}</p>\n",
                escape_html(note)
            ));
        }
    }
    if let Some(prereq) = &panel.prereq {
        if !prereq.is_empty() {
            children.push_str(&format!(
                "    <p class=\"cosam-panel-prereq\">{}</p>\n",
                escape_html(prereq)
            ));
        }
    }

    // Credits list
    if !panel.credits.is_empty() {
        let items: String = panel
            .credits
            .iter()
            .map(|c| format!("      <li>{}</li>\n", escape_html(c)))
            .collect();
        children.push_str(&format!(
            "    <ul class=\"cosam-panel-credits\">\n{items}    </ul>\n"
        ));
    }

    children
}

// ── Time formatting ───────────────────────────────────────────────────────────

fn build_time_element(start: &Option<String>, end: &Option<String>) -> String {
    let Some(start_str) = start else {
        return String::new();
    };

    let display = match end {
        Some(end_str) => format_time_range(start_str, end_str),
        None => format_single_time(start_str),
    };

    format!(
        "      <time class=\"cosam-panel-time\" datetime=\"{}\">{}</time>\n",
        escape_attr(start_str),
        escape_html(&display)
    )
}

fn format_time_range(start: &str, end: &str) -> String {
    match (parse_naive_dt(start), parse_naive_dt(end)) {
        (Some(s), Some(e)) => format!(
            "{} \u{2013} {}",
            s.format("%A, %-I:%M %p"),
            e.format("%-I:%M %p")
        ),
        (Some(s), None) => s.format("%A, %-I:%M %p").to_string(),
        _ => format!("{start} \u{2013} {end}"),
    }
}

fn format_single_time(s: &str) -> String {
    parse_naive_dt(s)
        .map(|dt| dt.format("%A, %-I:%M %p").to_string())
        .unwrap_or_else(|| s.to_string())
}

fn parse_naive_dt(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
}

// ── Room lookup ───────────────────────────────────────────────────────────────

fn build_rooms_text(room_ids: &[i32], rooms: &[WidgetRoom]) -> String {
    room_ids
        .iter()
        .filter_map(|id| rooms.iter().find(|r| r.uid == *id))
        .map(|r| r.long_name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

// ── HTML escaping ─────────────────────────────────────────────────────────────

/// Escape a string for use as HTML text content.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escape a string for use inside a double-quoted HTML attribute value.
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use schedule_core::widget_json::{WidgetMeta, WidgetPanel, WidgetRoom};
    use std::collections::BTreeMap;

    fn minimal_export() -> WidgetExport {
        WidgetExport {
            meta: WidgetMeta {
                title: "Test Schedule".to_string(),
                version: 0,
                variant: "display".to_string(),
                generator: "test".to_string(),
                generated: "2026-01-01T00:00:00Z".to_string(),
                modified: "2026-01-01T00:00:00Z".to_string(),
                start_time: "2026-06-26T10:00:00".to_string(),
                end_time: "2026-06-28T18:00:00".to_string(),
            },
            rooms: vec![WidgetRoom {
                uid: 1,
                short_name: "Main".to_string(),
                long_name: "Main Hall".to_string(),
                hotel_room: "Salon A".to_string(),
                sort_key: 1,
                is_break: false,
            }],
            panel_types: BTreeMap::new(),
            timeline: vec![],
            presenters: vec![],
            panels: vec![WidgetPanel {
                id: "GP001".to_string(),
                base_id: "GP001".to_string(),
                name: "Test Panel".to_string(),
                panel_type: Some("GP".to_string()),
                room_ids: vec![1],
                start_time: Some("2026-06-26T14:00:00".to_string()),
                end_time: Some("2026-06-26T15:00:00".to_string()),
                duration: 60,
                description: Some("A test panel.".to_string()),
                credits: vec!["Presenter One".to_string()],
                ..Default::default()
            }],
        }
    }

    #[test]
    fn test_generate_html_contains_script_block() {
        let export = minimal_export();
        let html = generate_static_schedule_html(&export).unwrap();
        assert!(
            html.contains("data-cosam=\"schedule\""),
            "should contain schedule script block"
        );
        assert!(
            html.contains("\"html-embedded\""),
            "variant should be html-embedded"
        );
        assert!(
            !html.contains("\"panels\""),
            "structural JSON block must not contain panels key"
        );
    }

    #[test]
    fn test_generate_html_panel_attributes() {
        let export = minimal_export();
        let html = generate_static_schedule_html(&export).unwrap();
        assert!(html.contains("data-id=\"GP001\""), "panel id attribute");
        assert!(
            html.contains("data-panel-type=\"GP\""),
            "panel type attribute"
        );
        assert!(html.contains("data-room-ids=\"1\""), "room ids attribute");
        assert!(html.contains("data-duration=\"60\""), "duration attribute");
        assert!(
            html.contains("data-is-premium=\"false\""),
            "is-premium attribute"
        );
    }

    #[test]
    fn test_generate_html_panel_content() {
        let export = minimal_export();
        let html = generate_static_schedule_html(&export).unwrap();
        assert!(
            html.contains("cosam-panel-name"),
            "panel name class present"
        );
        assert!(html.contains("Test Panel"), "panel name text content");
        assert!(html.contains("Main Hall"), "room name visible");
        assert!(html.contains("A test panel."), "description text");
        assert!(html.contains("Presenter One"), "credit text");
        assert!(html.contains("cosam-panel-credits"), "credits class");
    }

    #[test]
    fn test_generate_html_time_format() {
        let export = minimal_export();
        let html = generate_static_schedule_html(&export).unwrap();
        // Should contain the time element with datetime attribute
        assert!(
            html.contains("datetime=\"2026-06-26T14:00:00\""),
            "time datetime attribute"
        );
        // Should contain human-readable day name
        assert!(html.contains("Friday"), "day name in formatted time");
        // Should contain en-dash separator
        assert!(html.contains('\u{2013}'), "en-dash in time range");
    }

    #[test]
    fn test_escape_html_special_chars() {
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_html("normal"), "normal");
    }

    #[test]
    fn test_escape_attr_special_chars() {
        assert_eq!(escape_attr("say \"hi\""), "say &quot;hi&quot;");
        assert_eq!(escape_attr("a & b"), "a &amp; b");
        assert_eq!(escape_attr("<val>"), "&lt;val>");
    }

    #[test]
    fn test_build_rooms_text_lookup() {
        let rooms = vec![
            WidgetRoom {
                uid: 1,
                long_name: "Room A".to_string(),
                ..Default::default()
            },
            WidgetRoom {
                uid: 2,
                long_name: "Room B".to_string(),
                ..Default::default()
            },
        ];
        assert_eq!(build_rooms_text(&[1, 2], &rooms), "Room A, Room B");
        assert_eq!(build_rooms_text(&[2], &rooms), "Room B");
        assert_eq!(build_rooms_text(&[], &rooms), "");
        assert_eq!(build_rooms_text(&[99], &rooms), ""); // unknown uid
    }

    #[test]
    fn test_section_container_present() {
        let export = minimal_export();
        let html = generate_static_schedule_html(&export).unwrap();
        assert!(html.contains("cosam-static-schedule"), "section class");
        assert!(
            html.contains("aria-label=\"Schedule\""),
            "aria-label on section"
        );
        assert!(html.contains("cosam-panel"), "article class");
    }
}
