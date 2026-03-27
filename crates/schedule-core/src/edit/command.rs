/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::data::panel::ExtraFields;
use crate::data::panel_type::PanelType;
use crate::data::presenter::{
    Presenter, PresenterGroup, PresenterMember, PresenterRank, PresenterSortRank,
};
use crate::data::room::Room;
use crate::data::schedule::Schedule;
use crate::data::source_info::{ChangeState, SourceInfo};
use crate::data::time::TimeRange;

/// Identifies which `Option<String>` field on a flat [`crate::data::Panel`] to set.
///
/// In the flat model a `Panel` is fully self-contained so this enum covers
/// both what used to live on the base panel **and** what used to live on a
/// `PanelSession`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelField {
    // ── Base panel fields ────────────────────────────────────────
    Description,
    Note,
    Prereq,
    Cost,
    Capacity,
    Difficulty,
    PanelType,
    AltPanelist,
    PreRegMax,
    TicketUrl,
    SimpleTicketEvent,
    HaveTicketImage,
    // ── Scheduling / session fields (now part of flat Panel) ─────
    StartTime,
    EndTime,
    AvNotes,
    NotesNonPrinting,
    WorkshopNotes,
    PowerNeeds,
}

/// `SessionField` is kept as a type alias for backward source-level
/// compatibility. New code should use [`PanelField`] directly.
pub type SessionField = PanelField;

/// Snapshot of scheduling-related session state for undo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionScheduleState {
    pub room_ids: Vec<u32>,
    pub timing: crate::data::time::TimeRange,
}

/// Snapshot of a room's mutable fields for undo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomSnapshot {
    pub short_name: String,
    pub long_name: String,
    pub hotel_room: String,
    pub sort_key: u32,
    pub is_break: bool,
    pub metadata: Option<ExtraFields>,
}

impl RoomSnapshot {
    pub fn from_room(room: &Room) -> Self {
        Self {
            short_name: room.short_name.clone(),
            long_name: room.long_name.clone(),
            hotel_room: room.hotel_room.clone(),
            sort_key: room.sort_key,
            is_break: room.is_break,
            metadata: room.metadata.clone(),
        }
    }

    pub fn apply_to(&self, room: &mut Room) {
        room.short_name = self.short_name.clone();
        room.long_name = self.long_name.clone();
        room.hotel_room = self.hotel_room.clone();
        room.sort_key = self.sort_key;
        room.is_break = self.is_break;
        room.metadata = self.metadata.clone();
    }
}

/// Snapshot of a presenter's mutable fields for undo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresenterSnapshot {
    pub rank: PresenterRank,
    pub is_member: PresenterMember,
    pub is_grouped: PresenterGroup,
    pub sort_rank: Option<PresenterSortRank>,
    pub metadata: Option<ExtraFields>,
}

impl PresenterSnapshot {
    pub fn from_presenter(p: &Presenter) -> Self {
        Self {
            rank: p.rank.clone(),
            is_member: p.is_member.clone(),
            is_grouped: p.is_grouped.clone(),
            sort_rank: p.sort_rank.clone(),
            metadata: p.metadata.clone(),
        }
    }

    pub fn apply_to(&self, p: &mut Presenter) {
        p.rank = self.rank.clone();
        p.is_member = self.is_member.clone();
        p.is_grouped = self.is_grouped.clone();
        p.sort_rank = self.sort_rank.clone();
        p.metadata = self.metadata.clone();
    }
}

/// Snapshot of a panel type's mutable fields for undo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelTypeSnapshot {
    pub kind: String,
    pub colors: IndexMap<String, String>,
    pub is_break: bool,
    pub is_cafe: bool,
    pub is_workshop: bool,
    pub is_hidden: bool,
    pub is_room_hours: bool,
    pub is_timeline: bool,
    pub is_private: bool,
    pub metadata: Option<ExtraFields>,
}

impl PanelTypeSnapshot {
    pub fn from_panel_type(pt: &PanelType) -> Self {
        Self {
            kind: pt.kind.clone(),
            colors: pt.colors.clone(),
            is_break: pt.is_break,
            is_cafe: pt.is_cafe,
            is_workshop: pt.is_workshop,
            is_hidden: pt.is_hidden,
            is_room_hours: pt.is_room_hours,
            is_timeline: pt.is_timeline,
            is_private: pt.is_private,
            metadata: pt.metadata.clone(),
        }
    }

