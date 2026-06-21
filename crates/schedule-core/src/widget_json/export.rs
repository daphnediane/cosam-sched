/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget JSON export functionality.
//!
//! This module provides functions for exporting Schedule data to the widget JSON
//! display format, including credit formatting, break synthesis, and bidirectional
//! presenter group membership.

use crate::entity::EntityUuid;
use crate::schedule::Schedule;
use crate::tables::breaks::BreakEntityType;
use crate::tables::event_room::{self, EventRoomEntityType, EventRoomId};
use crate::tables::fields::note::NoteKind;
use crate::tables::hotel_room::HotelRoomEntityType;
use crate::tables::panel::{self, PanelEntityType, PanelId};
use crate::tables::panel_type::PanelTypeEntityType;
use crate::tables::presenter::{self, PresenterEntityType, PresenterId};
use crate::tables::timeline::TimelineEntityType;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use thiserror::Error;

use super::types::{
    WidgetDaySpan, WidgetExport, WidgetMeta, WidgetPanel, WidgetPanelColors, WidgetPanelType,
    WidgetPresenter, WidgetRoom, WidgetTimeline,
};

/// Errors that can occur during widget JSON export/import.
#[derive(Debug, Error)]
pub enum WidgetJsonError {
    #[error("Failed to access entity: {0}")]
    EntityAccess(String),

    #[error("Failed to format credits: {0}")]
    CreditFormatting(String),

    #[error("Failed to synthesize breaks: {0}")]
    BreakSynthesis(String),

    #[error("Failed to resolve group membership: {0}")]
    GroupResolution(String),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Failed to extract embedded data from webpage: {0}")]
    DataExtraction(String),

    #[error("Failed to decode base64 data: {0}")]
    Base64Decode(String),

    #[error("Failed to decompress gzip data: {0}")]
    GzipDecompress(String),
}

