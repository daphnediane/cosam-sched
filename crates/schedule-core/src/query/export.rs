/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget JSON export functionality.
//!
//! Converts from the internal CRDT/field-system format to the widget JSON display format
//! documented in `docs/widget-json-format.md`.

use crate::entity::EntityUuid;
use crate::schedule::Schedule;
use crate::tables::event_room::{self, EventRoomEntityType, EventRoomId};
use crate::tables::hotel_room::HotelRoomEntityType;
use crate::tables::panel::{self, PanelEntityType, PanelId};
use crate::tables::panel_type::PanelTypeEntityType;
use crate::tables::presenter::{self, PresenterEntityType, PresenterId};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ── Widget JSON Structures ───────────────────────────────────────────────────────

/// Top-level metadata for widget JSON export.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetMeta {
    pub title: String,
    pub version: i32,
    pub variant: String,
    pub generator: String,
    pub generated: String,
    pub modified: String,
    pub start_time: String,
    pub end_time: String,
}

/// Panel entry in widget JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPanel {
    pub id: String,
    pub base_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_num: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_num: Option<i32>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub room_ids: Vec<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    pub duration: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prereq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket_url: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub is_free: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub is_full: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub is_kids: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub credits: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub presenters: Vec<String>,
}

/// Room entry in widget JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetRoom {
    pub uid: i32,
    pub short_name: String,
    pub long_name: String,
    pub hotel_room: String,
    pub sort_key: i32,
    pub is_break: bool,
}

/// Panel type entry in widget JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPanelType {
    pub kind: String,
    pub colors: HashMap<String, String>,
    pub is_break: bool,
    pub is_cafe: bool,
    pub is_workshop: bool,
    pub is_hidden: bool,
    pub is_room_hours: bool,
    pub is_timeline: bool,
    pub is_private: bool,
}

/// Timeline entry in widget JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetTimeline {
    pub id: String,
    pub start_time: String,
    pub description: String,
    pub panel_type: Option<String>,
    pub note: Option<String>,
}

/// Presenter entry in widget JSON format (DisplayPresenter).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPresenter {
    pub name: String,
    pub rank: String,
    pub sort_key: i32,
    pub is_group: bool,
    pub members: Vec<String>,
    pub groups: Vec<String>,
    pub always_grouped: bool,
    pub always_shown: bool,
    pub panel_ids: Vec<String>,
}

/// Complete widget JSON export structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetExport {
    pub meta: WidgetMeta,
    pub panels: Vec<WidgetPanel>,
    pub rooms: Vec<WidgetRoom>,
    pub panel_types: HashMap<String, WidgetPanelType>,
    pub timeline: Vec<WidgetTimeline>,
    pub presenters: Vec<WidgetPresenter>,
}

// ── Export Function ───────────────────────────────────────────────────────────────

/// Export schedule data to widget JSON format.
///
/// Converts from the internal CRDT/field-system format to the widget JSON display
/// format, including credit formatting, break synthesis, and bidirectional
/// presenter group membership.
pub fn export_to_widget_json(
    schedule: &Schedule,
    title: &str,
) -> Result<WidgetExport, ExportError> {
    let now = Utc::now();

    let (rooms, room_uid_map) = build_room_uid_map(schedule);
    // All rooms in `rooms` are already non-pseudo; use them all for break synthesis.
    let visible_room_uids: Vec<i32> = rooms.iter().map(|r| r.uid).collect();

    let panel_types = export_panel_types(schedule)?;
    let panels = export_panels(schedule, &room_uid_map, &visible_room_uids, &panel_types)?;
    let timeline = export_timeline(schedule, &panel_types)?;
    let presenters = export_presenters(schedule, &panels)?;

    // Only include panel types actually referenced by panels or timeline entries.
    let used_prefixes: HashSet<String> = panels
        .iter()
        .filter_map(|p| p.panel_type.clone())
        .chain(timeline.iter().filter_map(|t| t.panel_type.clone()))
        .collect();
    let panel_types: HashMap<String, WidgetPanelType> = panel_types
        .into_iter()
        .filter(|(k, _)| used_prefixes.contains(k))
        .collect();

    let (start_time, end_time) = compute_schedule_bounds(&panels, &now);

    let meta = WidgetMeta {
        title: title.to_string(),
        version: 0,
        variant: "display".to_string(),
        generator: format!("cosam-convert {}", env!("CARGO_PKG_VERSION")),
        generated: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        modified: schedule
            .metadata
            .modified_at
            .unwrap_or(schedule.metadata.created_at)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        start_time,
        end_time,
    };

    Ok(WidgetExport {
        meta,
        panels,
        rooms,
        panel_types,
        timeline,
        presenters,
    })
}

// ── Room helpers ──────────────────────────────────────────────────────────────