    pub fn apply_to(&self, pt: &mut PanelType) {
        pt.kind = self.kind.clone();
        pt.colors = self.colors.clone();
        pt.is_break = self.is_break;
        pt.is_cafe = self.is_cafe;
        pt.is_workshop = self.is_workshop;
        pt.is_hidden = self.is_hidden;
        pt.is_room_hours = self.is_room_hours;
        pt.is_timeline = self.is_timeline;
        pt.is_private = self.is_private;
        pt.metadata = self.metadata.clone();
    }
}

/// A single atomic edit command that can be applied and undone.
///
/// Each variant stores the data needed for both forward application and
/// reversal. Old-state fields are populated at apply-time.
///
/// In the flat model every [`crate::data::Panel`] is fully self-contained and
/// is addressed by its full Uniq ID (e.g. `"GP002P1S2"`).  The old
/// `(base_id, part_index, session_index)` triple is no longer needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditCommand {
    // ── Panel fields (covers what used to be both panel- and session-level) ──
    SetPanelName {
        panel_id: String,
        old: String,
        new: String,
    },
    SetPanelField {
        panel_id: String,
        field: PanelField,
        old: Option<String>,
        new: Option<String>,
    },
    SetPanelBool {
        panel_id: String,
        field_name: String,
        old: bool,
        new: bool,
    },
    SetPanelDuration {
        panel_id: String,
        old: Option<chrono::Duration>,
        new: Option<chrono::Duration>,
    },

    // ── Presenters on panels ─────────────────────────────────────
    AddPresenterToPanel {
        panel_id: String,
        name: String,
    },
    RemovePresenterFromPanel {
        panel_id: String,
        name: String,
        position: usize,
    },

    // ── Scheduling ───────────────────────────────────────────────
    ReschedulePanel {
        panel_id: String,
        old_state: SessionScheduleState,
        new_state: SessionScheduleState,
    },
    UnschedulePanel {
        panel_id: String,
        old_state: SessionScheduleState,
    },

    // ── Soft delete ──────────────────────────────────────────────
    /// Soft-delete a single flat Panel.
    SoftDeletePanel {
        panel_id: String,
        old_change_state: ChangeState,
    },
    /// Soft-delete all Panels in a PanelSet.
    SoftDeletePanelSet {
        base_id: String,
        old_change_states: Vec<ChangeState>,
    },

    // ── Panel / PanelSet creation ────────────────────────────────
    CreatePanelSet {
        base_id: String,
        source: Option<SourceInfo>,
        change_state: ChangeState,
    },
    CreatePanel {
        panel: crate::data::Panel,
    },

    // ── Entity creation ─────────────────────────────────────────
    CreateRoom {
        uid: u32,
        snapshot: RoomSnapshot,
        source: Option<SourceInfo>,
        change_state: ChangeState,
    },
    CreatePresenter {
        name: String,
        snapshot: PresenterSnapshot,
        source: Option<SourceInfo>,
        change_state: ChangeState,
    },
    CreatePanelType {
        prefix: String,
        snapshot: PanelTypeSnapshot,
        source: Option<SourceInfo>,
        change_state: ChangeState,
    },

    // ── Entity update ─────────────────────────────────────────────
    UpdateRoom {
        uid: u32,
        old: RoomSnapshot,
        new: RoomSnapshot,
    },
    UpdatePresenter {
        name: String,
        old: PresenterSnapshot,
        new: PresenterSnapshot,
    },
    UpdatePanelType {
        prefix: String,
        old: PanelTypeSnapshot,
        new: PanelTypeSnapshot,
    },

    // ── Metadata ─────────────────────────────────────────────────
    SetPanelMetadata {
        panel_id: String,
        key: String,
        old: Option<crate::data::panel::ExtraValue>,
        new: crate::data::panel::ExtraValue,
    },
    ClearPanelMetadata {
        panel_id: String,
        key: String,
        old: crate::data::panel::ExtraValue,
    },
    SetRoomMetadata {
        uid: u32,
        key: String,
        old: Option<crate::data::panel::ExtraValue>,
        new: crate::data::panel::ExtraValue,
    },
    ClearRoomMetadata {
        uid: u32,
        key: String,
        old: crate::data::panel::ExtraValue,
    },
    SetPanelTypeMetadata {
        prefix: String,
        key: String,
        old: Option<crate::data::panel::ExtraValue>,
        new: crate::data::panel::ExtraValue,
    },
    ClearPanelTypeMetadata {
        prefix: String,
        key: String,
        old: crate::data::panel::ExtraValue,
    },

    // ── Presenter lists ──────────────────────────────────────────
    SetPanelPresenters {
        panel_id: String,
        old: Vec<String>,
        new: Vec<String>,
    },

    // ── Batch ────────────────────────────────────────────────────
    Batch(Vec<EditCommand>),
}

