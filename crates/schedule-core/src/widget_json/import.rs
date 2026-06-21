/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget JSON import functionality.
//!
//! This module provides functions for importing widget JSON data into a Schedule,
//! with best-effort reconstruction of entities and relationships.

use crate::edit::builder::build_entity;
use crate::entity::{EntityType, UuidPreference};
use crate::field::set::FieldUpdate;
use crate::schedule::Schedule;
use crate::tables::breaks::{self, BreakId};
use crate::tables::event_room::{self, EventRoomId};
use crate::tables::panel::{self, PanelEntityType, PanelId};
use crate::tables::panel_type::{PanelTypeEntityType, PanelTypeId};
use crate::tables::presenter::{self, PresenterId};
use crate::tables::timeline::TimelineId;
use crate::value::cost::parse_additional_cost;
use crate::value::AdditionalCost;
use chrono::Duration;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use super::export::WidgetJsonError;
use super::types::{
    WidgetExport, WidgetPanel, WidgetPanelType, WidgetPresenter, WidgetRoom, WidgetTimeline,
};

/// Import widget JSON from a file.
pub fn load_from_file(path: &Path) -> Result<WidgetExport, WidgetJsonError> {
    let json = std::fs::read_to_string(path)?;
    load_from_json(&json)
}

/// Import widget JSON from a string.
pub fn load_from_json(json: &str) -> Result<WidgetExport, WidgetJsonError> {
    Ok(serde_json::from_str(json)?)
}

/// Import widget JSON from a URL by extracting embedded gzip+base64 data.
///
/// Fetches the webpage, finds the `<script type="application/json" id="cosam-schedule-data">`
/// tag, extracts the base64-encoded gzip data, decompresses it, and parses the JSON.
pub fn load_from_url(url: &str) -> Result<WidgetExport, WidgetJsonError> {
    // Fetch the webpage with a browser-like User-Agent so that CDN/CMS systems
    // (e.g., Squarespace) serve the full page rather than a stripped bot response.
    let client = reqwest::blocking::Client::builder()
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
             AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/124.0.0.0 Safari/537.36",
        )
        .build()?;
    let response = client.get(url).send()?;
    let html = response.text()?;

    // Parse HTML and extract the embedded data.
    // Try the full selector first (with type attribute), then fall back to id-only
    // in case the host page omits or varies the type attribute.
    let document = scraper::Html::parse_document(&html);
    let selector =
        scraper::Selector::parse(r#"script[type="application/json"][id="cosam-schedule-data"]"#)
            .map_err(|e| WidgetJsonError::DataExtraction(format!("Invalid selector: {}", e)))?;
    let fallback_selector = scraper::Selector::parse(r#"script#cosam-schedule-data"#)
        .map_err(|e| WidgetJsonError::DataExtraction(format!("Invalid selector: {}", e)))?;

    let script_element = document
        .select(&selector)
        .next()
        .or_else(|| document.select(&fallback_selector).next())
        .ok_or_else(|| {
            WidgetJsonError::DataExtraction(
                "No script tag with id='cosam-schedule-data' found in webpage".to_string(),
            )
        })?;

    let encoded_data = script_element
        .text()
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string();

    if encoded_data.is_empty() {
        return Err(WidgetJsonError::DataExtraction(
            "Script tag is empty".to_string(),
        ));
    }

    // Decode and decompress the data
    let json_data = decode_gzip_base64(&encoded_data)?;

    // Parse the JSON
    load_from_json(&json_data)
}

/// Decode gzip+base64 encoded data to a JSON string.
///
/// Handles both gzip-compressed base64 data (detected by "H4sI" prefix)
/// and plain base64-encoded JSON.
fn decode_gzip_base64(encoded: &str) -> Result<String, WidgetJsonError> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use flate2::read::GzDecoder;
    use std::io::Read as _;

    let encoded = encoded.trim();

    // Decode base64
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|e| WidgetJsonError::Base64Decode(format!("{}", e)))?;

    // Check if it's gzip-compressed (H4sI is the gzip magic number in base64)
    let json_string = if encoded.starts_with("H4sI") {
        // Decompress gzip
        let mut decoder = GzDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| WidgetJsonError::GzipDecompress(format!("{}", e)))?;
        String::from_utf8(decompressed)
            .map_err(|e| WidgetJsonError::GzipDecompress(format!("Invalid UTF-8: {}", e)))?
    } else {
        // Plain base64-encoded JSON
        String::from_utf8(bytes)
            .map_err(|e| WidgetJsonError::Base64Decode(format!("Invalid UTF-8: {}", e)))?
    };

    Ok(json_string)
}

