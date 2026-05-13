/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

// TODO(EDITOR-111): extract to crates/cosam-editor-shared once framework is chosen

use std::collections::BTreeSet;

use chrono::{NaiveDate, NaiveDateTime};
use schedule_core::schedule::Schedule;
use schedule_core::tables::panel;
use schedule_core::tables::{EventRoomEntityType, EventRoomId, PanelEntityType, PanelId};
use schedule_core::ChangeState;
use schedule_core::EntityUuid;

#[derive(Debug, Clone)]
pub struct PanelDisplayInfo {
    pub panel_id: PanelId,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub time_range_str: String,
    pub room_ids: Vec<EventRoomId>,
    pub room_names: Vec<String>,
    pub change_state: ChangeState,
}

#[derive(Debug, Clone)]
pub struct RoomDisplayInfo {
    pub room_id: EventRoomId,
    pub display_name: String,
    pub sort_key: Option<i64>,
}

pub fn all_days(schedule: &Schedule) -> Vec<NaiveDate> {
    let mut dates: BTreeSet<NaiveDate> = BTreeSet::new();
    for (_, internal) in schedule.iter_entities::<PanelEntityType>() {
        if internal.time_slot.is_scheduled() {
            if let Some(start) = internal.time_slot.start_time() {
                dates.insert(start.date());
            }
        }
    }
    dates.into_iter().collect()
}

pub fn all_rooms(schedule: &Schedule) -> Vec<RoomDisplayInfo> {
    let mut rooms: Vec<RoomDisplayInfo> = schedule
        .iter_entities::<EventRoomEntityType>()
        .filter(|(_, r)| !r.data.is_pseudo)
        .map(|(id, r)| RoomDisplayInfo {
            room_id: id,
            display_name: r
                .data
                .long_name
                .as_deref()
                .unwrap_or(&r.data.room_name)
                .to_string(),
            sort_key: r.data.sort_key,
        })
        .collect();
    rooms.sort_by(|a, b| {
        a.sort_key
            .cmp(&b.sort_key)
            .then_with(|| a.display_name.cmp(&b.display_name))
    });
    rooms
}

pub fn panels_for(
    schedule: &Schedule,
    day: NaiveDate,
    room_filter: Option<EventRoomId>,
) -> Vec<PanelDisplayInfo> {
    let mut panels: Vec<PanelDisplayInfo> = schedule
        .iter_entities::<PanelEntityType>()
        .filter(|(_, p)| {
            p.time_slot.is_scheduled()
                && p.time_slot
                    .start_time()
                    .map(|dt| dt.date() == day)
                    .unwrap_or(false)
        })
        .filter(|(id, _)| {
            if let Some(filter_id) = room_filter {
                schedule
                    .connected_entities::<EventRoomEntityType>(*id, panel::EDGE_EVENT_ROOMS)
                    .contains(&filter_id)
            } else {
                true
            }
        })
        .filter_map(|(id, p)| {
            let start = p.time_slot.start_time()?;
            let end = p.time_slot.end_time().unwrap_or(start);
            let time_range_str = format!(
                "{} – {}",
                start.format("%l:%M %p").to_string().trim(),
                end.format("%l:%M %p").to_string().trim(),
            );
            let room_ids =
                schedule.connected_entities::<EventRoomEntityType>(id, panel::EDGE_EVENT_ROOMS);
            let room_names: Vec<String> = room_ids
                .iter()
                .filter_map(|rid| {
                    schedule.get_internal::<EventRoomEntityType>(*rid).map(|r| {
                        r.data
                            .long_name
                            .as_deref()
                            .unwrap_or(&r.data.room_name)
                            .to_string()
                    })
                })
                .collect();
            let change_state = schedule.entity_change_state(id.entity_uuid());
            Some(PanelDisplayInfo {
                panel_id: id,
                code: p.code.full_id(),
                name: p.data.name.clone(),
                description: p.data.description.clone(),
                start_time: start,
                end_time: end,
                time_range_str,
                room_ids,
                room_names,
                change_state,
            })
        })
        .collect();
    panels.sort_by_key(|p| p.start_time);
    panels
}