impl EditCommand {
    /// Apply this command to the schedule (forward direction).
    pub fn apply(&mut self, schedule: &mut Schedule) {
        match self {
            EditCommand::SetPanelName { panel_id, old, new } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    *old = panel.name.clone();
                    panel.name = new.clone();
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetPanelField {
                panel_id,
                field,
                old,
                new,
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    let mut target = panel_field_ref(panel, field);
                    *old = target.as_string();
                    target.set_from_string(new.clone()).unwrap_or_else(|e| {
                        // Log error but continue - this shouldn't happen in normal usage
                        eprintln!("Error setting panel field: {}", e);
                    });
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetPanelBool {
                panel_id,
                field_name,
                old,
                new,
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    let target = panel_bool_ref(panel, field_name);
                    *old = *target;
                    *target = *new;
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetPanelDuration { panel_id, old, new } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    *old = panel.timing.duration();
                    if let Some(new_duration) = *new {
                        panel.timing.set_duration(new_duration);
                    }

                    mark_panel_modified(panel);
                }
            }
            EditCommand::AddPresenterToPanel { panel_id, name } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    panel.credited_presenters.push(name.clone());
                    mark_panel_modified(panel);
                }
            }
            EditCommand::RemovePresenterFromPanel {
                panel_id,
                name,
                position,
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    if let Some(pos) = panel
                        .credited_presenters
                        .iter()
                        .position(|n| n.eq_ignore_ascii_case(name))
                    {
                        *position = pos;
                        panel.credited_presenters.remove(pos);
                        mark_panel_modified(panel);
                    }
                }
            }
            EditCommand::ReschedulePanel {
                panel_id,
                old_state,
                new_state,
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    *old_state = SessionScheduleState {
                        room_ids: panel.room_ids.clone(),
                        timing: panel.timing.clone(),
                    };
                    panel.room_ids = new_state.room_ids.clone();

                    // Set timing from new state
                    panel.timing = new_state.timing.clone();

                    mark_panel_modified(panel);
                }
            }
            EditCommand::UnschedulePanel {
                panel_id,
                old_state,
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    *old_state = SessionScheduleState {
                        room_ids: panel.room_ids.clone(),
                        timing: panel.timing.clone(),
                    };
                    panel.room_ids.clear();
                    panel.timing = TimeRange::Unspecified;

                    mark_panel_modified(panel);
                }
            }
            EditCommand::SoftDeletePanel {
                panel_id,
                old_change_state,
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    *old_change_state = panel.change_state;
                    panel.change_state = ChangeState::Deleted;
                }
            }
            EditCommand::SoftDeletePanelSet {
                base_id,
                old_change_states,
            } => {
                if let Some(ps) = schedule.panel_sets.get_mut(base_id) {
                    old_change_states.clear();
                    for panel in &mut ps.panels {
                        old_change_states.push(panel.change_state);
                        panel.change_state = ChangeState::Deleted;
                    }
                }
            }
            EditCommand::CreatePanelSet {
                base_id,
                source: _,
                change_state,
            } => {
                use crate::data::PanelSet;
                let mut ps = PanelSet::new(base_id.clone());
                ps.change_state = *change_state;
                schedule.panel_sets.insert(base_id.clone(), ps);
            }
            EditCommand::CreatePanel { panel } => {
                let base_id = panel.base_id.clone();
                let ps = schedule
                    .panel_sets
                    .entry(base_id)
                    .or_insert_with(|| crate::data::PanelSet::new(panel.base_id.clone()));
                ps.panels.push(panel.clone());
            }
            EditCommand::CreateRoom {
                uid,
                snapshot,
                source,
                change_state,
            } => {
                let room = Room {
                    uid: *uid,
                    short_name: snapshot.short_name.clone(),
                    long_name: snapshot.long_name.clone(),
                    hotel_room: snapshot.hotel_room.clone(),
                    sort_key: snapshot.sort_key,
                    is_break: snapshot.is_break,
                    metadata: snapshot.metadata.clone(),
                    source: source.clone(),
                    change_state: *change_state,
                };
                schedule.rooms.push(room);
            }
            EditCommand::CreatePresenter {
                name,
                snapshot,
                source,
                change_state,
            } => {
                let presenter = Presenter {
                    id: None,
                    name: name.clone(),
                    rank: snapshot.rank.clone(),
                    is_member: snapshot.is_member.clone(),
                    is_grouped: snapshot.is_grouped.clone(),
                    sort_rank: snapshot.sort_rank.clone(),
                    metadata: snapshot.metadata.clone(),
                    source: source.clone(),
                    change_state: *change_state,
                };
                schedule.presenters.push(presenter);
            }
            EditCommand::CreatePanelType {
                prefix,
                snapshot,
                source,
                change_state,
            } => {
                let pt = PanelType {
                    prefix: prefix.clone(),
                    kind: snapshot.kind.clone(),
                    colors: snapshot.colors.clone(),
                    is_break: snapshot.is_break,
                    is_cafe: snapshot.is_cafe,
                    is_workshop: snapshot.is_workshop,
                    is_hidden: snapshot.is_hidden,
                    is_room_hours: snapshot.is_room_hours,
                    is_timeline: snapshot.is_timeline,
                    is_private: snapshot.is_private,
                    metadata: snapshot.metadata.clone(),
                    source: source.clone(),
                    change_state: *change_state,
                };
                schedule.panel_types.insert(prefix.clone(), pt);
            }
            EditCommand::UpdateRoom { uid, old, new } => {
                if let Some(room) = schedule.rooms.iter_mut().find(|r| r.uid == *uid) {
                    *old = RoomSnapshot::from_room(room);
                    new.apply_to(room);
                    mark_room_modified(room);
                }
            }
            EditCommand::UpdatePresenter { name, old, new } => {
                if let Some(presenter) = schedule
                    .presenters
                    .iter_mut()
                    .find(|p| p.name.eq_ignore_ascii_case(name))
                {
                    *old = PresenterSnapshot::from_presenter(presenter);
                    new.apply_to(presenter);
                    mark_presenter_modified(presenter);
                }
            }
            EditCommand::UpdatePanelType { prefix, old, new } => {
                if let Some(pt) = schedule.panel_types.get_mut(prefix) {
                    *old = PanelTypeSnapshot::from_panel_type(pt);
                    new.apply_to(pt);
                    mark_panel_type_modified(pt);
                }
            }
            EditCommand::SetPanelMetadata {
                panel_id,
                key,
                old,
                new,
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    *old = panel.metadata.get(key).cloned();
                    panel.metadata.insert(key.clone(), new.clone());
                    mark_panel_modified(panel);
                }
            }
            EditCommand::ClearPanelMetadata { panel_id, key, old } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    if let Some(removed) = panel.metadata.shift_remove(key) {
                        *old = removed;
                        mark_panel_modified(panel);
                    }
                }
            }
            EditCommand::SetRoomMetadata { uid, key, old, new } => {
                if let Some(room) = schedule.rooms.iter_mut().find(|r| r.uid == *uid) {
                    let meta = room.metadata.get_or_insert_with(Default::default);
                    *old = meta.get(key).cloned();
                    meta.insert(key.clone(), new.clone());
                    mark_room_modified(room);
                }
            }
            EditCommand::ClearRoomMetadata { uid, key, old } => {
                if let Some(room) = schedule.rooms.iter_mut().find(|r| r.uid == *uid) {
                    if let Some(meta) = &mut room.metadata {
                        if let Some(removed) = meta.shift_remove(key) {
                            *old = removed;
                            mark_room_modified(room);
                        }
                    }
                }
            }
            EditCommand::SetPanelTypeMetadata {
                prefix,
                key,
                old,
                new,
            } => {
                if let Some(pt) = schedule.panel_types.get_mut(prefix) {
                    let meta = pt.metadata.get_or_insert_with(Default::default);
                    *old = meta.get(key).cloned();
                    meta.insert(key.clone(), new.clone());
                    mark_panel_type_modified(pt);
                }
            }
            EditCommand::ClearPanelTypeMetadata { prefix, key, old } => {
                if let Some(pt) = schedule.panel_types.get_mut(prefix) {
                    if let Some(meta) = &mut pt.metadata {
                        if let Some(removed) = meta.shift_remove(key) {
                            *old = removed;
                            mark_panel_type_modified(pt);
                        }
                    }
                }
            }
            EditCommand::SetPanelPresenters { panel_id, old, new } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    *old = panel.credited_presenters.clone();
                    panel.credited_presenters = new.clone();
                    mark_panel_modified(panel);
                }
            }
            EditCommand::Batch(commands) => {
                for cmd in commands.iter_mut() {
                    cmd.apply(schedule);
                }
            }
        }
    }

    /// Undo this command, reversing its effect on the schedule.
    pub fn undo(&self, schedule: &mut Schedule) {
        match self {
            EditCommand::SetPanelName { panel_id, old, .. } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    panel.name = old.clone();
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetPanelField {
                panel_id,
                field,
                old,
                ..
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    let mut target = panel_field_ref(panel, field);
                    target.set_from_string(old.clone()).unwrap_or_else(|e| {
                        // Log error but continue - this shouldn't happen in normal usage
                        eprintln!("Error undoing panel field: {}", e);
                    });
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetPanelBool {
                panel_id,
                field_name,
                old,
                ..
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    *panel_bool_ref(panel, field_name) = *old;
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetPanelDuration { panel_id, old, .. } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    if let Some(old_duration) = *old {
                        panel.timing.set_duration(old_duration);
                    }

                    mark_panel_modified(panel);
                }
            }
            EditCommand::AddPresenterToPanel { panel_id, .. } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    panel.credited_presenters.pop();
                    mark_panel_modified(panel);
                }
            }
            EditCommand::RemovePresenterFromPanel {
                panel_id,
                name,
                position,
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    let pos = (*position).min(panel.credited_presenters.len());
                    panel.credited_presenters.insert(pos, name.clone());
                    mark_panel_modified(panel);
                }
            }
            EditCommand::ReschedulePanel {
                panel_id,
                old_state,
                ..
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    panel.room_ids = old_state.room_ids.clone();

                    // Restore timing from old state
                    panel.timing = old_state.timing.clone();

                    mark_panel_modified(panel);
                }
            }
            EditCommand::UnschedulePanel {
                panel_id,
                old_state,
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    panel.room_ids = old_state.room_ids.clone();

                    // Restore timing from old state
                    panel.timing = old_state.timing.clone();

                    mark_panel_modified(panel);
                }
            }
            EditCommand::SoftDeletePanel {
                panel_id,
                old_change_state,
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    panel.change_state = *old_change_state;
                }
            }
            EditCommand::SoftDeletePanelSet {
                base_id,
                old_change_states,
            } => {
                if let Some(ps) = schedule.panel_sets.get_mut(base_id) {
                    for (panel, &old_cs) in ps.panels.iter_mut().zip(old_change_states.iter()) {
                        panel.change_state = old_cs;
                    }
                }
            }
            EditCommand::CreatePanelSet { base_id, .. } => {
                schedule.panel_sets.shift_remove(base_id);
            }
            EditCommand::CreatePanel { panel } => {
                if let Some(ps) = schedule.panel_sets.get_mut(&panel.base_id) {
                    ps.panels.retain(|p| p.id != panel.id);
                    if ps.panels.is_empty() {
                        let base_id = panel.base_id.clone();
                        schedule.panel_sets.shift_remove(&base_id);
                    }
                }
            }
            EditCommand::CreateRoom { uid, .. } => {
                schedule.rooms.retain(|r| r.uid != *uid);
            }
            EditCommand::CreatePresenter { name, .. } => {
                schedule
                    .presenters
                    .retain(|p| !p.name.eq_ignore_ascii_case(name));
            }
            EditCommand::CreatePanelType { prefix, .. } => {
                schedule.panel_types.shift_remove(prefix);
            }
            EditCommand::UpdateRoom { uid, old, .. } => {
                if let Some(room) = schedule.rooms.iter_mut().find(|r| r.uid == *uid) {
                    old.apply_to(room);
                    mark_room_modified(room);
                }
            }
            EditCommand::UpdatePresenter { name, old, .. } => {
                if let Some(presenter) = schedule
                    .presenters
                    .iter_mut()
                    .find(|p| p.name.eq_ignore_ascii_case(name))
                {
                    old.apply_to(presenter);
                    mark_presenter_modified(presenter);
                }
            }
            EditCommand::UpdatePanelType { prefix, old, .. } => {
                if let Some(pt) = schedule.panel_types.get_mut(prefix) {
                    old.apply_to(pt);
                    mark_panel_type_modified(pt);
                }
            }
            EditCommand::SetPanelMetadata {
                panel_id, key, old, ..
            } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    match old {
                        Some(val) => {
                            panel.metadata.insert(key.clone(), val.clone());
                        }
                        None => {
                            panel.metadata.shift_remove(key);
                        }
                    }
                    mark_panel_modified(panel);
                }
            }
            EditCommand::ClearPanelMetadata { panel_id, key, old } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    panel.metadata.insert(key.clone(), old.clone());
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetRoomMetadata { uid, key, old, .. } => {
                if let Some(room) = schedule.rooms.iter_mut().find(|r| r.uid == *uid) {
                    match old {
                        Some(val) => {
                            room.metadata
                                .get_or_insert_with(Default::default)
                                .insert(key.clone(), val.clone());
                        }
                        None => {
                            if let Some(meta) = &mut room.metadata {
                                meta.shift_remove(key);
                            }
                        }
                    }
                    mark_room_modified(room);
                }
            }
            EditCommand::ClearRoomMetadata { uid, key, old } => {
                if let Some(room) = schedule.rooms.iter_mut().find(|r| r.uid == *uid) {
                    room.metadata
                        .get_or_insert_with(Default::default)
                        .insert(key.clone(), old.clone());
                    mark_room_modified(room);
                }
            }
            EditCommand::SetPanelTypeMetadata {
                prefix, key, old, ..
            } => {
                if let Some(pt) = schedule.panel_types.get_mut(prefix) {
                    match old {
                        Some(val) => {
                            pt.metadata
                                .get_or_insert_with(Default::default)
                                .insert(key.clone(), val.clone());
                        }
                        None => {
                            if let Some(meta) = &mut pt.metadata {
                                meta.shift_remove(key);
                            }
                        }
                    }
                    mark_panel_type_modified(pt);
                }
            }
            EditCommand::ClearPanelTypeMetadata { prefix, key, old } => {
                if let Some(pt) = schedule.panel_types.get_mut(prefix) {
                    pt.metadata
                        .get_or_insert_with(Default::default)
                        .insert(key.clone(), old.clone());
                    mark_panel_type_modified(pt);
                }
            }
            EditCommand::SetPanelPresenters { panel_id, old, .. } => {
                if let Some(panel) = get_panel_mut(schedule, panel_id) {
                    panel.credited_presenters = old.clone();
                    mark_panel_modified(panel);
                }
            }
            EditCommand::Batch(commands) => {
                for cmd in commands.iter().rev() {
                    cmd.undo(schedule);
                }
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Look up a flat Panel by its full Uniq ID across all PanelSets.
fn get_panel_mut<'a>(
    schedule: &'a mut Schedule,
    panel_id: &str,
) -> Option<&'a mut crate::data::Panel> {
    schedule
        .panel_sets
        .values_mut()
        .flat_map(|ps| ps.panels.iter_mut())
        .find(|p| p.id == panel_id)
}