/// Build the widget room list and a UUID→integer-UID lookup.
///
/// Rooms are sorted by sort_key (ascending), then by room_name for stability.
/// UIDs are assigned 1-based sequentially in that order.
fn build_room_uid_map(schedule: &Schedule) -> (Vec<WidgetRoom>, HashMap<EventRoomId, i32>) {
    let mut room_list: Vec<(
        EventRoomId,
        &crate::tables::event_room::EventRoomInternalData,
    )> = schedule.iter_entities::<EventRoomEntityType>().collect();

    room_list.sort_by(|(_, a), (_, b)| {
        let sk_a = a.data.sort_key.unwrap_or(i64::MAX);
        let sk_b = b.data.sort_key.unwrap_or(i64::MAX);
        sk_a.cmp(&sk_b)
            .then(a.data.room_name.cmp(&b.data.room_name))
    });

    let mut uid_map = HashMap::new();
    let mut rooms = Vec::new();
    let mut uid_counter: i32 = 0;

    for (id, internal) in &room_list {
        // Pseudo rooms (SPLIT, BREAK, etc.) are excluded from the public output.
        // Panels assigned to them will have roomIds: [] in the export.
        if internal.data.is_pseudo {
            continue;
        }

        uid_counter += 1;
        let uid = uid_counter;
        uid_map.insert(*id, uid);

        let hotel_room = get_hotel_room_name(schedule, *id);
        let long_name = internal
            .data
            .long_name
            .clone()
            .unwrap_or_else(|| internal.data.room_name.clone());

        rooms.push(WidgetRoom {
            uid,
            short_name: internal.data.room_name.clone(),
            long_name,
            hotel_room,
            sort_key: internal.data.sort_key.unwrap_or(0) as i32,
            is_break: false,
        });
    }

    (rooms, uid_map)
}

fn get_hotel_room_name(schedule: &Schedule, event_room_id: EventRoomId) -> String {
    schedule
        .connected_field_nodes(event_room_id, event_room::EDGE_HOTEL_ROOMS)
        .into_iter()
        .find_map(|e| {
            let hr_id =
                unsafe { crate::tables::hotel_room::HotelRoomId::new_unchecked(e.entity_uuid()) };
            schedule
                .get_internal::<HotelRoomEntityType>(hr_id)
                .map(|d| d.data.hotel_room_name.clone())
        })
        .unwrap_or_default()
}

// ── Panel type export ─────────────────────────────────────────────────────────

fn export_panel_types(
    schedule: &Schedule,
) -> Result<HashMap<String, WidgetPanelType>, ExportError> {
    let mut panel_types = HashMap::new();

    for (_, internal) in schedule.iter_entities::<PanelTypeEntityType>() {
        let data = &internal.data;
        let mut colors = HashMap::new();
        if let Some(ref color) = data.color {
            colors.insert("color".to_string(), color.clone());
        }
        if let Some(ref bw) = data.bw {
            colors.insert("bw".to_string(), bw.clone());
        }
        panel_types.insert(
            data.prefix.clone(),
            WidgetPanelType {
                kind: data.panel_kind.clone(),
                colors,
                is_break: data.is_break,
                is_cafe: data.is_cafe,
                is_workshop: data.is_workshop,
                is_hidden: data.hidden,
                is_room_hours: data.is_room_hours,
                is_timeline: data.is_timeline,
                is_private: data.is_private,
            },
        );
    }

    panel_types
        .entry("%IB".to_string())
        .or_insert_with(|| WidgetPanelType {
            kind: "Implicit Break".to_string(),
            colors: [("color".to_string(), "#F5F5F5".to_string())]
                .into_iter()
                .collect(),
            is_break: true,
            is_cafe: false,
            is_workshop: false,
            is_hidden: false,
            is_room_hours: false,
            is_timeline: false,
            is_private: false,
        });

    panel_types
        .entry("%NB".to_string())
        .or_insert_with(|| WidgetPanelType {
            kind: "Overnight Break".to_string(),
            colors: [("color".to_string(), "#F5F5F5".to_string())]
                .into_iter()
                .collect(),
            is_break: true,
            is_cafe: false,
            is_workshop: false,
            is_hidden: false,
            is_room_hours: false,
            is_timeline: false,
            is_private: false,
        });

    Ok(panel_types)
}

// ── Timeline export ───────────────────────────────────────────────────────────