/// Best-effort import from widget JSON to a Schedule.
///
/// This function creates a Schedule from widget JSON data, but significant data
/// is lost in the conversion because widget JSON is a display format, not a full
/// data interchange format.
///
/// **Data Loss / Limitations:**
///
/// - **Hotel rooms**: Widget JSON only has `hotel_room` string per room; this function
///   synthesizes HotelRoom entities for each unique hotel_room value but loses richer
///   hotel metadata that would be in an XLSX Hotels sheet.
///
/// - **Presenter relationships**: Widget JSON flattens presenter groups into `members`
///   and `groups` arrays. This function reconstructs group membership edges but loses
///   the distinction between credited vs uncredited presenters (all presenters in
///   widget JSON are treated as credited).
///
/// - **Panel fields**: Many panel fields from XLSX are not present in widget JSON:
///   - `hide_panelist`, `sewing_machines`, `pre_reg_max`
///   - `notes_non_printing`, `workshop_notes`, `power_needs`, `av_notes`
///   - Formula cells and sidecar data (not applicable to widget JSON)
///   - Soft-delete markers (leading `*` in UNIQ_ID)
///
/// - **Alt panelist detection**: This function compares the computed credits from
///   `panel::compute_credits` (after presenters are linked) with the original credits
///   from the widget JSON. If they differ, it sets `FIELD_ALT_PANELIST` to preserve
///   the original custom credits text.
///
/// - **Panel type matching**: Widget JSON uses panel type prefixes (e.g., "GP") to
///   link panels to types. If the prefix doesn't match any existing panel type,
///   a new panel type is synthesized from the `kind` field.
///
/// - **Timeline entries**: Timeline entries in widget JSON are imported as Timeline
///   entities, but the distinction between timeline panels and regular panels in
///   the Schedule sheet (is_timeline flag) is inferred from the panel type.
///
/// - **UUIDs**: All entities get fresh UUIDs; original UUIDs are not preserved.
///
/// - **Metadata**: Schedule metadata (created_at, modified_at) is set to current time.
///
/// - **Synthesized breaks**: Widget JSON may contain synthesized break panels (%IB, %NB)
///   from the export process. These are skipped during import as they can be
///   regenerated on export.
///
/// **Use Case:**
/// This is intended for recovering schedule data when only the widget JSON export
/// is available (e.g., from a published website). For full round-trip editing, use
/// the native .schedule format or XLSX.
pub fn import_from_widget_json(widget: &WidgetExport) -> Result<Schedule, WidgetJsonError> {
    let mut schedule = Schedule::new();
    schedule.metadata.created_at = chrono::Utc::now();
    schedule.metadata.modified_at = Some(chrono::Utc::now());
    // The internal schedule stores naive wall-clock times in this zone; epoch
    // fields are converted back to wall-clock through it during import (FEATURE-154).
    let tz_name = widget.meta.timezone.clone();
    if !tz_name.is_empty() {
        schedule.metadata.timezone = Some(tz_name.clone());
    }

    // Import panel types first (needed for panel/timeline linking)
    let panel_type_map = import_panel_types(&widget.panel_types, &mut schedule)?;

    // Import rooms and synthesize hotel rooms
    let room_map = import_rooms(&widget.rooms, &mut schedule)?;

    // Import presenters with group membership
    let presenter_map = import_presenters(&widget.presenters, &mut schedule)?;

    // Import timeline entries
    import_timeline(&widget.timeline, &mut schedule, &tz_name)?;

    // Import panels and reconstruct edges
    import_panels(
        &widget.panels,
        &panel_type_map,
        &room_map,
        &presenter_map,
        &mut schedule,
        &tz_name,
    )?;

    Ok(schedule)
}

// ── Import helpers ─────────────────────────────────────────────────────────────