fn panel_field_ref<'a>(panel: &'a mut crate::data::Panel, field: &PanelField) -> PanelFieldRef<'a> {
    match field {
        PanelField::Description => PanelFieldRef::String(&mut panel.description),
        PanelField::Note => PanelFieldRef::String(&mut panel.note),
        PanelField::Prereq => PanelFieldRef::String(&mut panel.prereq),
        PanelField::Cost => PanelFieldRef::String(&mut panel.cost),
        PanelField::Capacity => PanelFieldRef::String(&mut panel.capacity),
        PanelField::Difficulty => PanelFieldRef::String(&mut panel.difficulty),
        PanelField::PanelType => PanelFieldRef::String(&mut panel.panel_type),
        PanelField::AltPanelist => PanelFieldRef::String(&mut panel.alt_panelist),
        PanelField::PreRegMax => PanelFieldRef::String(&mut panel.pre_reg_max),
        PanelField::TicketUrl => PanelFieldRef::String(&mut panel.ticket_url),
        PanelField::SimpleTicketEvent => PanelFieldRef::String(&mut panel.simple_tix_event),
        PanelField::HaveTicketImage => {
            // HaveTicketImage is bool stored as Option<bool>; return description as proxy
            return PanelFieldRef::String(&mut panel.description);
        }
        PanelField::StartTime => PanelFieldRef::StartTime(&mut panel.timing),
        PanelField::EndTime => PanelFieldRef::EndTime(&mut panel.timing),
        PanelField::AvNotes => PanelFieldRef::String(&mut panel.av_notes),
        PanelField::NotesNonPrinting => PanelFieldRef::String(&mut panel.notes_non_printing),
        PanelField::WorkshopNotes => PanelFieldRef::String(&mut panel.workshop_notes),
        PanelField::PowerNeeds => PanelFieldRef::String(&mut panel.power_needs),
    }
}

