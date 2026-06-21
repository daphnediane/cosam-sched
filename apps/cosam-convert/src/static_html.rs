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
//!   carrying structural data (meta, rooms, panelTypes, timeline, presenters).
//! - A `<section class="cosam-static-schedule">` containing one
//!   `<article class="cosam-panel">` per panel, with `data-*` attributes for
//!   machine-readable scalar fields and visible HTML children for text content.
//!   Hidden by CSS (`.cosam-calendar ~ .cosam-static-schedule { display:none }`)
//!   once the widget JS initializes.

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use schedule_core::widget_json::{ScheduleConfig, WidgetExport, WidgetRoom};

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

/// Generate the config script block for the widget-html embed format.
///
/// Returns a string containing the config JSON script block with branding and
/// print formats. This is placed **outside** `#cosam-calendar-root` by the caller.
#[must_use = "generated HTML must be embedded in the output document"]
pub fn generate_config_html(config: &ScheduleConfig) -> Result<String> {
    let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    Ok(format!(
        "<script type=\"application/json\" id=\"cosam-config-data\" data-cosam=\"config\">\n\
         {json}\n</script>"
    ))
}

// ── Structural JSON block ──────────────────────────────────────────────────────

fn build_structural_json(export: &WidgetExport) -> Result<String> {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "meta".to_string(),
        serde_json::to_value(&export.meta).context("Failed to serialize meta")?,
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
    // FEATURE-154: precomputed day buckets travel with the structural block.
    if !export.day_timeline.is_empty() {
        obj.insert(
            "dayTimeline".to_string(),
            serde_json::to_value(&export.day_timeline)
                .context("Failed to serialize dayTimeline")?,
        );
    }
    if !export.half_day_timeline.is_empty() {
        obj.insert(
            "halfDayTimeline".to_string(),
            serde_json::to_value(&export.half_day_timeline)
                .context("Failed to serialize halfDayTimeline")?,
        );
    }

    serde_json::to_string_pretty(&serde_json::Value::Object(obj))
        .context("Failed to serialize structural JSON block")
}

// ── Panel HTML elements ────────────────────────────────────────────────────────

fn build_panels_html(export: &WidgetExport) -> Result<String> {
    let mut html = String::new();
    for panel in &export.panels {
        let attrs = build_panel_attrs(panel);
        let children = build_panel_children(panel, &export.rooms, &export.meta.timezone);
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

    // FEATURE-154: times are emitted as canonical epoch seconds (widget format
    // v2). The human-readable wall-clock lives in the `<time>` element below.
    if let Some(se) = panel.start_epoch {
        attrs.push_str(&format!(" data-start-epoch=\"{se}\""));
    }
    if let Some(ee) = panel.end_epoch {
        attrs.push_str(&format!(" data-end-epoch=\"{ee}\""));
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
    if let Some(tp) = panel.total_parts {
        attrs.push_str(&format!(" data-total-parts=\"{tp}\""));
    }
    if panel.is_series_lead {
        attrs.push_str(" data-is-series-lead=\"true\"");
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
    // FEATURE-154: precomputed day-bucket key; omitted for unscheduled panels.
    if let Some(dk) = &panel.day_key {
        attrs.push_str(&format!(" data-day-key=\"{}\"", escape_attr(dk)));
    }

    attrs
}

fn build_panel_children(
    panel: &schedule_core::widget_json::WidgetPanel,
    rooms: &[WidgetRoom],
    tz_name: &str,
) -> String {
    let mut children = String::new();

    // Header: name, time, rooms
    let time_html = build_time_element(panel.start_epoch, panel.end_epoch, tz_name);
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

fn build_time_element(start: Option<i64>, end: Option<i64>, tz_name: &str) -> String {
    let Some(start_epoch) = start else {
        return String::new();
    };
    // The `datetime` attribute stays a local wall-clock ISO string for HTML5/SEO
    // (FEATURE-154); it is derived from the canonical epoch in the meta timezone.
    let start_str = schedule_core::value::timezone::epoch_to_local_iso(start_epoch, tz_name);
    let end_str = end.map(|e| schedule_core::value::timezone::epoch_to_local_iso(e, tz_name));

    let display = match &end_str {
        Some(end_str) => format_time_range(&start_str, end_str),
        None => format_single_time(&start_str),
    };

    format!(
        "      <time class=\"cosam-panel-time\" datetime=\"{}\">{}</time>\n",
        escape_attr(&start_str),
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
                version: 2,
                generator: "test".to_string(),
                generated: "2026-01-01T00:00:00Z".to_string(),
                modified: "2026-01-01T00:00:00Z".to_string(),
                // 2026-06-26T10:00 and 2026-06-28T18:00 in America/New_York (EDT).
                start_epoch: 1_782_482_400,
                end_epoch: 1_782_684_000,
                timezone: "America/New_York".to_string(),
                vtimezone: String::new(),
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
            day_timeline: vec![],
            half_day_timeline: vec![],
            panels: vec![WidgetPanel {
                id: "GP001".to_string(),
                base_id: "GP001".to_string(),
                name: "Test Panel".to_string(),
                panel_type: Some("GP".to_string()),
                room_ids: vec![1],
                // 2026-06-26T14:00 and 15:00 in America/New_York (EDT).
                start_epoch: Some(1_782_496_800),
                end_epoch: Some(1_782_500_400),
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
        // FEATURE-154: epoch-seconds attributes (widget format v2).
        assert!(
            html.contains("data-start-epoch=\"1782496800\""),
            "start-epoch attribute"
        );
        assert!(
            html.contains("data-end-epoch=\"1782500400\""),
            "end-epoch attribute"
        );
    }

    #[test]
    fn test_generate_html_day_key_attribute() {
        // Panel with day_key emits data-day-key; panel without does not.
        let mut export = minimal_export();
        export.panels[0].day_key = Some("2026-06-26".to_string());
        let html = generate_static_schedule_html(&export).unwrap();
        assert!(
            html.contains("data-day-key=\"2026-06-26\""),
            "data-day-key attribute present when day_key is set"
        );

        // Panel without day_key must not emit the attribute.
        export.panels[0].day_key = None;
        let html = generate_static_schedule_html(&export).unwrap();
        assert!(
            !html.contains("data-day-key"),
            "data-day-key attribute absent when day_key is None"
        );
    }

    #[test]
    fn test_generate_html_multipart_series_attributes() {
        let mut export = minimal_export();
        export.panels[0].total_parts = Some(3);
        export.panels[0].is_series_lead = true;
        let html = generate_static_schedule_html(&export).unwrap();
        assert!(
            html.contains("data-total-parts=\"3\""),
            "total-parts attribute"
        );
        assert!(
            html.contains("data-is-series-lead=\"true\""),
            "is-series-lead attribute"
        );

        // A continuation part omits the lead flag.
        export.panels[0].is_series_lead = false;
        let html = generate_static_schedule_html(&export).unwrap();
        assert!(html.contains("data-total-parts=\"3\""));
        assert!(
            !html.contains("data-is-series-lead"),
            "lead flag omitted on continuation parts"
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