/// Export widget JSON to a file.
pub fn save_to_file(widget: &WidgetExport, path: &Path) -> Result<(), WidgetJsonError> {
    let json = serde_json::to_string_pretty(widget)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Export widget JSON to a string.
pub fn save_to_json(widget: &WidgetExport) -> Result<String, WidgetJsonError> {
    Ok(serde_json::to_string_pretty(widget)?)
}

/// Export schedule data to widget JSON format.
///
/// Converts from the internal CRDT/field-system format to the widget JSON display
/// format, including credit formatting, break synthesis, and bidirectional
/// presenter group membership.
///
/// When `private_export` is true, includes private panels, timeline panels, and
/// uncredited presenters that are normally excluded from public exports.
pub fn export_to_widget_json(
    schedule: &Schedule,
    title: &str,
    private_export: bool,
) -> Result<WidgetExport, WidgetJsonError> {
    let now = Utc::now();

    // Timezone the schedule's naive wall-clock times are expressed in. Resolved
    // up front (FEATURE-154) because every time field is now emitted as epoch
    // seconds, which requires the zone to convert from wall-clock.
    let timezone = schedule.metadata.timezone.clone().unwrap_or_default();

    let (rooms, room_uid_map) = build_room_uid_map(schedule);
    // All rooms in `rooms` are already non-pseudo; use them all for break synthesis.
    let visible_room_uids: Vec<i32> = rooms.iter().map(|r| r.uid).collect();

    let panel_types = export_panel_types(schedule)?;
    let panels = export_panels(
        schedule,
        &room_uid_map,
        &visible_room_uids,
        &panel_types,
        private_export,
        &timezone,
    )?;
    let timeline = export_timeline(schedule, &panel_types, private_export, &timezone)?;
    let presenters = export_presenters(schedule, &panels, private_export)?;

    // Precompute day / half-day buckets over the real (non-break) sessions so
    // consumers can group by day without re-deriving wall-clock dates from epoch.
    let is_break = |prefix: Option<&str>| {
        panel_types
            .get(prefix.unwrap_or(""))
            .map(|pt| pt.is_break)
            .unwrap_or(false)
    };
    let day_timeline = compute_day_spans(&panels, &is_break, &timezone, false);
    let half_day_timeline = compute_day_spans(&panels, &is_break, &timezone, true);

    // Only include panel types actually referenced by panels or timeline entries.
    let used_prefixes: HashSet<String> = panels
        .iter()
        .filter_map(|p| p.panel_type.clone())
        .chain(timeline.iter().filter_map(|t| t.panel_type.clone()))
        .collect();
    let panel_types: BTreeMap<String, WidgetPanelType> = panel_types
        .into_iter()
        .filter(|(k, v)| used_prefixes.contains(k) && (private_export || !v.is_private))
        .collect();

    let (start_epoch, end_epoch) =
        compute_schedule_bounds(&panels, &schedule.metadata, &now, &timezone);

    // The VTIMEZONE block is precomputed over the resolved window so the widget
    // can emit anchored .ics. It needs the wall-clock window, so the epoch bounds
    // are converted back to local time here.
    let vtimezone = crate::value::timezone::parse_tz(&timezone)
        .map(|tz| {
            crate::value::timezone::build_vtimezone(
                tz,
                crate::value::timezone::epoch_to_local(start_epoch, &timezone),
                crate::value::timezone::epoch_to_local(end_epoch, &timezone),
            )
        })
        .unwrap_or_default();

    let meta = WidgetMeta {
        title: title.to_string(),
        version: 2,
        generator: format!("cosam-convert {}", env!("CARGO_PKG_VERSION")),
        generated: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        modified: schedule
            .metadata
            .modified_at
            .unwrap_or(schedule.metadata.created_at)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        start_epoch,
        end_epoch,
        timezone,
        vtimezone,
    };

    Ok(WidgetExport {
        meta,
        panels,
        rooms,
        panel_types,
        timeline,
        presenters,
        day_timeline,
        half_day_timeline,
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
) -> Result<BTreeMap<String, WidgetPanelType>, WidgetJsonError> {
    let mut panel_types = BTreeMap::new();

    for (_, internal) in schedule.iter_entities::<PanelTypeEntityType>() {
        let data = &internal.data;
        let colors = WidgetPanelColors {
            color: data.color.clone(),
            bw: data.bw.clone(),
        };
        panel_types.insert(
            data.prefix.clone(),
            WidgetPanelType {
                prefix: data.prefix.clone(),
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
            prefix: "%IB".to_string(),
            kind: "Implicit Break".to_string(),
            colors: WidgetPanelColors {
                color: Some("#F5F5F5".to_string()),
                bw: None,
            },
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
            prefix: "%NB".to_string(),
            kind: "Overnight Break".to_string(),
            colors: WidgetPanelColors {
                color: Some("#F5F5F5".to_string()),
                bw: None,
            },
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
    panel_types: &BTreeMap<String, WidgetPanelType>,
    private_export: bool,
    tz_name: &str,
) -> Result<Vec<WidgetTimeline>, WidgetJsonError> {
    let mut timeline = Vec::new();

    for (_timeline_id, internal) in schedule.iter_entities::<TimelineEntityType>() {
        let prefix = {
            let p = internal.code.type_prefix();
            (!p.is_empty()).then(|| p.to_string())
        };

        // Private timeline panels are excluded unless private_export
        let is_private = prefix
            .as_deref()
            .and_then(|p| panel_types.get(p))
            .is_some_and(|pt| pt.is_private);
        if is_private && !private_export {
            continue;
        }

        let Some(start) = internal.data.time else {
            continue;
        };

        timeline.push(WidgetTimeline {
            id: internal.code.full_id(),
            start_epoch: Some(naive_to_epoch(start, tz_name)),
            name: internal.data.name.clone(),
            panel_type: prefix,
            note: internal.notes.get_owned(NoteKind::Public),
        });
    }

    timeline.sort_by(|a, b| a.start_epoch.cmp(&b.start_epoch));
    Ok(timeline)
}

// ── Panel export ──────────────────────────────────────────────────────────────

fn export_panels(
    schedule: &Schedule,
    room_uid_map: &HashMap<EventRoomId, i32>,
    visible_room_uids: &[i32],
    panel_types: &BTreeMap<String, WidgetPanelType>,
    private_export: bool,
    tz_name: &str,
) -> Result<Vec<WidgetPanel>, WidgetJsonError> {
    let mut panels = Vec::new();

    for (panel_id, internal) in schedule.iter_entities::<PanelEntityType>() {
        let prefix = {
            let p = internal.code.type_prefix();
            (!p.is_empty()).then(|| p.to_string())
        };

        // Private panels are excluded from public export (unless private export)
        let is_private = prefix
            .as_deref()
            .and_then(|p| panel_types.get(p))
            .is_some_and(|pt| pt.is_private);
        if is_private && !private_export {
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
        // Under a private export, surface uncredited (unlisted) presenters on the
        // panel itself so consumers (e.g. print layout) can attribute panels to
        // them. Public exports keep credited-only, as before.
        let presenter_names = individual_presenter_names(schedule, panel_id, private_export);

        let code = &internal.code;
        let start_epoch = internal
            .time_slot
            .start_time()
            .map(|dt| naive_to_epoch(dt, tz_name));
        let end_epoch = internal
            .time_slot
            .end_time()
            .map(|dt| naive_to_epoch(dt, tz_name));
        let duration = internal
            .time_slot
            .duration()
            .map_or(0, |d| d.num_minutes() as i32);

        panels.push(WidgetPanel {
            id: code.full_id(),
            base_id: code.base_id(),
            part_num: code.part_num().map(|n| n as i32),
            session_num: code.session_num().map(|n| n as i32),
            total_parts: None,
            is_series_lead: false,
            name: internal.data.name.clone(),
            panel_type: prefix,
            room_ids,
            start_epoch,
            end_epoch,
            duration,
            description: internal.data.description.clone(),
            note: internal.notes.get_owned(NoteKind::Public),
            prereq: internal.data.prereq.clone(),
            cost: crate::value::cost::additional_cost_to_string(&internal.data.additional_cost),
            capacity: internal.data.capacity.map(|c| c.to_string()),
            difficulty: internal.data.difficulty.clone(),
            ticket_url: internal.data.ticket_url.clone(),
            is_premium: matches!(
                internal.data.additional_cost,
                crate::value::AdditionalCost::TBD | crate::value::AdditionalCost::Premium(_)
            ),
            is_full: internal.data.is_full,
            is_kids: internal.data.for_kids,
            credits,
            presenters: presenter_names,
        });
    }

    // Breaks are a separate entity (FEATURE-144) but are serialized into the
    // panels array as break-typed entries (no rooms/presenters) so the widget
    // and print layout can keep rendering breaks inline. Including them here —
    // before the sort and break synthesis below — means real breaks fill gaps,
    // so %IB/%NB are only synthesized in the gaps that remain.
    for (_break_id, internal) in schedule.iter_entities::<BreakEntityType>() {
        let prefix = {
            let p = internal.code.type_prefix();
            (!p.is_empty()).then(|| p.to_string())
        };

        // Private breaks are excluded from public export (unless private export).
        let is_private = prefix
            .as_deref()
            .and_then(|p| panel_types.get(p))
            .is_some_and(|pt| pt.is_private);
        if is_private && !private_export {
            continue;
        }

        let code = &internal.code;
        let start_epoch = internal
            .time_slot
            .start_time()
            .map(|dt| naive_to_epoch(dt, tz_name));
        let end_epoch = internal
            .time_slot
            .end_time()
            .map(|dt| naive_to_epoch(dt, tz_name));
        let duration = internal
            .time_slot
            .duration()
            .map_or(0, |d| d.num_minutes() as i32);

        panels.push(WidgetPanel {
            id: code.full_id(),
            base_id: code.base_id(),
            part_num: code.part_num().map(|n| n as i32),
            session_num: code.session_num().map(|n| n as i32),
            total_parts: None,
            is_series_lead: false,
            name: internal.data.name.clone(),
            panel_type: prefix,
            room_ids: Vec::new(),
            start_epoch,
            end_epoch,
            duration,
            description: internal.data.description.clone(),
            note: internal.notes.get_owned(NoteKind::Public),
            prereq: None,
            cost: None,
            capacity: None,
            difficulty: None,
            ticket_url: None,
            is_premium: false,
            is_full: false,
            is_kids: false,
            credits: Vec::new(),
            presenters: Vec::new(),
        });
    }

    // Annotate multi-part series so a single shared cost is shown only once.
    annotate_multipart_series(&mut panels);

    // Sort: scheduled before unscheduled, then within scheduled:
    //   earliest start → longest duration → lowest room uid → id → name
    panels.sort_by(|a, b| match (a.start_epoch, b.start_epoch) {
        (Some(at), Some(bt)) => at
            .cmp(&bt)
            .then_with(|| b.duration.cmp(&a.duration))
            .then_with(|| first_room_uid(a).cmp(&first_room_uid(b)))
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| a.name.cmp(&b.name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    });

    synthesize_breaks(panels, visible_room_uids, tz_name)
}

fn first_room_uid(p: &WidgetPanel) -> i32 {
    p.room_ids.first().copied().unwrap_or(i32::MAX)
}

/// Mark multi-part panel series so a single shared cost is presented once.
///
/// Panels sharing a `base_id` and carrying a `part_num` form a series. When a
/// series spans more than one distinct part, every member gets `total_parts`
/// set, and exactly one "lead" instance — the lowest part number, breaking ties
/// by earliest start time (normally Part 1) — gets `is_series_lead = true`. The
/// lead displays the price (which covers the whole series); continuation parts
/// suppress it. Plain multi-session reruns (a single part repeated) are left
/// untouched, since their cost applies per session.
fn annotate_multipart_series(panels: &mut [WidgetPanel]) {
    use std::collections::HashMap;

    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, p) in panels.iter().enumerate() {
        if p.part_num.is_some() {
            groups.entry(p.base_id.clone()).or_default().push(i);
        }
    }

    for idxs in groups.into_values() {
        let mut distinct: Vec<i32> = idxs.iter().filter_map(|&i| panels[i].part_num).collect();
        distinct.sort_unstable();
        distinct.dedup();
        if distinct.len() < 2 {
            // A single part, possibly with reruns — not a multi-part series.
            continue;
        }
        let total = distinct.len() as i32;

        // Lead: lowest part number, then earliest start, then id for stability.
        // Missing start times sort last so a scheduled instance leads.
        let lead = *idxs
            .iter()
            .min_by(|&&a, &&b| {
                let pa = panels[a].part_num.unwrap_or(i32::MAX);
                let pb = panels[b].part_num.unwrap_or(i32::MAX);
                pa.cmp(&pb)
                    .then_with(|| {
                        let ta = panels[a].start_epoch.unwrap_or(i64::MAX);
                        let tb = panels[b].start_epoch.unwrap_or(i64::MAX);
                        ta.cmp(&tb)
                    })
                    .then_with(|| panels[a].id.cmp(&panels[b].id))
            })
            .expect("group is non-empty");

        for &i in &idxs {
            panels[i].total_parts = Some(total);
            panels[i].is_series_lead = i == lead;
        }
    }
}

fn synthesize_breaks(
    panels: Vec<WidgetPanel>,
    visible_room_uids: &[i32],
    tz_name: &str,
) -> Result<Vec<WidgetPanel>, WidgetJsonError> {
    if visible_room_uids.is_empty() {
        return Ok(panels);
    }

    // Overnight gaps are those that cross local midnight. Rather than resolving
    // both gap ends to wall-clock per gap, track a single `overnight_boundary`:
    // the epoch of the first local midnight strictly after the running end. A gap
    // is overnight when the next panel starts at/after that boundary. The boundary
    // is recomputed (one conversion) only when the running end advances.
    let next_local_midnight = |epoch: i64| -> i64 {
        let local = crate::value::timezone::epoch_to_local(epoch, tz_name);
        let next_day = local
            .date()
            .succ_opt()
            .unwrap_or(local.date())
            .and_hms_opt(0, 0, 0)
            .unwrap_or(local);
        naive_to_epoch(next_day, tz_name)
    };

    let mut result = Vec::with_capacity(panels.len() + 8);
    let mut current_end: Option<i64> = None;
    let mut overnight_boundary: i64 = i64::MAX;
    let mut ib_counter: u32 = 0;
    let mut nb_counter: u32 = 0;

    for panel in panels {
        if let Some(start) = panel.start_epoch {
            if let Some(prev_end) = current_end {
                if start > prev_end {
                    let gap_minutes = ((start - prev_end) / 60) as i32;
                    let is_overnight = start >= overnight_boundary || gap_minutes > 240;

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

            // Advance the running end (and its overnight boundary) to the latest
            // end seen so far.
            if let Some(end) = panel.end_epoch {
                let new_end = current_end.map_or(end, |ce| ce.max(end));
                if current_end != Some(new_end) {
                    overnight_boundary = next_local_midnight(new_end);
                }
                current_end = Some(new_end);
            }
        }
        result.push(panel);
    }

    Ok(result)
}

fn make_break_panel(
    id: String,
    panel_type: &str,
    start_epoch: i64,
    end_epoch: i64,
    gap_minutes: i32,
    room_uids: &[i32],
) -> WidgetPanel {
    WidgetPanel {
        base_id: id.clone(),
        id,
        part_num: None,
        session_num: None,
        total_parts: None,
        is_series_lead: false,
        name: "Break".to_string(),
        panel_type: Some(panel_type.to_string()),
        room_ids: room_uids.to_vec(),
        start_epoch: Some(start_epoch),
        end_epoch: Some(end_epoch),
        duration: gap_minutes,
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
        credits: Vec::new(),
        presenters: Vec::new(),
    }
}

// ── Presenter export ──────────────────────────────────────────────────────────

fn export_presenters(
    schedule: &Schedule,
    panels: &[WidgetPanel],
    private_export: bool,
) -> Result<Vec<WidgetPresenter>, WidgetJsonError> {
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
        for p_id in inclusive_presenter_ids(schedule, panel_id, private_export) {
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

    // Canonical presenter display order: rank tier (Guest first, FanPanelist
    // last), then alphabetically by name.
    presenters_with_data.sort_by(|(_, a), (_, b)| a.data.cmp_for_display(&b.data));

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
            rank: p_data.data.rank.effective().as_str().to_string(),
            sort_key: sort_key as i32,
            is_group,
            members,
            groups,
            panel_ids: presenter_panel_ids[p_id].clone(),
            subsumes_members: p_data.data.subsumes_members,
        });
    }

    Ok(widget_presenters)
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Return the panel type prefix string for the given panel, if one is linked.
/// All presenter IDs reachable from a panel via credited+uncredited edges,
/// including transitive groups and transitive members.
///
/// Mirrors the logic of `FIELD_INCLUSIVE_PRESENTERS` in panel.rs.
///
/// When `include_uncredited` is true, also includes uncredited presenters.
fn inclusive_presenter_ids(
    schedule: &Schedule,
    panel_id: PanelId,
    include_uncredited: bool,
) -> HashSet<PresenterId> {
    let credited = schedule.connected_field_nodes(panel_id, panel::EDGE_CREDITED_PRESENTERS);
    let uncredited = if include_uncredited {
        schedule.connected_field_nodes(panel_id, panel::EDGE_UNCREDITED_PRESENTERS)
    } else {
        Vec::new()
    };
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
///
/// When `include_uncredited` is true, also includes uncredited (unlisted)
/// presenters; otherwise only credited presenters are returned.
fn individual_presenter_names(
    schedule: &Schedule,
    panel_id: PanelId,
    include_uncredited: bool,
) -> Vec<String> {
    let ids = inclusive_presenter_ids(schedule, panel_id, include_uncredited);
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

/// Compute the schedule-wide event window as Unix epoch-second bounds.
///
/// The metadata `start_time`/`end_time` (if set) seed the window; real
/// (non-break) scheduled panels then *extend* it earlier/later as needed. When
/// neither metadata nor any panel supplies a bound, falls back to `now`. Works
/// entirely in epoch seconds (FEATURE-154); the only conversion is seeding from
/// the metadata's naive wall-clock bounds.
fn compute_schedule_bounds(
    panels: &[WidgetPanel],
    metadata: &crate::schedule::ScheduleMetadata,
    now: &DateTime<Utc>,
    tz_name: &str,
) -> (i64, i64) {
    let mut start: Option<i64> = metadata.start_time.map(|dt| naive_to_epoch(dt, tz_name));
    let mut end: Option<i64> = metadata.end_time.map(|dt| naive_to_epoch(dt, tz_name));

    for panel in panels {
        if panel.id.starts_with('%') {
            continue;
        }
        if let Some(st) = panel.start_epoch {
            start = Some(start.map_or(st, |s| s.min(st)));
        }
        if let Some(et) = panel.end_epoch {
            end = Some(end.map_or(et, |e| e.max(et)));
        }
    }

    let fallback = now.timestamp();
    (start.unwrap_or(fallback), end.unwrap_or(fallback))
}

/// Early-morning local hour before which a session rolls into the previous day
/// when there is no clear overnight gap (the "hour" half of the gap-else-hour
/// rule). A session starting at/after this hour on a new calendar date opens a
/// new day.
const DAY_ROLLOVER_HOUR: u32 = 4;

/// Minimum gap (seconds) that counts as an overnight lull and therefore a day
/// boundary regardless of the wall-clock hour (the "gap" half of the rule).
/// Matches the overnight-break threshold used for `%NB` synthesis.
const DAY_ROLLOVER_GAP_SECS: i64 = 4 * 3600;

/// `(start, end_on_day, borrowed_end)` for a set of intervals within a half-open
/// window: `end_on_day` is the latest instant clamped to `clamp_hi` (the day or
/// half boundary), `borrowed_end` the true latest instant (past the boundary
/// when a session spans across it). `None` when nothing overlaps `[lo, hi)`.
fn window_span(
    intervals: &[(i64, i64)],
    lo: i64,
    hi: i64,
    clamp_hi: i64,
) -> Option<(i64, i64, i64)> {
    let mut start: Option<i64> = None;
    let mut end_on_day: Option<i64> = None;
    let mut borrowed: Option<i64> = None;
    for &(s, e) in intervals {
        if s < hi && e > lo {
            let cs = s.max(lo);
            start = Some(start.map_or(cs, |x| x.min(cs)));
            end_on_day = Some(end_on_day.map_or(e.min(clamp_hi), |x| x.max(e.min(clamp_hi))));
            borrowed = Some(borrowed.map_or(e, |x| x.max(e)));
        }
    }
    Some((start?, end_on_day?, borrowed?))
}

/// Compute the per-day (or AM/PM half-day) buckets over the real (non-break)
/// scheduled sessions, expressed in `tz_name` (FEATURE-154).
///
/// Sessions are grouped into "schedule days" by a gap-else-hour rollover: a new
/// day begins on a later calendar date only when preceded by an overnight gap
/// ([`DAY_ROLLOVER_GAP_SECS`]) or once the start reaches [`DAY_ROLLOVER_HOUR`];
/// otherwise contiguous post-midnight (late-night) sessions stay with the
/// previous day. Each bucket is labelled by its anchor (evening) calendar day
/// and carries the epoch range it covers; `end_epoch` is clamped to the
/// day/half boundary and `borrowed_end_epoch` extends past it when late-night
/// sessions are borrowed in. For half-day mode, a day with content in both
/// halves emits `"<Day> AM"`/`"<Day> PM"`; one half emits a single `"<Day>"`.
fn compute_day_spans(
    panels: &[WidgetPanel],
    is_break: &impl Fn(Option<&str>) -> bool,
    tz_name: &str,
    half_day: bool,
) -> Vec<WidgetDaySpan> {
    use chrono::Timelike;

    let mut intervals: Vec<(i64, i64)> = panels
        .iter()
        .filter(|p| !is_break(p.panel_type.as_deref()))
        .filter_map(|p| p.start_epoch.map(|s| (s, p.end_epoch.unwrap_or(s))))
        .collect();
    if intervals.is_empty() {
        return Vec::new();
    }
    intervals.sort_unstable();

    let local = |epoch: i64| crate::value::timezone::epoch_to_local(epoch, tz_name);
    let local_midnight = |date: chrono::NaiveDate| -> i64 {
        naive_to_epoch(date.and_hms_opt(0, 0, 0).expect("midnight valid"), tz_name)
    };

    // 1. Split into overnight-gap-separated runs. (Afternoon gaps split runs too,
    //    but the runs re-merge by calendar date below, so they add no extra days.)
    let mut runs: Vec<Vec<(i64, i64)>> = Vec::new();
    let mut run_end = i64::MIN;
    for &(s, e) in &intervals {
        if runs.is_empty() || s - run_end >= DAY_ROLLOVER_GAP_SECS {
            runs.push(vec![(s, e)]);
        } else {
            runs.last_mut().expect("non-empty").push((s, e));
        }
        run_end = run_end.max(e);
    }

    // 2. Assign each session to a schedule-day date. A late-night *tail* — a
    //    post-midnight session whose run has no session at/after the rollover
    //    hour on that date (so the run ends in the early hours, followed by a
    //    gap) — is borrowed into the previous calendar day. Continuous content
    //    that runs past the rollover into morning is not borrowed: it keeps its
    //    own date (splitting at midnight).
    use std::collections::{BTreeMap, HashSet};
    let mut by_day: BTreeMap<chrono::NaiveDate, Vec<(i64, i64)>> = BTreeMap::new();
    for run in &runs {
        let first_date = local(run[0].0).date();
        let morning_dates: HashSet<chrono::NaiveDate> = run
            .iter()
            .filter(|&&(s, _)| local(s).hour() >= DAY_ROLLOVER_HOUR)
            .map(|&(s, _)| local(s).date())
            .collect();
        for &(s, e) in run {
            let d = local(s).date();
            let sd = if d == first_date || morning_dates.contains(&d) {
                d
            } else {
                d.pred_opt().unwrap_or(d)
            };
            by_day.entry(sd).or_default().push((s, e));
        }
    }

    let min_d = *by_day.keys().next().expect("non-empty");
    let max_d = *by_day.keys().next_back().expect("non-empty");

    let emit = |out: &mut Vec<WidgetDaySpan>, label: String, span: (i64, i64, i64)| {
        let (start, end_on_day, borrowed) = span;
        out.push(WidgetDaySpan {
            label,
            start_epoch: start,
            end_epoch: end_on_day,
            borrowed_end_epoch: (borrowed > end_on_day).then_some(borrowed),
        });
    };

    let mut out = Vec::new();
    for (&day, group) in &by_day {
        let anchor_mid = local_midnight(day);
        let next_mid = local_midnight(day.succ_opt().unwrap_or(day));
        let label = crate::value::timezone::day_label(day, min_d, max_d);
        // Upper bound for "overlap" is the group's true end (so borrowed
        // late-night sessions are included); clamps use the calendar boundary.
        let hi = group.iter().map(|&(_, e)| e).max().expect("non-empty") + 1;
        if half_day {
            let noon = naive_to_epoch(
                day.and_hms_opt(12, 0, 0).expect("noon valid"),
                tz_name,
            );
            // AM never borrows: a session crossing *noon* spans both halves
            // (clamped at noon), it is not late-night content. Only the PM/day
            // end side borrows across midnight.
            let am = window_span(group, anchor_mid, noon, noon).map(|(s, e, _)| (s, e, e));
            let pm = window_span(group, noon, hi, next_mid);
            match (am, pm) {
                (Some(a), Some(p)) => {
                    emit(&mut out, format!("{label} AM"), a);
                    emit(&mut out, format!("{label} PM"), p);
                }
                (Some(span), None) | (None, Some(span)) => emit(&mut out, label, span),
                (None, None) => {}
            }
        } else if let Some(span) = window_span(group, anchor_mid, hi, next_mid) {
            emit(&mut out, label, span);
        }
    }
    out
}

/// Convert a naive wall-clock datetime, interpreted in `tz`, to Unix epoch
/// seconds. With no timezone the value is treated as UTC. Ambiguous local times
/// (DST fall-back) resolve to the earliest instant; nonexistent local times (DST
/// spring-forward gap) fall back to a naive-UTC interpretation.
fn naive_to_epoch(dt: NaiveDateTime, tz_name: &str) -> i64 {
    use chrono::offset::LocalResult;
    use chrono::TimeZone;
    match crate::value::timezone::parse_tz(tz_name) {
        Some(tz) => match tz.from_local_datetime(&dt) {
            LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => t.timestamp(),
            LocalResult::None => dt.and_utc().timestamp(),
        },
        None => dt.and_utc().timestamp(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Schedule;
    use crate::tables::event_room::EventRoomInternalData;
    use crate::tables::fields::code::CodeHistory;
    use crate::tables::panel::PanelInternalData;
    use crate::tables::panel_type::PanelTypeInternalData;
    use crate::tables::presenter::PresenterInternalData;
    use crate::tables::timeline::{TimelineId, TimelineInternalData};
    use crate::tables::TimelineCommonData;
    use crate::value::time::TimeRange;
    use crate::value::uniq_id::PanelUniqId;
    use crate::widget_json::import::load_from_json;
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
                code: CodeHistory::new(code),
                data: crate::tables::panel::PanelCommonData {
                    name: format!("Panel {code_str}"),
                    ..Default::default()
                },
                notes: crate::tables::fields::note::NoteBag::default(),
                time_slot,
            },
        );
        id
    }

    fn make_timeline(
        sched: &mut Schedule,
        code_str: &str,
        start_hms: Option<(i32, u32, u32, u32)>,
    ) -> TimelineId {
        let code = PanelUniqId::parse(code_str).unwrap();
        let id = crate::entity::EntityId::generate();
        let time = start_hms.map(|(day_offset, h, m, s)| {
            let base = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
            let date = base + chrono::Duration::days(day_offset as i64);
            date.and_hms_opt(h, m, s).unwrap()
        });
        sched.insert(
            id,
            TimelineInternalData {
                id,
                code: CodeHistory::new(code),
                data: TimelineCommonData {
                    name: format!("Timeline {code_str}"),
                    time,
                    ..Default::default()
                },
                notes: crate::tables::fields::note::NoteBag::default(),
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

    // Panel type is derived from the Uniq ID prefix (FEATURE-146); these test
    // fixtures use codes whose prefix matches the intended panel type, so no
    // explicit edge link is needed. Kept as no-ops to preserve test structure.
    fn link_panel_type(
        _sched: &mut Schedule,
        _panel_id: PanelId,
        _pt_id: crate::tables::panel_type::PanelTypeId,
    ) {
    }

    fn link_timeline_panel_type(
        _sched: &mut Schedule,
        _timeline_id: TimelineId,
        _pt_id: crate::tables::panel_type::PanelTypeId,
    ) {
    }

    fn link_panel_room(sched: &mut Schedule, panel_id: PanelId, room_id: EventRoomId) {
        let _ = sched.edge_add(panel_id, panel::EDGE_EVENT_ROOMS, [room_id]);
    }

    fn link_credited_presenter(sched: &mut Schedule, panel_id: PanelId, presenter_id: PresenterId) {
        let _ = sched.edge_add(panel_id, panel::EDGE_CREDITED_PRESENTERS, [presenter_id]);
    }

    fn link_uncredited_presenter(
        sched: &mut Schedule,
        panel_id: PanelId,
        presenter_id: PresenterId,
    ) {
        let _ = sched.edge_add(panel_id, panel::EDGE_UNCREDITED_PRESENTERS, [presenter_id]);
    }

    // ── tests ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_uncredited_presenter_only_in_private_export() {
        // A panel with one credited and one uncredited (unlisted) presenter.
        let mut sched = Schedule::new();
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let panel_id = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        let listed = make_presenter(&mut sched, "Listed Guest");
        let unlisted = make_presenter(&mut sched, "Unlisted Guest");
        link_panel_type(&mut sched, panel_id, pt_id);
        link_credited_presenter(&mut sched, panel_id, listed);
        link_uncredited_presenter(&mut sched, panel_id, unlisted);

        let (_, uid_map) = build_room_uid_map(&sched);
        let panel_types = export_panel_types(&sched).unwrap();

        // Public export: only the credited presenter is on the panel.
        let public = export_panels(&sched, &uid_map, &[], &panel_types, false, "").unwrap();
        let pub_panel = public.iter().find(|p| p.id == "GP001").unwrap();
        assert!(pub_panel.presenters.contains(&"Listed Guest".to_string()));
        assert!(!pub_panel.presenters.contains(&"Unlisted Guest".to_string()));

        // Private export: the unlisted presenter is surfaced on the panel's
        // `presenters` (split/search) field so print layout can attribute the
        // panel to them in per-presenter sections...
        let private = export_panels(&sched, &uid_map, &[], &panel_types, true, "").unwrap();
        let priv_panel = private.iter().find(|p| p.id == "GP001").unwrap();
        assert!(priv_panel.presenters.contains(&"Listed Guest".to_string()));
        assert!(priv_panel
            .presenters
            .contains(&"Unlisted Guest".to_string()));

        // ...but the visible `credits` byline stays credited-only, even in the
        // private export.
        assert!(priv_panel.credits.contains(&"Listed Guest".to_string()));
        assert!(!priv_panel.credits.contains(&"Unlisted Guest".to_string()));
    }

    #[test]
    fn test_export_creates_valid_structure() {
        let schedule = Schedule::new();
        let result = export_to_widget_json(&schedule, "Test Schedule", false);
        assert!(result.is_ok());
        let export = result.unwrap();
        assert_eq!(export.meta.version, 2);
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
        assert!(gp.colors.color.is_some());
        assert_eq!(gp.prefix, "GP");
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
        let panels = export_panels(&sched, &uid_map, &[], &panel_types, false, "").unwrap();

        let real: Vec<_> = panels.iter().filter(|p| !p.id.starts_with('%')).collect();
        assert_eq!(real.len(), 1);
        let p = &real[0];
        assert_eq!(p.id, "GP001");
        assert_eq!(p.base_id, "GP001");
        assert_eq!(p.panel_type.as_deref(), Some("GP"));
        assert_eq!(p.room_ids, vec![1]);
        assert_eq!(p.duration, 60);
        // tz_name "" → wall-clock interpreted as UTC.
        let expect = |s: &str| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .unwrap()
                .and_utc()
                .timestamp()
        };
        assert_eq!(p.start_epoch, Some(expect("2026-06-01T14:00:00")));
        assert_eq!(p.end_epoch, Some(expect("2026-06-01T15:00:00")));
    }

    #[test]
    fn test_epoch_uses_metadata_timezone() {
        use chrono::TimeZone;
        let mut sched = Schedule::new();
        sched.metadata.timezone = Some("America/New_York".to_string());
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        // 2026-06-01 is EDT (UTC-4).
        let panel_id = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        link_panel_type(&mut sched, panel_id, pt_id);

        let export = export_to_widget_json(&sched, "Test", false).unwrap();
        let p = export.panels.iter().find(|p| p.id == "GP001").unwrap();
        let want = chrono_tz::Tz::America__New_York
            .with_ymd_and_hms(2026, 6, 1, 14, 0, 0)
            .unwrap()
            .timestamp();
        assert_eq!(p.start_epoch, Some(want));
        assert_eq!(export.meta.start_epoch, want);
        assert_eq!(export.meta.version, 2);
    }

    fn ep(s: &str) -> i64 {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .and_utc()
            .timestamp()
    }

    #[test]
    fn test_day_and_half_day_timelines() {
        // 2026-06-01 is a Monday; offsets 0/1 stay in one ISO week → bare labels.
        let mut sched = Schedule::new();
        let pt = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let p_mon = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60)); // Mon 14–15 (PM)
        let p_tue_am = make_panel(&mut sched, "GP002", Some((1, 9, 0, 0)), Some(60)); // Tue 09–10
        let p_tue_pm = make_panel(&mut sched, "GP003", Some((1, 16, 0, 0)), Some(60)); // Tue 16–17
        for p in [p_mon, p_tue_am, p_tue_pm] {
            link_panel_type(&mut sched, p, pt);
        }

        let export = export_to_widget_json(&sched, "Test", false).unwrap();

        let days: Vec<(&str, i64, i64)> = export
            .day_timeline
            .iter()
            .map(|d| (d.label.as_str(), d.start_epoch, d.end_epoch))
            .collect();
        assert_eq!(
            days,
            vec![
                ("Monday", ep("2026-06-01T14:00:00"), ep("2026-06-01T15:00:00")),
                ("Tuesday", ep("2026-06-02T09:00:00"), ep("2026-06-02T17:00:00")),
            ]
        );

        let halves: Vec<(&str, i64, i64)> = export
            .half_day_timeline
            .iter()
            .map(|d| (d.label.as_str(), d.start_epoch, d.end_epoch))
            .collect();
        assert_eq!(
            halves,
            vec![
                // Monday has afternoon content only → single bare "Monday".
                ("Monday", ep("2026-06-01T14:00:00"), ep("2026-06-01T15:00:00")),
                ("Tuesday AM", ep("2026-06-02T09:00:00"), ep("2026-06-02T10:00:00")),
                ("Tuesday PM", ep("2026-06-02T16:00:00"), ep("2026-06-02T17:00:00")),
            ]
        );
    }

    #[test]
    fn test_day_timeline_clamps_midnight_crossing() {
        // A single session running past midnight stays with its anchor day; its
        // real end clamps to midnight and borrowedEnd carries the true end.
        let mut sched = Schedule::new();
        let pt = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        // Mon 23:00 + 120m → Tue 01:00.
        let p = make_panel(&mut sched, "GP001", Some((0, 23, 0, 0)), Some(120));
        link_panel_type(&mut sched, p, pt);

        let export = export_to_widget_json(&sched, "Test", false).unwrap();
        assert_eq!(export.day_timeline.len(), 1);
        let d = &export.day_timeline[0];
        assert_eq!(d.label, "Monday");
        assert_eq!(d.start_epoch, ep("2026-06-01T23:00:00"));
        assert_eq!(d.end_epoch, ep("2026-06-02T00:00:00"));
        assert_eq!(d.borrowed_end_epoch, Some(ep("2026-06-02T01:00:00")));
    }

    #[test]
    fn test_day_timeline_borrows_late_night() {
        // Late-night sessions just past midnight (small gap, before the rollover
        // hour) belong to the previous day's bucket; a morning session opens the
        // next day.
        let mut sched = Schedule::new();
        let pt = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let p1 = make_panel(&mut sched, "GP001", Some((0, 20, 0, 0)), Some(180)); // Mon 20:00–23:00
        let p2 = make_panel(&mut sched, "GP002", Some((1, 0, 30, 0)), Some(60)); // Tue 00:30–01:30
        let p3 = make_panel(&mut sched, "GP003", Some((1, 10, 0, 0)), Some(60)); // Tue 10:00–11:00
        for p in [p1, p2, p3] {
            link_panel_type(&mut sched, p, pt);
        }

        let export = export_to_widget_json(&sched, "Test", false).unwrap();
        let days: Vec<(&str, i64, i64, Option<i64>)> = export
            .day_timeline
            .iter()
            .map(|d| (d.label.as_str(), d.start_epoch, d.end_epoch, d.borrowed_end_epoch))
            .collect();
        assert_eq!(
            days,
            vec![
                (
                    "Monday",
                    ep("2026-06-01T20:00:00"),
                    ep("2026-06-02T00:00:00"),
                    Some(ep("2026-06-02T01:30:00")),
                ),
                (
                    "Tuesday",
                    ep("2026-06-02T10:00:00"),
                    ep("2026-06-02T11:00:00"),
                    None,
                ),
            ]
        );
    }

    #[test]
    fn test_day_timeline_no_borrow_when_continuous() {
        // Continuous programming running past the rollover hour into the morning
        // is NOT borrowed — it splits at midnight (no extra borrow hours).
        let mut sched = Schedule::new();
        let pt = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let p1 = make_panel(&mut sched, "GP001", Some((0, 22, 0, 0)), Some(60)); // Mon 22–23
        let p2 = make_panel(&mut sched, "GP002", Some((1, 0, 0, 0)), Some(60)); // Tue 00–01
        let p3 = make_panel(&mut sched, "GP003", Some((1, 2, 0, 0)), Some(60)); // Tue 02–03
        let p4 = make_panel(&mut sched, "GP004", Some((1, 5, 0, 0)), Some(60)); // Tue 05–06 (>= rollover)
        for p in [p1, p2, p3, p4] {
            link_panel_type(&mut sched, p, pt);
        }

        let export = export_to_widget_json(&sched, "Test", false).unwrap();
        let days: Vec<(&str, i64, i64, Option<i64>)> = export
            .day_timeline
            .iter()
            .map(|d| (d.label.as_str(), d.start_epoch, d.end_epoch, d.borrowed_end_epoch))
            .collect();
        assert_eq!(
            days,
            vec![
                (
                    "Monday",
                    ep("2026-06-01T22:00:00"),
                    ep("2026-06-01T23:00:00"),
                    None,
                ),
                (
                    "Tuesday",
                    ep("2026-06-02T00:00:00"),
                    ep("2026-06-02T06:00:00"),
                    None,
                ),
            ]
        );
    }

    #[test]
    fn test_epoch_without_timezone_is_utc() {
        // No metadata timezone → naive wall-clock is interpreted as UTC.
        let mut sched = Schedule::new();
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let panel_id = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        link_panel_type(&mut sched, panel_id, pt_id);

        let export = export_to_widget_json(&sched, "Test", false).unwrap();
        let p = export.panels.iter().find(|p| p.id == "GP001").unwrap();
        let want = NaiveDateTime::parse_from_str("2026-06-01T14:00:00", "%Y-%m-%dT%H:%M:%S")
            .unwrap()
            .and_utc()
            .timestamp();
        assert_eq!(p.start_epoch, Some(want));
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
        let panels = export_panels(&sched, &uid_map, &[], &panel_types, false, "").unwrap();

        let real: Vec<_> = panels.iter().filter(|p| !p.id.starts_with('%')).collect();
        assert_eq!(real.len(), 2);
        // Scheduled panel must come before unscheduled
        assert!(real[0].start_epoch.is_some());
        assert!(real[1].start_epoch.is_none());
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

        let (rooms, uid_map) = build_room_uid_map(&sched);
        let visible_room_uids: Vec<i32> = rooms.iter().map(|r| r.uid).collect();
        let panel_types = export_panel_types(&sched).unwrap();
        let panels =
            export_panels(&sched, &uid_map, &visible_room_uids, &panel_types, false, "").unwrap();

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
        let room_id = make_event_room(&mut sched, "R1", None, 1);
        // Panel 1 ends 23:00 day 0, Panel 2 starts 09:00 day 1
        let p1 = make_panel(&mut sched, "GP001", Some((0, 21, 0, 0)), Some(120)); // 21:00–23:00
        let p2 = make_panel(&mut sched, "GP002", Some((1, 9, 0, 0)), Some(60)); // 09:00–10:00 next day
        link_panel_type(&mut sched, p1, pt_id);
        link_panel_type(&mut sched, p2, pt_id);
        link_panel_room(&mut sched, p1, room_id);
        link_panel_room(&mut sched, p2, room_id);

        let (rooms, uid_map) = build_room_uid_map(&sched);
        let visible_room_uids: Vec<i32> = rooms.iter().map(|r| r.uid).collect();
        let panel_types = export_panel_types(&sched).unwrap();
        let panels =
            export_panels(&sched, &uid_map, &visible_room_uids, &panel_types, false, "").unwrap();

        let ids: Vec<&str> = panels.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"%NB001"), "expected %NB001 in {ids:?}");
    }

    #[test]
    fn test_export_timeline_only() {
        let mut sched = Schedule::new();
        let tl_pt = make_panel_type(&mut sched, "SP", "Split", true);
        let gp_pt = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let tl_timeline = make_timeline(&mut sched, "SP001", Some((0, 8, 0, 0)));
        let gp_panel = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        link_timeline_panel_type(&mut sched, tl_timeline, tl_pt);
        link_panel_type(&mut sched, gp_panel, gp_pt);

        let panel_types = export_panel_types(&sched).unwrap();
        let (_, uid_map) = build_room_uid_map(&sched);
        let panels = export_panels(&sched, &uid_map, &[], &panel_types, false, "").unwrap();
        let timeline = export_timeline(&sched, &panel_types, false, "").unwrap();

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
        let panels = export_panels(&sched, &uid_map, &[], &panel_types, false, "").unwrap();
        let presenters = export_presenters(&sched, &panels, false).unwrap();

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
        let panels = export_panels(&sched, &uid_map, &[], &panel_types, false, "").unwrap();
        let presenters = export_presenters(&sched, &panels, false).unwrap();

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

        let export = export_to_widget_json(&sched, "Test", false).unwrap();
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

        let export = export_to_widget_json(&sched, "Test", false).unwrap();
        // No metadata timezone → epoch reflects the wall-clock as UTC.
        let start_iso = crate::value::timezone::epoch_to_local_iso(export.meta.start_epoch, "");
        let end_iso = crate::value::timezone::epoch_to_local_iso(export.meta.end_epoch, "");
        assert!(
            start_iso.contains("09:00:00"),
            "start should contain 09:00:00, got {start_iso}"
        );
        assert!(
            end_iso.contains("15:30:00"),
            "end should contain 15:30:00, got {end_iso}"
        );
    }

    #[test]
    fn test_load_save_roundtrip() {
        let mut sched = Schedule::new();
        let pt_id = make_panel_type(&mut sched, "GP", "Guest Panel", false);
        let room_id = make_event_room(&mut sched, "R1", None, 1);
        let panel_id = make_panel(&mut sched, "GP001", Some((0, 14, 0, 0)), Some(60));
        link_panel_type(&mut sched, panel_id, pt_id);
        link_panel_room(&mut sched, panel_id, room_id);

        let export = export_to_widget_json(&sched, "Test", false).unwrap();
        let json = save_to_json(&export).unwrap();
        let loaded = load_from_json(&json).unwrap();

        assert_eq!(export.meta.title, loaded.meta.title);
        assert_eq!(export.panels.len(), loaded.panels.len());
        assert_eq!(export.rooms.len(), loaded.rooms.len());
    }

    fn part_panel(id: &str, base: &str, part: i32, start_epoch: i64) -> WidgetPanel {
        WidgetPanel {
            id: id.to_string(),
            base_id: base.to_string(),
            part_num: Some(part),
            start_epoch: Some(start_epoch),
            ..Default::default()
        }
    }

    #[test]
    fn test_annotate_multipart_marks_lead_and_total() {
        // GP001 has two parts; part 1 also has a rerun. The lead is the lowest
        // part number, earliest start — here the 2pm Part 1.
        let mut panels = vec![
            part_panel("GP001P2", "GP001", 2, 1600),
            part_panel("GP001P1S2", "GP001", 1, 1800),
            part_panel("GP001P1", "GP001", 1, 1400),
        ];
        annotate_multipart_series(&mut panels);

        for p in &panels {
            assert_eq!(p.total_parts, Some(2), "{} total_parts", p.id);
        }
        let lead: Vec<&str> = panels
            .iter()
            .filter(|p| p.is_series_lead)
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(lead, vec!["GP001P1"], "exactly the earliest Part 1 leads");
    }

    #[test]
    fn test_annotate_single_part_reruns_not_multipart() {
        // A single part repeated (reruns) is not a multi-part series — cost is
        // per session, so nothing is annotated.
        let mut panels = vec![
            part_panel("GP002P1", "GP002", 1, 1400),
            part_panel("GP002P1S2", "GP002", 1, 1600),
        ];
        annotate_multipart_series(&mut panels);
        assert!(panels.iter().all(|p| p.total_parts.is_none()));
        assert!(panels.iter().all(|p| !p.is_series_lead));
    }
}