/// Import panel types from widget JSON, returning a prefix→PanelTypeId map.
fn import_panel_types(
    panel_types: &BTreeMap<String, WidgetPanelType>,
    schedule: &mut Schedule,
) -> Result<HashMap<String, PanelTypeId>, WidgetJsonError> {
    let mut map = HashMap::new();

    // Skip synthesized break types (%IB, %NB) - they're regenerated on export
    for (prefix, wpt) in panel_types {
        if prefix.starts_with('%') {
            continue;
        }

        let uuid_pref = UuidPreference::PreferFromV5 {
            name: prefix.to_uppercase(),
        };

        let mut updates = vec![
            FieldUpdate::set(&crate::tables::panel_type::FIELD_PREFIX, prefix.as_str()),
            FieldUpdate::set(
                &crate::tables::panel_type::FIELD_PANEL_KIND,
                wpt.kind.as_str(),
            ),
            FieldUpdate::set(&crate::tables::panel_type::FIELD_IS_BREAK, wpt.is_break),
            FieldUpdate::set(&crate::tables::panel_type::FIELD_IS_CAFE, wpt.is_cafe),
            FieldUpdate::set(
                &crate::tables::panel_type::FIELD_IS_WORKSHOP,
                wpt.is_workshop,
            ),
            FieldUpdate::set(&crate::tables::panel_type::FIELD_HIDDEN, wpt.is_hidden),
            FieldUpdate::set(
                &crate::tables::panel_type::FIELD_IS_ROOM_HOURS,
                wpt.is_room_hours,
            ),
            FieldUpdate::set(
                &crate::tables::panel_type::FIELD_IS_TIMELINE,
                wpt.is_timeline,
            ),
            FieldUpdate::set(&crate::tables::panel_type::FIELD_IS_PRIVATE, wpt.is_private),
        ];

        if let Some(color) = wpt.colors.color.as_ref() {
            updates.push(FieldUpdate::set(
                &crate::tables::panel_type::FIELD_COLOR,
                color.as_str(),
            ));
        }
        if let Some(bw) = wpt.colors.bw.as_ref() {
            updates.push(FieldUpdate::set(
                &crate::tables::panel_type::FIELD_BW,
                bw.as_str(),
            ));
        }

        let pt_id: PanelTypeId = build_entity(schedule, uuid_pref, updates).map_err(|e| {
            WidgetJsonError::EntityAccess(format!("Failed to create panel type {}: {}", prefix, e))
        })?;

        map.insert(prefix.clone(), pt_id);
    }

    Ok(map)
}

/// Import rooms from widget JSON, synthesizing hotel room entities.
/// Returns a room uid→EventRoomId map.
fn import_rooms(
    rooms: &[WidgetRoom],
    schedule: &mut Schedule,
) -> Result<HashMap<i32, EventRoomId>, WidgetJsonError> {
    let mut room_map = HashMap::new();
    let mut hotel_room_map: HashMap<String, crate::tables::hotel_room::HotelRoomId> =
        HashMap::new();

    for room in rooms {
        // Synthesize hotel room if needed
        let hotel_room_id = if !room.hotel_room.is_empty() {
            if let Some(&existing_id) = hotel_room_map.get(&room.hotel_room) {
                Some(existing_id)
            } else {
                let uuid_pref = UuidPreference::PreferFromV5 {
                    name: format!("HOTEL_{}", room.hotel_room.to_uppercase()),
                };
                let updates = vec![FieldUpdate::set(
                    &crate::tables::hotel_room::FIELD_HOTEL_ROOM_NAME,
                    room.hotel_room.as_str(),
                )];
                let id: crate::tables::hotel_room::HotelRoomId =
                    build_entity(schedule, uuid_pref, updates).map_err(|e| {
                        WidgetJsonError::EntityAccess(format!(
                            "Failed to create hotel room {}: {}",
                            room.hotel_room, e
                        ))
                    })?;
                hotel_room_map.insert(room.hotel_room.clone(), id);
                Some(id)
            }
        } else {
            None
        };

        // Create event room
        let uuid_pref = UuidPreference::PreferFromV5 {
            name: format!("ROOM_{}", room.short_name.to_uppercase()),
        };
        let mut updates = vec![
            FieldUpdate::set(
                &crate::tables::event_room::FIELD_ROOM_NAME,
                room.short_name.as_str(),
            ),
            FieldUpdate::set(
                &crate::tables::event_room::FIELD_SORT_KEY,
                room.sort_key as i64,
            ),
        ];

        if room.long_name != room.short_name {
            updates.push(FieldUpdate::set(
                &crate::tables::event_room::FIELD_LONG_NAME,
                room.long_name.as_str(),
            ));
        }

        let room_id: EventRoomId = build_entity(schedule, uuid_pref, updates).map_err(|e| {
            WidgetJsonError::EntityAccess(format!(
                "Failed to create event room {}: {}",
                room.short_name, e
            ))
        })?;

        // Link to hotel room if present
        if let Some(hotel_id) = hotel_room_id {
            let _ = schedule.edge_add(room_id, event_room::EDGE_HOTEL_ROOMS, [hotel_id]);
        }

        room_map.insert(room.uid, room_id);
    }

    Ok(room_map)
}