fn export_timeline(
    schedule: &Schedule,
    panel_types: &HashMap<String, WidgetPanelType>,
) -> Result<Vec<WidgetTimeline>, ExportError> {
    let mut timeline = Vec::new();

    for (panel_id, internal) in schedule.iter_entities::<PanelEntityType>() {
        let prefix = get_panel_type_prefix(schedule, panel_id);
        let is_timeline = prefix
            .as_deref()
            .and_then(|p| panel_types.get(p))
            .is_some_and(|pt| pt.is_timeline);
        if !is_timeline {
            continue;
        }
        let Some(start) = internal.time_slot.start_time() else {
            continue;
        };
        timeline.push(WidgetTimeline {
            id: internal.code.full_id(),
            start_time: format_naive_dt(start),
            description: internal.data.name.clone(),
            panel_type: prefix,
            note: internal.data.note.clone(),
        });
    }

    timeline.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    Ok(timeline)
}

// ── Panel export ──────────────────────────────────────────────────────────────

fn export_panels(
    schedule: &Schedule,
    room_uid_map: &HashMap<EventRoomId, i32>,
    visible_room_uids: &[i32],
    panel_types: &HashMap<String, WidgetPanelType>,
) -> Result<Vec<WidgetPanel>, ExportError> {
    let mut panels = Vec::new();

    for (panel_id, internal) in schedule.iter_entities::<PanelEntityType>() {
        let prefix = get_panel_type_prefix(schedule, panel_id);

        // Timeline panels go to the timeline array, not panels
        let is_timeline = prefix
            .as_deref()
            .and_then(|p| panel_types.get(p))
            .is_some_and(|pt| pt.is_timeline);
        if is_timeline {
            continue;
        }

        let room_ids: Vec<i32> = schedule
            .connected_field_nodes(panel_id, panel::EDGE_EVENT_ROOMS)
            .into_iter()
            .filter_map(|e| {
                let event_room_id = unsafe { EventRoomId::new_unchecked(e.entity_uuid()) };
                room_uid_map.get(&event_room_id).copied()
            })
            .collect();

        let credits = panel::compute_credits(schedule, panel_id);
        let presenter_names = individual_presenter_names(schedule, panel_id);

        let code = &internal.code;
        let start_time = internal.time_slot.start_time().map(format_naive_dt);
        let end_time = internal.time_slot.end_time().map(format_naive_dt);
        let duration = internal
            .time_slot
            .duration()
            .map_or(0, |d| d.num_minutes() as i32);

        panels.push(WidgetPanel {
            id: code.full_id(),
            base_id: code.base_id(),
            part_num: code.part_num.map(|n| n as i32),
            session_num: code.session_num.map(|n| n as i32),
            name: internal.data.name.clone(),
            panel_type: prefix,
            room_ids,
            start_time,
            end_time,
            duration,
            description: internal.data.description.clone(),
            note: internal.data.note.clone(),
            prereq: internal.data.prereq.clone(),
            cost: internal.data.cost.clone(),
            capacity: internal.data.capacity.map(|c| c.to_string()),
            difficulty: internal.data.difficulty.clone(),
            ticket_url: internal.data.ticket_url.clone(),
            is_free: internal.data.is_free,
            is_full: internal.data.is_full,
            is_kids: internal.data.is_kids,
            credits,
            presenters: presenter_names,
        });
    }

    // Sort: scheduled before unscheduled, then within scheduled:
    //   earliest start → longest duration → lowest room uid → id → name
    panels.sort_by(|a, b| match (&a.start_time, &b.start_time) {
        (Some(at), Some(bt)) => at
            .cmp(bt)
            .then_with(|| b.duration.cmp(&a.duration))
            .then_with(|| first_room_uid(a).cmp(&first_room_uid(b)))
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| a.name.cmp(&b.name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });

    synthesize_breaks(panels, visible_room_uids)
}

fn first_room_uid(p: &WidgetPanel) -> i32 {
    p.room_ids.first().copied().unwrap_or(i32::MAX)
}

fn synthesize_breaks(
    panels: Vec<WidgetPanel>,
    visible_room_uids: &[i32],
) -> Result<Vec<WidgetPanel>, ExportError> {
    if visible_room_uids.is_empty() {
        return Ok(panels);
    }

    let mut result = Vec::with_capacity(panels.len() + 8);
    let mut current_end: Option<NaiveDateTime> = None;
    let mut ib_counter: u32 = 0;
    let mut nb_counter: u32 = 0;

    for panel in panels {
        if let Some(ref start_str) = panel.start_time.clone() {
            if let Ok(start) = NaiveDateTime::parse_from_str(start_str, "%Y-%m-%dT%H:%M:%S") {
                if let Some(prev_end) = current_end {
                    if start > prev_end {
                        let gap_minutes = (start - prev_end).num_minutes() as i32;
                        let crosses_midnight = start.date() != prev_end.date();
                        let is_overnight = crosses_midnight || gap_minutes > 240;

                        if is_overnight {
                            nb_counter += 1;
                            result.push(make_break_panel(
                                format!("%NB{:03}", nb_counter),
                                "%NB",
                                prev_end,
                                start,
                                gap_minutes,
                                visible_room_uids,
                            ));
                        } else {
                            ib_counter += 1;
                            result.push(make_break_panel(
                                format!("%IB{:03}", ib_counter),
                                "%IB",
                                prev_end,
                                start,
                                gap_minutes,
                                visible_room_uids,
                            ));
                        }
                    }
                }

                // Advance current_end to max of prev and this panel's end
                if let Some(ref end_str) = panel.end_time {
                    if let Ok(end) = NaiveDateTime::parse_from_str(end_str, "%Y-%m-%dT%H:%M:%S") {
                        current_end = Some(match current_end {
                            Some(ce) if end > ce => end,
                            Some(ce) => ce,
                            None => end,
                        });
                    }
                }
            }
        }
        result.push(panel);
    }

    Ok(result)
}

fn make_break_panel(
    id: String,
    panel_type: &str,
    start: NaiveDateTime,
    end: NaiveDateTime,
    gap_minutes: i32,
    room_uids: &[i32],
) -> WidgetPanel {
    WidgetPanel {
        base_id: id.clone(),
        id,
        part_num: None,
        session_num: None,
        name: "Break".to_string(),
        panel_type: Some(panel_type.to_string()),
        room_ids: room_uids.to_vec(),
        start_time: Some(format_naive_dt(start)),
        end_time: Some(format_naive_dt(end)),
        duration: gap_minutes,
        description: None,
        note: None,
        prereq: None,
        cost: None,
        capacity: None,
        difficulty: None,
        ticket_url: None,
        is_free: true,
        is_full: false,
        is_kids: false,
        credits: Vec::new(),
        presenters: Vec::new(),
    }
}

// ── Presenter export ──────────────────────────────────────────────────────────

fn export_presenters(
    schedule: &Schedule,
    panels: &[WidgetPanel],
) -> Result<Vec<WidgetPresenter>, ExportError> {
    // Build panel code → panel ID mapping for schedule lookup
    let code_to_panel_id: HashMap<String, PanelId> = schedule
        .iter_entities::<PanelEntityType>()
        .map(|(id, data)| (data.code.full_id(), id))
        .collect();

    // For each non-break panel, compute inclusive presenters and record which panels they appear on
    let mut presenter_panel_ids: HashMap<PresenterId, Vec<String>> = HashMap::new();

    for panel in panels {
        // Skip synthesized break panels
        if panel.id.starts_with('%') {
            continue;
        }
        let Some(&panel_id) = code_to_panel_id.get(&panel.id) else {
            continue;
        };
        let panel_code = panel.id.clone();
        for p_id in inclusive_presenter_ids(schedule, panel_id) {
            presenter_panel_ids
                .entry(p_id)
                .or_default()
                .push(panel_code.clone());
        }
    }

    // Deduplicate and sort each panel list
    for ids in presenter_panel_ids.values_mut() {
        ids.sort();
        ids.dedup();
    }

    // Collect presenters that appear in at least one panel
    let mut presenters_with_data: Vec<(
        PresenterId,
        &crate::tables::presenter::PresenterInternalData,
    )> = presenter_panel_ids
        .keys()
        .filter_map(|&p_id| {
            schedule
                .get_internal::<PresenterEntityType>(p_id)
                .map(|d| (p_id, d))
        })
        .collect();

    // Sort by sort_rank, then by name for stability
    presenters_with_data.sort_by(
        |(_, a), (_, b)| match (&a.data.sort_rank, &b.data.sort_rank) {
            (Some(ra), Some(rb)) => ra.cmp(rb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.data.name.cmp(&b.data.name),
        },
    );

    let mut widget_presenters = Vec::new();

    for (sort_key, (p_id, p_data)) in presenters_with_data.iter().enumerate() {
        let is_group = p_data.data.is_explicit_group
            || !schedule
                .connected_field_nodes(*p_id, presenter::EDGE_MEMBERS)
                .is_empty();

        let members: Vec<String> = if is_group {
            schedule
                .connected_field_nodes(*p_id, presenter::EDGE_MEMBERS)
                .into_iter()
                .filter_map(|e| {
                    let m_id = unsafe { PresenterId::new_unchecked(e.entity_uuid()) };
                    schedule
                        .get_internal::<PresenterEntityType>(m_id)
                        .map(|d| d.data.name.clone())
                })
                .collect()
        } else {
            Vec::new()
        };

        // Individuals: transitive upward groups; groups: empty
        let groups: Vec<String> = if !is_group {
            schedule
                .inclusive_edges::<PresenterEntityType, PresenterEntityType>(
                    *p_id,
                    presenter::EDGE_GROUPS,
                )
                .into_iter()
                .filter_map(|g_id| {
                    schedule
                        .get_internal::<PresenterEntityType>(g_id)
                        .map(|d| d.data.name.clone())
                })
                .collect()
        } else {
            Vec::new()
        };

        widget_presenters.push(WidgetPresenter {
            name: p_data.data.name.clone(),
            rank: p_data.data.rank.as_str().to_string(),
            sort_key: sort_key as i32,
            is_group,
            members,
            groups,
            always_grouped: p_data.data.always_grouped,
            always_shown: p_data.data.always_shown_in_group,
            panel_ids: presenter_panel_ids[p_id].clone(),
        });
    }

    Ok(widget_presenters)
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Return the panel type prefix string for the given panel, if one is linked.
fn get_panel_type_prefix(schedule: &Schedule, panel_id: PanelId) -> Option<String> {
    schedule
        .connected_field_nodes(panel_id, panel::EDGE_PANEL_TYPE)
        .into_iter()
        .next()
        .and_then(|e| {
            let pt_id =
                unsafe { crate::tables::panel_type::PanelTypeId::new_unchecked(e.entity_uuid()) };
            schedule
                .get_internal::<PanelTypeEntityType>(pt_id)
                .map(|d| d.data.prefix.clone())
        })
}

/// All presenter IDs reachable from a panel via credited+uncredited edges,
/// including transitive groups and transitive members.
///
/// Mirrors the logic of `FIELD_INCLUSIVE_PRESENTERS` in panel.rs.
fn inclusive_presenter_ids(schedule: &Schedule, panel_id: PanelId) -> HashSet<PresenterId> {
    let credited = schedule.connected_field_nodes(panel_id, panel::EDGE_CREDITED_PRESENTERS);
    let uncredited = schedule.connected_field_nodes(panel_id, panel::EDGE_UNCREDITED_PRESENTERS);
    let direct = credited
        .into_iter()
        .chain(uncredited)
        .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) });

    let mut result = HashSet::new();
    for p in direct {
        result.insert(p);
        for m in schedule
            .inclusive_edges::<PresenterEntityType, PresenterEntityType>(p, presenter::EDGE_MEMBERS)
        {
            result.insert(m);
        }
        for g in schedule
            .inclusive_edges::<PresenterEntityType, PresenterEntityType>(p, presenter::EDGE_GROUPS)
        {
            result.insert(g);
        }
    }
    result
}

