/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Scheduling conflict detection for cosam-convert.
//!
//! Detects room double-booking and presenter availability conflicts by
//! iterating over all panels and checking for time-range overlaps.

use chrono::NaiveDateTime;
use schedule_core::schedule::Schedule;
use schedule_core::tables::event_room::EventRoomEntityType;
use schedule_core::tables::panel::{self, PanelEntityType};
use schedule_core::tables::presenter::PresenterEntityType;

// ── Conflict types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    Room,
    Presenter,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub panel1_name: String,
    pub panel2_name: String,
    pub kind: ConflictKind,
    pub context: String, // room name or presenter name
}

// ── Panel snapshot ────────────────────────────────────────────────────────────

struct PanelSnapshot {
    name: String,
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
    room_ids: Vec<schedule_core::tables::event_room::EventRoomId>,
    presenter_ids: Vec<schedule_core::tables::presenter::PresenterId>,
}

// ── Detection ─────────────────────────────────────────────────────────────────

fn overlaps(
    a_start: NaiveDateTime,
    a_end: NaiveDateTime,
    b_start: NaiveDateTime,
    b_end: NaiveDateTime,
) -> bool {
    a_start < b_end && b_start < a_end
}

pub fn detect_conflicts(schedule: &Schedule) -> Vec<Conflict> {
    // Build snapshots for all scheduled panels (those with both start and end).
    let snapshots: Vec<PanelSnapshot> =
        schedule
            .iter_entities::<PanelEntityType>()
            .filter_map(|(id, internal)| {
                let start = internal.time_slot.start_time()?;
                let end = internal.time_slot.end_time()?;
                let name = internal.data.name.clone();

                let room_ids =
                    schedule.connected_entities::<EventRoomEntityType>(id, panel::EDGE_EVENT_ROOMS);

                let mut presenter_ids = schedule
                    .connected_entities::<PresenterEntityType>(id, panel::EDGE_CREDITED_PRESENTERS);
                presenter_ids.extend(schedule.connected_entities::<PresenterEntityType>(
                    id,
                    panel::EDGE_UNCREDITED_PRESENTERS,
                ));

                Some(PanelSnapshot {
                    name,
                    start: Some(start),
                    end: Some(end),
                    room_ids,
                    presenter_ids,
                })
            })
            .collect();

    let mut conflicts = Vec::new();

    // Check every pair once.
    for i in 0..snapshots.len() {
        for j in (i + 1)..snapshots.len() {
            let a = &snapshots[i];
            let b = &snapshots[j];

            let (Some(a_start), Some(a_end)) = (a.start, a.end) else {
                continue;
            };
            let (Some(b_start), Some(b_end)) = (b.start, b.end) else {
                continue;
            };

            if !overlaps(a_start, a_end, b_start, b_end) {
                continue;
            }

            // Room conflicts
            for room_id in &a.room_ids {
                if b.room_ids.contains(room_id) {
                    let room_name = schedule
                        .get_internal::<EventRoomEntityType>(*room_id)
                        .map(|r| r.data.room_name.as_str())
                        .unwrap_or("unknown room")
                        .to_string();
                    conflicts.push(Conflict {
                        panel1_name: a.name.clone(),
                        panel2_name: b.name.clone(),
                        kind: ConflictKind::Room,
                        context: room_name,
                    });
                }
            }

            // Presenter conflicts
            for presenter_id in &a.presenter_ids {
                if b.presenter_ids.contains(presenter_id) {
                    let presenter_name = schedule
                        .get_internal::<PresenterEntityType>(*presenter_id)
                        .map(|p| p.data.name.as_str())
                        .unwrap_or("unknown presenter")
                        .to_string();
                    conflicts.push(Conflict {
                        panel1_name: a.name.clone(),
                        panel2_name: b.name.clone(),
                        kind: ConflictKind::Presenter,
                        context: presenter_name,
                    });
                }
            }
        }
    }

    conflicts
}

// ── Reporting ─────────────────────────────────────────────────────────────────

pub fn print_conflicts(conflicts: &[Conflict]) {
    if conflicts.is_empty() {
        eprintln!("No conflicts detected");
        return;
    }

    eprintln!("Conflicts found: {}", conflicts.len());

    let room_count = conflicts
        .iter()
        .filter(|c| c.kind == ConflictKind::Room)
        .count();
    let presenter_count = conflicts
        .iter()
        .filter(|c| c.kind == ConflictKind::Presenter)
        .count();

    if room_count > 0 {
        eprintln!("  Room conflicts: {room_count}");
    }
    if presenter_count > 0 {
        eprintln!("  Presenter conflicts: {presenter_count}");
    }

    const MAX_EXAMPLES: usize = 5;
    for (i, conflict) in conflicts.iter().take(MAX_EXAMPLES).enumerate() {
        let kind_label = match conflict.kind {
            ConflictKind::Room => "room",
            ConflictKind::Presenter => "presenter",
        };
        eprintln!(
            "  {}. \"{}\" vs \"{}\" ({}: {})",
            i + 1,
            conflict.panel1_name,
            conflict.panel2_name,
            kind_label,
            conflict.context,
        );
    }

    if conflicts.len() > MAX_EXAMPLES {
        eprintln!(
            "  ... and {} more conflicts",
            conflicts.len() - MAX_EXAMPLES
        );
    }
}