/// Import presenters from widget JSON, reconstructing group membership edges.
/// Returns a presenter name→PresenterId map.
fn import_presenters(
    presenters: &[WidgetPresenter],
    schedule: &mut Schedule,
) -> Result<HashMap<String, PresenterId>, WidgetJsonError> {
    let mut presenter_map = HashMap::new();
    let mut name_to_id: HashMap<String, PresenterId> = HashMap::new();

    // First pass: create all presenters
    for wp in presenters {
        let uuid_pref = UuidPreference::PreferFromV5 {
            name: format!("PRESENTER_{}", wp.name.to_uppercase()),
        };

        let mut updates = vec![
            FieldUpdate::set(&crate::tables::presenter::FIELD_NAME, wp.name.as_str()),
            FieldUpdate::set(
                &crate::tables::presenter::FIELD_IS_EXPLICIT_GROUP,
                wp.is_group,
            ),
            FieldUpdate::set(
                &crate::tables::presenter::FIELD_SUBSUMES_MEMBERS,
                wp.subsumes_members,
            ),
        ];

        // Parse rank from string - use string representation directly
        updates.push(FieldUpdate::set(
            &crate::tables::presenter::FIELD_RANK,
            wp.rank.as_str(),
        ));

        let p_id: PresenterId = build_entity(schedule, uuid_pref, updates).map_err(|e| {
            WidgetJsonError::EntityAccess(format!("Failed to create presenter {}: {}", wp.name, e))
        })?;

        presenter_map.insert(wp.name.clone(), p_id);
        name_to_id.insert(wp.name.clone(), p_id);
    }

    // Second pass: reconstruct group membership edges
    for wp in presenters {
        let p_id = presenter_map.get(&wp.name).unwrap();

        // Add members edge for groups
        if wp.is_group {
            for member_name in &wp.members {
                if let Some(&member_id) = name_to_id.get(member_name) {
                    let _ = schedule.edge_add(*p_id, presenter::EDGE_MEMBERS, [member_id]);
                }
            }
        }

        // Add groups edge for individuals
        if !wp.is_group {
            for group_name in &wp.groups {
                if let Some(&group_id) = name_to_id.get(group_name) {
                    let _ = schedule.edge_add(*p_id, presenter::EDGE_GROUPS, [group_id]);
                }
            }
        }
    }

    Ok(presenter_map)
}

/// Import timeline entries from widget JSON. Panel type is derived from the
/// Uniq ID prefix, so no panel-type map is needed.
fn import_timeline(
    timeline: &[WidgetTimeline],
    schedule: &mut Schedule,
    tz_name: &str,
) -> Result<(), WidgetJsonError> {
    for wt in timeline {
        let uuid_pref = UuidPreference::PreferFromV5 {
            name: wt.id.to_uppercase(),
        };

        let mut updates = vec![
            FieldUpdate::set(&crate::tables::timeline::FIELD_CODE, wt.id.as_str()),
            FieldUpdate::set(&crate::tables::timeline::FIELD_NAME, wt.name.as_str()),
        ];

        if let Some(ref note) = wt.note {
            updates.push(FieldUpdate::set(
                &crate::tables::timeline::FIELD_NOTE,
                note.as_str(),
            ));
        }

        if let Some(dt) = wt
            .start_epoch
            .map(|e| crate::value::timezone::epoch_to_local(e, tz_name))
        {
            updates.push(FieldUpdate::set(&crate::tables::timeline::FIELD_TIME, dt));
        }

        let _tl_id: TimelineId = build_entity(schedule, uuid_pref, updates).map_err(|e| {
            WidgetJsonError::EntityAccess(format!("Failed to create timeline {}: {}", wt.id, e))
        })?;

        // Panel type is derived from the Uniq ID prefix — no edge to link.
    }

    Ok(())
}

/// Import a single break entry (a break-typed panel) into the Break table.
fn import_break(
    wp: &WidgetPanel,
    schedule: &mut Schedule,
    tz_name: &str,
) -> Result<(), WidgetJsonError> {
    let uuid_pref = UuidPreference::PreferFromV5 {
        name: wp.id.to_uppercase(),
    };

    let mut updates = vec![
        FieldUpdate::set(&breaks::FIELD_CODE, wp.id.as_str()),
        FieldUpdate::set(&breaks::FIELD_NAME, wp.name.as_str()),
    ];

    if let Some(st) = wp
        .start_epoch
        .map(|e| crate::value::timezone::epoch_to_local(e, tz_name))
    {
        updates.push(FieldUpdate::set(&breaks::FIELD_START_TIME, st));
    }
    if wp.duration > 0 {
        updates.push(FieldUpdate::set(
            &breaks::FIELD_DURATION,
            Duration::minutes(wp.duration as i64),
        ));
    }
    if let Some(ref desc) = wp.description {
        updates.push(FieldUpdate::set(&breaks::FIELD_DESCRIPTION, desc.as_str()));
    }
    if let Some(ref note) = wp.note {
        updates.push(FieldUpdate::set(&breaks::FIELD_NOTE, note.as_str()));
    }

    let _break_id: BreakId = build_entity(schedule, uuid_pref, updates).map_err(|e| {
        WidgetJsonError::EntityAccess(format!("Failed to create break {}: {}", wp.id, e))
    })?;

    // Panel type is derived from the Uniq ID prefix — no edge to link.

    Ok(())
}