/// Enum representing different field reference types for flexible editing
pub enum PanelFieldRef<'a> {
    String(&'a mut Option<String>),
    DateTime(&'a mut Option<chrono::NaiveDateTime>),
    StartTime(&'a mut crate::data::time::TimeRange),
    EndTime(&'a mut crate::data::time::TimeRange),
    Duration(&'a mut crate::data::time::TimeRange),
}

impl<'a> PanelFieldRef<'a> {
    /// Get the current value as a string for serialization/editing
    pub fn as_string(&self) -> Option<String> {
        match self {
            PanelFieldRef::String(opt) => (*opt).clone(),
            PanelFieldRef::DateTime(opt) => opt.map(|dt| crate::data::time::format_storage(dt)),
            PanelFieldRef::StartTime(timerange) => timerange.start_time_str(),
            PanelFieldRef::EndTime(timerange) => timerange.end_time_str(),
            PanelFieldRef::Duration(timerange) => timerange.duration_minutes_str(),
        }
    }

    /// Set the value from a string input (with parsing for datetime fields)
    pub fn set_from_string(&mut self, value: Option<String>) -> Result<(), String> {
        match self {
            PanelFieldRef::String(opt) => {
                **opt = value;
                Ok(())
            }
            PanelFieldRef::DateTime(opt) => {
                if let Some(s) = value {
                    if let Some(dt) = crate::data::time::parse_datetime(&s) {
                        **opt = Some(dt);
                        Ok(())
                    } else {
                        Err(format!("Invalid datetime format: {}", s))
                    }
                } else {
                    **opt = None;
                    Ok(())
                }
            }
            PanelFieldRef::StartTime(timerange) => {
                if let Some(s) = value {
                    if timerange.set_start_time_from_str(&s) {
                        Ok(())
                    } else {
                        Err(format!("Invalid datetime format: {}", s))
                    }
                } else {
                    timerange.clear_start_time();
                    Ok(())
                }
            }
            PanelFieldRef::EndTime(timerange) => {
                if let Some(s) = value {
                    if let Some(dt) = crate::data::time::parse_datetime(&s) {
                        // Use preserve_start method for user-friendly end time setting
                        timerange.set_end_time_preserve_start(dt);
                        Ok(())
                    } else {
                        Err(format!("Invalid datetime format: {}", s))
                    }
                } else {
                    timerange.clear_end_time();
                    Ok(())
                }
            }
            PanelFieldRef::Duration(timerange) => {
                if let Some(s) = value {
                    if timerange.set_duration_from_str(&s) {
                        Ok(())
                    } else {
                        Err(format!("Invalid duration format: {}", s))
                    }
                } else {
                    timerange.clear_duration();
                    Ok(())
                }
            }
        }
    }
}

fn panel_bool_ref<'a>(panel: &'a mut crate::data::Panel, field_name: &str) -> &'a mut bool {
    match field_name {
        "is_free" => &mut panel.is_free,
        "is_kids" => &mut panel.is_kids,
        _ => unreachable!("Unknown panel bool field: {}", field_name),
    }
}

fn mark_panel_modified(panel: &mut crate::data::Panel) {
    if panel.change_state == ChangeState::Unchanged {
        panel.change_state = ChangeState::Modified;
    }
}

fn mark_room_modified(room: &mut Room) {
    if room.change_state == ChangeState::Unchanged {
        room.change_state = ChangeState::Modified;
    }
}

fn mark_presenter_modified(presenter: &mut Presenter) {
    if presenter.change_state == ChangeState::Unchanged {
        presenter.change_state = ChangeState::Modified;
    }
}

fn mark_panel_type_modified(pt: &mut PanelType) {
    if pt.change_state == ChangeState::Unchanged {
        pt.change_state = ChangeState::Modified;
    }
}