/// Individual (non-group) presenter names for the panel's `presenters` search field.
fn individual_presenter_names(schedule: &Schedule, panel_id: PanelId) -> Vec<String> {
    let ids = inclusive_presenter_ids(schedule, panel_id);
    let mut names: Vec<String> = ids
        .into_iter()
        .filter_map(|p_id| {
            let d = schedule.get_internal::<PresenterEntityType>(p_id)?;
            // Include only individuals: not explicitly a group, no members edge
            if d.data.is_explicit_group {
                return None;
            }
            if !schedule
                .connected_field_nodes(p_id, presenter::EDGE_MEMBERS)
                .is_empty()
            {
                return None;
            }
            Some(d.data.name.clone())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Compute schedule-wide start/end times from real (non-break) scheduled panels.
fn compute_schedule_bounds(panels: &[WidgetPanel], now: &DateTime<Utc>) -> (String, String) {
    let mut start: Option<String> = None;
    let mut end: Option<String> = None;

    for panel in panels {
        if panel.id.starts_with('%') {
            continue;
        }
        if let Some(ref st) = panel.start_time {
            start = Some(match start {
                None => st.clone(),
                Some(ref s) if st < s => st.clone(),
                Some(s) => s,
            });
        }
        if let Some(ref et) = panel.end_time {
            end = Some(match end {
                None => et.clone(),
                Some(ref e) if et > e => et.clone(),
                Some(e) => e,
            });
        }
    }

    let fallback = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    (
        start.unwrap_or_else(|| fallback.clone()),
        end.unwrap_or(fallback),
    )
}

fn format_naive_dt(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn is_false(b: &bool) -> bool {
    !b
}

// ── Error Types ─────────────────────────────────────────────────────────────────

/// Errors that can occur during widget JSON export.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Failed to access entity: {0}")]
    EntityAccess(String),

    #[error("Failed to format credits: {0}")]
    CreditFormatting(String),

    #[error("Failed to synthesize breaks: {0}")]
    BreakSynthesis(String),

    #[error("Failed to resolve group membership: {0}")]
    GroupResolution(String),
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Schedule;
    use crate::tables::event_room::EventRoomInternalData;
    use crate::tables::panel::PanelInternalData;
    use crate::tables::panel_type::PanelTypeInternalData;
    use crate::tables::presenter::PresenterInternalData;
    use crate::value::time::TimeRange;
    use crate::value::uniq_id::PanelUniqId;
    use chrono::NaiveDate;

    // ── helpers ────────────────────────────────────────────────────────────────

    fn make_event_room(
        sched: &mut Schedule,
        room_name: &str,
        long_name: Option<&str>,
        sort_key: i64,
    ) -> EventRoomId {
        let id = crate::entity::EntityId::generate();
        sched.insert(
            id,
            EventRoomInternalData {
                id,
                data: crate::tables::event_room::EventRoomCommonData {
                    room_name: room_name.to_string(),
                    long_name: long_name.map(|s| s.to_string()),
                    sort_key: Some(sort_key),
                    is_pseudo: false,
                },
            },
        );
        id
    }

    fn make_panel_type(
        sched: &mut Schedule,
        prefix: &str,
        kind: &str,
        is_timeline: bool,
    ) -> crate::tables::panel_type::PanelTypeId {
        let id = crate::entity::EntityId::generate();
        sched.insert(
            id,
            PanelTypeInternalData {
                id,
                data: crate::tables::panel_type::PanelTypeCommonData {
                    prefix: prefix.to_string(),
                    panel_kind: kind.to_string(),
                    hidden: false,
                    is_workshop: false,
                    is_break: false,
                    is_cafe: false,
                    is_room_hours: false,
                    is_timeline,
                    is_private: false,
                    color: Some("#AABBCC".to_string()),
                    bw: None,
                },
            },
        );
        id
    }

    fn make_panel(
        sched: &mut Schedule,
        code_str: &str,
        start_hms: Option<(i32, u32, u32, u32)>,
        duration_mins: Option<i64>,
    ) -> PanelId {
        let code = PanelUniqId::parse(code_str).unwrap();
        let id = crate::entity::EntityId::generate();
        let time_slot = match (start_hms, duration_mins) {
            (Some((day_offset, h, m, s)), Some(dur)) => {
                let base = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
                let date = base + chrono::Duration::days(day_offset as i64);
                let start = date.and_hms_opt(h, m, s).unwrap();
                let end = start + chrono::Duration::minutes(dur);
                TimeRange::ScheduledWithEnd {
                    start_time: start,
                    end_time_minus_start_time: end - start,
                }
            }
            _ => TimeRange::default(),
        };
        sched.insert(
            id,
            PanelInternalData {
                id,
                code,
                data: crate::tables::panel::PanelCommonData {
                    name: format!("Panel {code_str}"),
                    is_free: true,
                    ..Default::default()
                },
                time_slot,
            },
        );
        id
    }

    fn make_presenter(sched: &mut Schedule, name: &str) -> PresenterId {
        let id = crate::entity::EntityId::generate();
        sched.insert(
            id,
            PresenterInternalData {
                id,
                data: crate::tables::presenter::PresenterCommonData {
                    name: name.to_string(),
                    ..Default::default()
                },
            },
        );
        id
    }

    fn link_panel_type(
        sched: &mut Schedule,
        panel_id: PanelId,
        pt_id: crate::tables::panel_type::PanelTypeId,
    ) {
        let _ = sched.edge_set(panel_id, panel::EDGE_PANEL_TYPE, [pt_id]);
    }

    fn link_panel_room(sched: &mut Schedule, panel_id: PanelId, room_id: EventRoomId) {
        let _ = sched.edge_add(panel_id, panel::EDGE_EVENT_ROOMS, [room_id]);
    }

    fn link_credited_presenter(sched: &mut Schedule, panel_id: PanelId, presenter_id: PresenterId) {
        let _ = sched.edge_add(panel_id, panel::EDGE_CREDITED_PRESENTERS, [presenter_id]);
    }

    // ── tests ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_export_creates_valid_structure() {
        let schedule = Schedule::new();
        let result = export_to_widget_json(&schedule, "Test Schedule");
        assert!(result.is_ok());
        let export = result.unwrap();
        assert_eq!(export.meta.version, 0);
        assert_eq!(export.meta.variant, "display");
        // Empty schedule has no panels so no panel types should be emitted
        assert!(export.panel_types.is_empty());
    }

    #[test]
    fn test_export_rooms_uid_assignment() {
        let mut sched = Schedule::new();
        make_event_room(&mut sched, "RoomB", Some("Room B Long"), 2);
        make_event_room(&mut sched, "RoomA", None, 1);

        let (rooms, uid_map) = build_room_uid_map(&sched);
        assert_eq!(rooms.len(), 2);
        // RoomA has sort_key 1, RoomB has sort_key 2 → RoomA gets uid=1
        let room_a = rooms.iter().find(|r| r.short_name == "RoomA").unwrap();
        let room_b = rooms.iter().find(|r| r.short_name == "RoomB").unwrap();
        assert_eq!(room_a.uid, 1);
        assert_eq!(room_b.uid, 2);
        // long_name fallback when absent
        assert_eq!(room_a.long_name, "RoomA");
        assert_eq!(room_b.long_name, "Room B Long");
        assert_eq!(uid_map.len(), 2);
    }

    #[test]
    fn test_export_panel_types_maps_fields() {
        let mut sched = Schedule::new();
        make_panel_type(&mut sched, "GP", "Guest Panel", false);

        let result = export_panel_types(&sched).unwrap();
        assert!(result.contains_key("GP"));
        let gp = &result["GP"];
        assert_eq!(gp.kind, "Guest Panel");
        assert!(!gp.is_timeline);
        assert!(gp.colors.contains_key("color"));
        // Synthetic break types always added
        assert!(result.contains_key("%IB"));
        assert!(result.contains_key("%NB"));
        assert!(result["%IB"].is_break);
        assert!(result["%NB"].is_break);
    }

    #[test]
    fn test_export_panels_basic() {
        let mut sched = Schedule::new();
        let room_id = make_event_room(&mut sched, "WS1", Some("Workshop 1"), 1);
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let panel_id = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        link_panel_type(&mut sched, panel_id, pt_id);
        link_panel_room(&mut sched, panel_id, room_id);

        let (_, uid_map) = build_room_uid_map(&sched);
        let panel_types = export_panel_types(&sched).unwrap();
        let panels = export_panels(&sched, &uid_map, &[1], &panel_types).unwrap();

        let real: Vec<_> = panels.iter().filter(|p| !p.id.starts_with('%')).collect();
        assert_eq!(real.len(), 1);
        let p = &real[0];
        assert_eq!(p.id, "GP001");
        assert_eq!(p.base_id, "GP001");
        assert_eq!(p.panel_type.as_deref(), Some("GP"));
        assert_eq!(p.room_ids, vec![1]);
        assert_eq!(p.duration, 60);
        assert_eq!(p.start_time.as_deref(), Some("2026-06-01T14:00:00"));
        assert_eq!(p.end_time.as_deref(), Some("2026-06-01T15:00:00"));
    }

    #[test]
    fn test_export_panels_sorted_unscheduled_last() {
        let mut sched = Schedule::new();
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let p1 = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        let p2 = make_panel(&mut sched, "GP002", None, None);
        link_panel_type(&mut sched, p1, pt_id);
        link_panel_type(&mut sched, p2, pt_id);

        let (_, uid_map) = build_room_uid_map(&sched);
        let panel_types = export_panel_types(&sched).unwrap();
        let panels = export_panels(&sched, &uid_map, &[], &panel_types).unwrap();

        let real: Vec<_> = panels.iter().filter(|p| !p.id.starts_with('%')).collect();
        assert_eq!(real.len(), 2);
        // Scheduled panel must come before unscheduled
        assert!(real[0].start_time.is_some());
        assert!(real[1].start_time.is_none());
    }

    #[test]
    fn test_export_panels_break_synthesis() {
        let mut sched = Schedule::new();
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let room_id = make_event_room(&mut sched, "R1", None, 1);
        // Panel 1: 14:00–15:00, Panel 2: 15:30–16:30 → 30-min gap
        let p1 = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        let p2 = make_panel(&mut sched, "GP002", Some((0, 15, 30, 0)), Some(60));
        link_panel_type(&mut sched, p1, pt_id);
        link_panel_type(&mut sched, p2, pt_id);
        link_panel_room(&mut sched, p1, room_id);
        link_panel_room(&mut sched, p2, room_id);

        let (_, uid_map) = build_room_uid_map(&sched);
        let panel_types = export_panel_types(&sched).unwrap();
        let panels = export_panels(&sched, &uid_map, &[1], &panel_types).unwrap();

        let ids: Vec<&str> = panels.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"%IB001"), "expected %IB001 in {ids:?}");
        let ib = panels.iter().find(|p| p.id == "%IB001").unwrap();
        assert_eq!(ib.duration, 30);
        assert_eq!(ib.room_ids, vec![1]);
    }

    #[test]
    fn test_export_panels_overnight_break() {
        let mut sched = Schedule::new();
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        // Panel 1 ends 23:00 day 0, Panel 2 starts 09:00 day 1
        let p1 = make_panel(&mut sched, "GP001", Some((0, 21, 0, 0)), Some(120)); // 21:00–23:00
        let p2 = make_panel(&mut sched, "GP002", Some((1, 9, 0, 0)), Some(60)); // 09:00–10:00 next day
        link_panel_type(&mut sched, p1, pt_id);
        link_panel_type(&mut sched, p2, pt_id);

        let (_, uid_map) = build_room_uid_map(&sched);
        let panel_types = export_panel_types(&sched).unwrap();
        let panels = export_panels(&sched, &uid_map, &[1], &panel_types).unwrap();

        let ids: Vec<&str> = panels.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"%NB001"), "expected %NB001 in {ids:?}");
    }

    #[test]
    fn test_export_timeline_only() {
        let mut sched = Schedule::new();
        let tl_pt = make_panel_type(&mut sched, "SP", "Split", true);
        let gp_pt = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let tl_panel = make_panel(&mut sched, "SP001", Some((0, 8, 0, 0)), Some(0));
        let gp_panel = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        link_panel_type(&mut sched, tl_panel, tl_pt);
        link_panel_type(&mut sched, gp_panel, gp_pt);

        let panel_types = export_panel_types(&sched).unwrap();
        let (_, uid_map) = build_room_uid_map(&sched);
        let panels = export_panels(&sched, &uid_map, &[], &panel_types).unwrap();
        let timeline = export_timeline(&sched, &panel_types).unwrap();

        let real: Vec<_> = panels.iter().filter(|p| !p.id.starts_with('%')).collect();
        assert_eq!(real.len(), 1);
        assert_eq!(real[0].id, "GP001");
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].id, "SP001");
    }

    #[test]
    fn test_export_presenters_individual() {
        let mut sched = Schedule::new();
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let panel_id = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        let p_id = make_presenter(&mut sched, "Jane Doe");
        link_panel_type(&mut sched, panel_id, pt_id);
        link_credited_presenter(&mut sched, panel_id, p_id);

        let (_, uid_map) = build_room_uid_map(&sched);
        let panel_types = export_panel_types(&sched).unwrap();
        let panels = export_panels(&sched, &uid_map, &[], &panel_types).unwrap();
        let presenters = export_presenters(&sched, &panels).unwrap();

        let jane = presenters.iter().find(|p| p.name == "Jane Doe").unwrap();
        assert!(!jane.is_group);
        assert!(jane.members.is_empty());
        assert!(jane.panel_ids.contains(&"GP001".to_string()));
    }

    #[test]
    fn test_export_presenters_group() {
        let mut sched = Schedule::new();
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let panel_id = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        let member_id = make_presenter(&mut sched, "Alice");
        let group_id = {
            let id = crate::entity::EntityId::generate();
            sched.insert(
                id,
                PresenterInternalData {
                    id,
                    data: crate::tables::presenter::PresenterCommonData {
                        name: "Team Alpha".to_string(),
                        is_explicit_group: true,
                        ..Default::default()
                    },
                },
            );
            id
        };
        // Link group → member
        let _ = sched.edge_add(group_id, presenter::EDGE_MEMBERS, [member_id]);
        link_panel_type(&mut sched, panel_id, pt_id);
        link_credited_presenter(&mut sched, panel_id, group_id);

        let (_, uid_map) = build_room_uid_map(&sched);
        let panel_types = export_panel_types(&sched).unwrap();
        let panels = export_panels(&sched, &uid_map, &[], &panel_types).unwrap();
        let presenters = export_presenters(&sched, &panels).unwrap();

        let group = presenters.iter().find(|p| p.name == "Team Alpha").unwrap();
        assert!(group.is_group);
        assert!(group.members.contains(&"Alice".to_string()));
        assert!(group.panel_ids.contains(&"GP001".to_string()));

        let alice = presenters.iter().find(|p| p.name == "Alice").unwrap();
        assert!(!alice.is_group);
        assert!(alice.groups.contains(&"Team Alpha".to_string()));
        assert!(alice.panel_ids.contains(&"GP001".to_string()));
    }

    #[test]
    fn test_export_filters_unused_panel_types() {
        let mut sched = Schedule::new();
        // GP has a panel; FP has no panels → FP must be absent from output
        let gp_pt = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        make_panel_type(&mut sched, "FP", "Fan Panel", false);
        let panel_id = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        link_panel_type(&mut sched, panel_id, gp_pt);

        let export = export_to_widget_json(&sched, "Test").unwrap();
        assert!(
            export.panel_types.contains_key("GP"),
            "GP should be present"
        );
        assert!(
            !export.panel_types.contains_key("FP"),
            "FP should be absent"
        );
        // %IB/%NB should NOT be present when there are no gaps between panels
        // (single panel — no synthesized breaks)
        assert!(!export.panel_types.contains_key("%IB"));
    }

    #[test]
    fn test_export_meta_bounds() {
        let mut sched = Schedule::new();
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let p1 = make_panel(&mut sched, "GP001", Some((0, 9, 0, 0)), Some(60));
        let p2 = make_panel(&mut sched, "GP002", Some((0, 14, 0, 0)), Some(90));
        link_panel_type(&mut sched, p1, pt_id);
        link_panel_type(&mut sched, p2, pt_id);

        let export = export_to_widget_json(&sched, "Test").unwrap();
        assert!(
            export.meta.start_time.contains("09:00:00"),
            "start_time should contain 09:00:00, got {}",
            export.meta.start_time
        );
        assert!(
            export.meta.end_time.contains("15:30:00"),
            "end_time should contain 15:30:00, got {}",
            export.meta.end_time
        );
    }
}