/// Import panels from widget JSON, reconstructing edges and detecting alt_panelist.
fn import_panels(
    panels: &[WidgetPanel],
    panel_type_map: &HashMap<String, PanelTypeId>,
    room_map: &HashMap<i32, EventRoomId>,
    presenter_map: &HashMap<String, PresenterId>,
    schedule: &mut Schedule,
    tz_name: &str,
) -> Result<(), WidgetJsonError> {
    for wp in panels {
        // Skip synthesized break panels (%IB/%NB) - they're regenerated on export.
        if wp.id.starts_with('%') {
            continue;
        }

        // Real breaks ride in the panels array but are stored as their own
        // entity (FEATURE-144); route break-typed entries into the Break table.
        let is_break = wp
            .panel_type
            .as_deref()
            .and_then(|p| panel_type_map.get(p))
            .and_then(|&pt_id| schedule.get_internal::<PanelTypeEntityType>(pt_id))
            .map(|d| d.data.is_break)
            .unwrap_or(false);
        if is_break {
            import_break(wp, schedule, tz_name)?;
            continue;
        }

        let uuid_pref = UuidPreference::PreferFromV5 {
            name: wp.id.to_uppercase(),
        };

        let mut updates = vec![
            FieldUpdate::set(&crate::tables::panel::FIELD_CODE, wp.id.as_str()),
            FieldUpdate::set(&crate::tables::panel::FIELD_NAME, wp.name.as_str()),
            FieldUpdate::set(&crate::tables::panel::FIELD_IS_FULL, wp.is_full),
            FieldUpdate::set(&crate::tables::panel::FIELD_FOR_KIDS, wp.is_kids),
        ];

        // Resolve timing: epoch → naive wall-clock in the schedule's timezone.
        let start_time = wp
            .start_epoch
            .map(|e| crate::value::timezone::epoch_to_local(e, tz_name));
        let _end_time = wp
            .end_epoch
            .map(|e| crate::value::timezone::epoch_to_local(e, tz_name));
        let duration = if wp.duration > 0 {
            Some(Duration::minutes(wp.duration as i64))
        } else {
            None
        };

        if let Some(st) = start_time {
            updates.push(FieldUpdate::set(
                &crate::tables::panel::FIELD_START_TIME,
                st,
            ));
        }
        if let Some(dur) = duration {
            updates.push(FieldUpdate::set(&crate::tables::panel::FIELD_DURATION, dur));
        }

        // Parse cost
        if let Some(ref cost_str) = wp.cost {
            let cost = parse_additional_cost(cost_str).unwrap_or(AdditionalCost::Included);
            updates.push(FieldUpdate::set(
                &crate::tables::panel::FIELD_ADDITIONAL_COST,
                cost,
            ));
        }

        // Parse capacity
        if let Some(ref cap_str) = wp.capacity {
            if let Ok(cap) = cap_str.parse::<i64>() {
                updates.push(FieldUpdate::set(&crate::tables::panel::FIELD_CAPACITY, cap));
            }
        }

        // Optional fields
        if let Some(ref desc) = wp.description {
            updates.push(FieldUpdate::set(
                &crate::tables::panel::FIELD_DESCRIPTION,
                desc.as_str(),
            ));
        }
        if let Some(ref note) = wp.note {
            updates.push(FieldUpdate::set(
                &crate::tables::panel::FIELD_NOTE,
                note.as_str(),
            ));
        }
        if let Some(ref prereq) = wp.prereq {
            updates.push(FieldUpdate::set(
                &crate::tables::panel::FIELD_PREREQ,
                prereq.as_str(),
            ));
        }
        if let Some(ref diff) = wp.difficulty {
            updates.push(FieldUpdate::set(
                &crate::tables::panel::FIELD_DIFFICULTY,
                diff.as_str(),
            ));
        }
        if let Some(ref url) = wp.ticket_url {
            updates.push(FieldUpdate::set(
                &crate::tables::panel::FIELD_TICKET_URL,
                url.as_str(),
            ));
        }

        // Store original credits for later comparison
        let original_credits = wp.credits.clone();

        let panel_id: PanelId = build_entity(schedule, uuid_pref, updates).map_err(|e| {
            WidgetJsonError::EntityAccess(format!("Failed to create panel {}: {}", wp.id, e))
        })?;

        // Panel type is derived from the Uniq ID prefix — no edge to link.

        // Link to rooms
        if !wp.room_ids.is_empty() {
            let room_ids: Vec<EventRoomId> = wp
                .room_ids
                .iter()
                .filter_map(|&uid| room_map.get(&uid).copied())
                .collect();
            if !room_ids.is_empty() {
                let _ = schedule.edge_add(panel_id, panel::EDGE_EVENT_ROOMS, room_ids);
            }
        }

        // Link to presenters (all treated as credited)
        let presenter_ids: Vec<PresenterId> = wp
            .presenters
            .iter()
            .filter_map(|name| presenter_map.get(name).copied())
            .collect();
        if !presenter_ids.is_empty() {
            let _ = schedule.edge_add(panel_id, panel::EDGE_CREDITED_PRESENTERS, presenter_ids);
        }

        // Compare computed credits with original credits to detect alt_panelist
        if !original_credits.is_empty() {
            let computed_credits = crate::tables::panel::compute_credits(schedule, panel_id);

            // Compare as sets to handle different ordering
            let computed_set: std::collections::HashSet<_> = computed_credits.iter().collect();
            let original_set: std::collections::HashSet<_> = original_credits.iter().collect();

            if computed_set != original_set {
                // Credits differ - set alt_panelist to preserve original
                let update = FieldUpdate::set(
                    &crate::tables::panel::FIELD_ALT_PANELIST,
                    original_credits.join(", ").as_str(),
                );
                let _ = PanelEntityType::field_set().write_multiple(panel_id, schedule, &[update]);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::panel::PanelEntityType;
    use crate::widget_json::types::{
        WidgetExport, WidgetMeta, WidgetPanel, WidgetPanelColors, WidgetPanelType, WidgetPresenter,
        WidgetRoom,
    };
    use std::collections::BTreeMap;

    #[test]
    fn test_decode_gzip_base64_plain() {
        // Test plain base64-encoded JSON (not gzip)
        let json = r#"{"test":"value"}"#;
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let encoded = STANDARD.encode(json.as_bytes());
        let decoded = decode_gzip_base64(&encoded).unwrap();
        assert_eq!(decoded, json);
    }

    #[test]
    fn test_decode_gzip_base64_compressed() {
        // Test gzip+base64 encoded JSON
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write as _;

        let json = r#"{"test":"value","nested":{"key":"data"}}"#;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        let encoded = STANDARD.encode(&compressed);

        // Should start with H4sI (gzip magic number in base64)
        assert!(encoded.starts_with("H4sI"));

        let decoded = decode_gzip_base64(&encoded).unwrap();
        assert_eq!(decoded, json);
    }

    #[test]
    fn test_decode_gzip_base64_invalid_base64() {
        let result = decode_gzip_base64("invalid!base64");
        assert!(result.is_err());
        matches!(result.unwrap_err(), WidgetJsonError::Base64Decode(_));
    }

    #[test]
    fn test_import_from_widget_json_basic() {
        // Test basic import with minimal data
        let mut panel_types = BTreeMap::new();
        panel_types.insert(
            "GP".to_string(),
            WidgetPanelType {
                prefix: "GP".to_string(),
                kind: "Guest Panel".to_string(),
                colors: WidgetPanelColors::default(),
                is_break: false,
                is_cafe: false,
                is_workshop: false,
                is_hidden: false,
                is_room_hours: false,
                is_timeline: false,
                is_private: false,
            },
        );

        let rooms = vec![WidgetRoom {
            uid: 1,
            short_name: "Room1".to_string(),
            long_name: "Room 1".to_string(),
            hotel_room: "Hotel".to_string(),
            sort_key: 1,
            is_break: false,
        }];

        let presenters = vec![WidgetPresenter {
            name: "Alice".to_string(),
            is_group: false,
            subsumes_members: false,
            rank: "guest".to_string(),
            sort_key: 1,
            groups: Vec::new(),
            members: Vec::new(),
            panel_ids: Vec::new(),
        }];

        let panels = vec![WidgetPanel {
            id: "GP001".to_string(),
            base_id: "GP001".to_string(),
            part_num: None,
            session_num: None,
            total_parts: None,
            is_series_lead: false,
            name: "Test Panel".to_string(),
            panel_type: Some("GP".to_string()),
            room_ids: vec![1],
            presenters: vec!["Alice".to_string()],
            // 2026-01-01T10:00:00Z and 11:00:00Z (UTC; meta has no timezone).
            start_epoch: Some(1_767_261_600),
            end_epoch: Some(1_767_265_200),
            duration: 60,
            is_full: false,
            is_kids: false,
            is_premium: false,
            description: None,
            note: None,
            cost: None,
            capacity: None,
            credits: Vec::new(),
            prereq: None,
            difficulty: None,
            ticket_url: None,
            day_key: None,
        }];

        let widget = WidgetExport {
            meta: WidgetMeta {
                title: "Test Schedule".to_string(),
                version: 1,
                generator: "cosam-convert".to_string(),
                generated: "2026-01-01T00:00:00Z".to_string(),
                modified: "2026-01-01T00:00:00Z".to_string(),
                start_epoch: 0,
                end_epoch: 0,
                timezone: String::new(),
                vtimezone: String::new(),
                ..Default::default()
            },
            panel_types,
            rooms,
            presenters,
            timeline: Vec::new(),
            panels,
            day_timeline: Vec::new(),
            half_day_timeline: Vec::new(),
        };

        let schedule = import_from_widget_json(&widget).unwrap();
        assert_eq!(schedule.entity_count::<PanelEntityType>(), 1);
    }

    #[test]
    fn test_import_alt_panelist_matching_credits() {
        // Test that alt_panelist is NOT set when computed credits match original
        let mut panel_types = BTreeMap::new();
        panel_types.insert(
            "GP".to_string(),
            WidgetPanelType {
                prefix: "GP".to_string(),
                kind: "Guest Panel".to_string(),
                colors: WidgetPanelColors::default(),
                is_break: false,
                is_cafe: false,
                is_workshop: false,
                is_hidden: false,
                is_room_hours: false,
                is_timeline: false,
                is_private: false,
            },
        );

        let presenters = vec![WidgetPresenter {
            name: "Alice".to_string(),
            is_group: false,
            subsumes_members: false,
            rank: "guest".to_string(),
            sort_key: 1,
            groups: Vec::new(),
            members: Vec::new(),
            panel_ids: Vec::new(),
        }];

        let panels = vec![WidgetPanel {
            id: "GP001".to_string(),
            base_id: "GP001".to_string(),
            part_num: None,
            session_num: None,
            total_parts: None,
            is_series_lead: false,
            name: "Test Panel".to_string(),
            panel_type: Some("GP".to_string()),
            room_ids: Vec::new(),
            presenters: vec!["Alice".to_string()],
            start_epoch: None,
            end_epoch: None,
            duration: 0,
            is_full: false,
            is_kids: false,
            is_premium: false,
            description: None,
            note: None,
            cost: None,
            capacity: None,
            credits: vec!["Alice".to_string()],
            prereq: None,
            difficulty: None,
            ticket_url: None,
            day_key: None,
        }];

        let widget = WidgetExport {
            meta: WidgetMeta {
                title: "Test Schedule".to_string(),
                version: 1,
                generator: "cosam-convert".to_string(),
                generated: "2026-01-01T00:00:00Z".to_string(),
                modified: "2026-01-01T00:00:00Z".to_string(),
                start_epoch: 0,
                end_epoch: 0,
                timezone: String::new(),
                vtimezone: String::new(),
                ..Default::default()
            },
            panel_types,
            rooms: Vec::new(),
            presenters,
            timeline: Vec::new(),
            panels,
            day_timeline: Vec::new(),
            half_day_timeline: Vec::new(),
        };

        let schedule = import_from_widget_json(&widget).unwrap();
        let (panel_id, _) = schedule.iter_entities::<PanelEntityType>().next().unwrap();

        // Check that alt_panelist is NOT set (credits match)
        let panel_data = schedule.get_internal(panel_id).unwrap();
        assert!(panel_data.data.alt_panelist.is_none());
    }

    #[test]
    fn test_import_alt_panelist_differing_credits() {
        // Test that alt_panelist IS set when computed credits differ from original
        let mut panel_types = BTreeMap::new();
        panel_types.insert(
            "GP".to_string(),
            WidgetPanelType {
                prefix: "GP".to_string(),
                kind: "Guest Panel".to_string(),
                colors: WidgetPanelColors::default(),
                is_break: false,
                is_cafe: false,
                is_workshop: false,
                is_hidden: false,
                is_room_hours: false,
                is_timeline: false,
                is_private: false,
            },
        );

        let presenters = vec![WidgetPresenter {
            name: "Alice".to_string(),
            is_group: false,
            subsumes_members: false,
            rank: "guest".to_string(),
            sort_key: 1,
            groups: Vec::new(),
            members: Vec::new(),
            panel_ids: Vec::new(),
        }];

        let panels = vec![WidgetPanel {
            id: "GP001".to_string(),
            base_id: "GP001".to_string(),
            part_num: None,
            session_num: None,
            total_parts: None,
            is_series_lead: false,
            name: "Test Panel".to_string(),
            panel_type: Some("GP".to_string()),
            room_ids: Vec::new(),
            presenters: vec!["Alice".to_string()],
            start_epoch: None,
            end_epoch: None,
            duration: 0,
            is_full: false,
            is_kids: false,
            is_premium: false,
            description: None,
            note: None,
            cost: None,
            capacity: None,
            credits: vec!["Custom Credits Text".to_string()],
            prereq: None,
            difficulty: None,
            ticket_url: None,
            day_key: None,
        }];

        let widget = WidgetExport {
            meta: WidgetMeta {
                title: "Test Schedule".to_string(),
                version: 1,
                generator: "cosam-convert".to_string(),
                generated: "2026-01-01T00:00:00Z".to_string(),
                modified: "2026-01-01T00:00:00Z".to_string(),
                start_epoch: 0,
                end_epoch: 0,
                timezone: String::new(),
                vtimezone: String::new(),
                ..Default::default()
            },
            panel_types,
            rooms: Vec::new(),
            presenters,
            timeline: Vec::new(),
            panels,
            day_timeline: Vec::new(),
            half_day_timeline: Vec::new(),
        };

        let schedule = import_from_widget_json(&widget).unwrap();
        let (panel_id, _) = schedule.iter_entities::<PanelEntityType>().next().unwrap();

        // Check that alt_panelist IS set (credits differ)
        let panel_data = schedule.get_internal(panel_id).unwrap();
        assert!(panel_data.data.alt_panelist.is_some());
        assert_eq!(
            panel_data.data.alt_panelist.as_ref().unwrap(),
            "Custom Credits Text"
        );
    }

    #[test]
    fn test_import_alt_panelist_different_order() {
        // Test that alt_panelist is NOT set when credits have different order but same elements
        let mut panel_types = BTreeMap::new();
        panel_types.insert(
            "GP".to_string(),
            WidgetPanelType {
                prefix: "GP".to_string(),
                kind: "Guest Panel".to_string(),
                colors: WidgetPanelColors::default(),
                is_break: false,
                is_cafe: false,
                is_workshop: false,
                is_hidden: false,
                is_room_hours: false,
                is_timeline: false,
                is_private: false,
            },
        );

        let presenters = vec![
            WidgetPresenter {
                name: "Alice".to_string(),
                is_group: false,
                subsumes_members: false,
                rank: "guest".to_string(),
                sort_key: 1,
                groups: Vec::new(),
                members: Vec::new(),
                panel_ids: Vec::new(),
            },
            WidgetPresenter {
                name: "Bob".to_string(),
                is_group: false,
                subsumes_members: false,
                rank: "guest".to_string(),
                sort_key: 2,
                groups: Vec::new(),
                members: Vec::new(),
                panel_ids: Vec::new(),
            },
        ];

        let panels = vec![WidgetPanel {
            id: "GP001".to_string(),
            base_id: "GP001".to_string(),
            part_num: None,
            session_num: None,
            total_parts: None,
            is_series_lead: false,
            name: "Test Panel".to_string(),
            panel_type: Some("GP".to_string()),
            room_ids: Vec::new(),
            presenters: vec!["Alice".to_string(), "Bob".to_string()],
            start_epoch: None,
            end_epoch: None,
            duration: 0,
            is_full: false,
            is_kids: false,
            is_premium: false,
            description: None,
            note: None,
            cost: None,
            capacity: None,
            // Credits in different order than presenters
            credits: vec!["Bob".to_string(), "Alice".to_string()],
            prereq: None,
            difficulty: None,
            ticket_url: None,
            day_key: None,
        }];

        let widget = WidgetExport {
            meta: WidgetMeta {
                title: "Test Schedule".to_string(),
                version: 1,
                generator: "cosam-convert".to_string(),
                generated: "2026-01-01T00:00:00Z".to_string(),
                modified: "2026-01-01T00:00:00Z".to_string(),
                start_epoch: 0,
                end_epoch: 0,
                timezone: String::new(),
                vtimezone: String::new(),
                ..Default::default()
            },
            panel_types,
            rooms: Vec::new(),
            presenters,
            timeline: Vec::new(),
            panels,
            day_timeline: Vec::new(),
            half_day_timeline: Vec::new(),
        };

        let schedule = import_from_widget_json(&widget).unwrap();
        let (panel_id, _) = schedule.iter_entities::<PanelEntityType>().next().unwrap();

        // Check that alt_panelist is NOT set (same credits, different order)
        let panel_data = schedule.get_internal(panel_id).unwrap();
        assert!(panel_data.data.alt_panelist.is_none());
    }
}
