/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use indexmap::IndexMap;

use crate::data::panel::ExtraFields;
use crate::data::panel_type::PanelType;
use crate::data::presenter::{Presenter, PresenterGroup, PresenterMember, PresenterRank};
use crate::data::room::Room;
use crate::data::schedule::Schedule;
use crate::data::source_info::{ChangeState, SourceInfo};

/// Identifies which string field on a panel to set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelField {
    Description,
    Note,
    Prereq,
    Cost,
    Capacity,
    Difficulty,
    PanelType,
    AltPanelist,
}

/// Identifies which string field on a session to set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionField {
    Description,
    Note,
    Prereq,
    AltPanelist,
    Capacity,
    AvNotes,
    StartTime,
    EndTime,
}

/// Snapshot of scheduling-related session state for undo.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionScheduleState {
    pub room_ids: Vec<u32>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub duration: u32,
}

/// Snapshot of a room's mutable fields for undo.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct PresenterSnapshot {
    pub rank: PresenterRank,
    pub is_member: PresenterMember,
    pub is_grouped: PresenterGroup,
    pub metadata: Option<ExtraFields>,
}

impl PresenterSnapshot {
    pub fn from_presenter(p: &Presenter) -> Self {
        Self {
            rank: p.rank.clone(),
            is_member: p.is_member.clone(),
            is_grouped: p.is_grouped.clone(),
            metadata: p.metadata.clone(),
        }
    }

    pub fn apply_to(&self, p: &mut Presenter) {
        p.rank = self.rank.clone();
        p.is_member = self.is_member.clone();
        p.is_grouped = self.is_grouped.clone();
        p.metadata = self.metadata.clone();
    }
}

/// Snapshot of a panel type's mutable fields for undo.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone)]
pub enum EditCommand {
    // ── Panel fields ────────────────────────────────────────────
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

    // ── Session fields ──────────────────────────────────────────
    SetSessionField {
        panel_id: String,
        part_index: usize,
        session_index: usize,
        field: SessionField,
        old: Option<String>,
        new: Option<String>,
    },
    SetSessionDuration {
        panel_id: String,
        part_index: usize,
        session_index: usize,
        old: u32,
        new: u32,
    },

    // ── Presenters on sessions ──────────────────────────────────
    AddPresenterToSession {
        panel_id: String,
        part_index: usize,
        session_index: usize,
        name: String,
    },
    RemovePresenterFromSession {
        panel_id: String,
        part_index: usize,
        session_index: usize,
        name: String,
        position: usize,
    },

    // ── Scheduling ──────────────────────────────────────────────
    RescheduleSession {
        panel_id: String,
        part_index: usize,
        session_index: usize,
        old_state: SessionScheduleState,
        new_state: SessionScheduleState,
    },
    UnscheduleSession {
        panel_id: String,
        part_index: usize,
        session_index: usize,
        old_state: SessionScheduleState,
    },

    // ── Soft delete ─────────────────────────────────────────────
    SoftDeleteSession {
        panel_id: String,
        part_index: usize,
        session_index: usize,
        old_change_state: ChangeState,
    },
    SoftDeletePanel {
        panel_id: String,
        old_change_state: ChangeState,
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

    // ── Entity update ───────────────────────────────────────────
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

    // ── Metadata ────────────────────────────────────────────────
    SetSessionMetadata {
        panel_id: String,
        part_index: usize,
        session_index: usize,
        key: String,
        old: Option<crate::data::panel::ExtraValue>,
        new: crate::data::panel::ExtraValue,
    },
    ClearSessionMetadata {
        panel_id: String,
        part_index: usize,
        session_index: usize,
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

    // ── Presenter lists ─────────────────────────────────────────
    SetPanelPresenters {
        panel_id: String,
        old: Vec<String>,
        new: Vec<String>,
    },
    SetSessionPresenters {
        panel_id: String,
        part_index: usize,
        session_index: usize,
        old: Vec<String>,
        new: Vec<String>,
    },

    // ── Batch ───────────────────────────────────────────────────
    Batch(Vec<EditCommand>),
}

impl EditCommand {
    /// Apply this command to the schedule (forward direction).
    pub fn apply(&mut self, schedule: &mut Schedule) {
        match self {
            EditCommand::SetPanelName { panel_id, old, new } => {
                if let Some(panel) = schedule.panels.get_mut(panel_id) {
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
                if let Some(panel) = schedule.panels.get_mut(panel_id) {
                    let target = panel_field_ref(panel, field);
                    *old = target.clone();
                    *target = new.clone();
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetPanelBool {
                panel_id,
                field_name,
                old,
                new,
            } => {
                if let Some(panel) = schedule.panels.get_mut(panel_id) {
                    let target = panel_bool_ref(panel, field_name);
                    *old = *target;
                    *target = *new;
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetSessionField {
                panel_id,
                part_index,
                session_index,
                field,
                old,
                new,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    let target = session_field_ref(session, field);
                    *old = target.clone();
                    *target = new.clone();
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::SetSessionDuration {
                panel_id,
                part_index,
                session_index,
                old,
                new,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    *old = session.duration;
                    session.duration = *new;
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::AddPresenterToSession {
                panel_id,
                part_index,
                session_index,
                name,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    session.credited_presenters.push(name.clone());
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::RemovePresenterFromSession {
                panel_id,
                part_index,
                session_index,
                name,
                position,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    if let Some(pos) = session
                        .credited_presenters
                        .iter()
                        .position(|n| n.eq_ignore_ascii_case(name))
                    {
                        *position = pos;
                        session.credited_presenters.remove(pos);
                        mark_session_modified(session);
                    }
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::RescheduleSession {
                panel_id,
                part_index,
                session_index,
                old_state,
                new_state,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    *old_state = SessionScheduleState {
                        room_ids: session.room_ids.clone(),
                        start_time: session.start_time.clone(),
                        end_time: session.end_time.clone(),
                        duration: session.duration,
                    };
                    session.room_ids = new_state.room_ids.clone();
                    session.start_time = new_state.start_time.clone();
                    session.end_time = new_state.end_time.clone();
                    session.duration = new_state.duration;
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::UnscheduleSession {
                panel_id,
                part_index,
                session_index,
                old_state,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    *old_state = SessionScheduleState {
                        room_ids: session.room_ids.clone(),
                        start_time: session.start_time.clone(),
                        end_time: session.end_time.clone(),
                        duration: session.duration,
                    };
                    session.room_ids.clear();
                    session.start_time = None;
                    session.end_time = None;
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::SoftDeleteSession {
                panel_id,
                part_index,
                session_index,
                old_change_state,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    *old_change_state = session.change_state;
                    session.change_state = ChangeState::Deleted;
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::SoftDeletePanel {
                panel_id,
                old_change_state,
            } => {
                if let Some(panel) = schedule.panels.get_mut(panel_id) {
                    *old_change_state = panel.change_state;
                    panel.change_state = ChangeState::Deleted;
                    for part in &mut panel.parts {
                        for session in &mut part.sessions {
                            session.change_state = ChangeState::Deleted;
                        }
                    }
                }
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
            EditCommand::SetSessionMetadata {
                panel_id,
                part_index,
                session_index,
                key,
                old,
                new,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    *old = session.metadata.get(key).cloned();
                    session.metadata.insert(key.clone(), new.clone());
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::ClearSessionMetadata {
                panel_id,
                part_index,
                session_index,
                key,
                old,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    if let Some(removed) = session.metadata.shift_remove(key) {
                        *old = removed;
                        mark_session_modified(session);
                    }
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
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
                if let Some(panel) = schedule.panels.get_mut(panel_id) {
                    *old = panel.credited_presenters.clone();
                    panel.credited_presenters = new.clone();
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetSessionPresenters {
                panel_id,
                part_index,
                session_index,
                old,
                new,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    *old = session.credited_presenters.clone();
                    session.credited_presenters = new.clone();
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
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
                if let Some(panel) = schedule.panels.get_mut(panel_id) {
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
                if let Some(panel) = schedule.panels.get_mut(panel_id) {
                    *panel_field_ref(panel, field) = old.clone();
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetPanelBool {
                panel_id,
                field_name,
                old,
                ..
            } => {
                if let Some(panel) = schedule.panels.get_mut(panel_id) {
                    *panel_bool_ref(panel, field_name) = *old;
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetSessionField {
                panel_id,
                part_index,
                session_index,
                field,
                old,
                ..
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    *session_field_ref(session, field) = old.clone();
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::SetSessionDuration {
                panel_id,
                part_index,
                session_index,
                old,
                ..
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    session.duration = *old;
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::AddPresenterToSession {
                panel_id,
                part_index,
                session_index,
                ..
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    session.credited_presenters.pop();
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::RemovePresenterFromSession {
                panel_id,
                part_index,
                session_index,
                name,
                position,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    let pos = (*position).min(session.credited_presenters.len());
                    session.credited_presenters.insert(pos, name.clone());
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::RescheduleSession {
                panel_id,
                part_index,
                session_index,
                old_state,
                ..
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    session.room_ids = old_state.room_ids.clone();
                    session.start_time = old_state.start_time.clone();
                    session.end_time = old_state.end_time.clone();
                    session.duration = old_state.duration;
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::UnscheduleSession {
                panel_id,
                part_index,
                session_index,
                old_state,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    session.room_ids = old_state.room_ids.clone();
                    session.start_time = old_state.start_time.clone();
                    session.end_time = old_state.end_time.clone();
                    session.duration = old_state.duration;
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::SoftDeleteSession {
                panel_id,
                part_index,
                session_index,
                old_change_state,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    session.change_state = *old_change_state;
                }
            }
            EditCommand::SoftDeletePanel {
                panel_id,
                old_change_state,
            } => {
                if let Some(panel) = schedule.panels.get_mut(panel_id) {
                    panel.change_state = *old_change_state;
                    // Restoring sub-item change states would require storing
                    // them all; for now we mark them Modified which is safe.
                    for part in &mut panel.parts {
                        for session in &mut part.sessions {
                            if session.change_state == ChangeState::Deleted {
                                session.change_state = ChangeState::Modified;
                            }
                        }
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
            EditCommand::SetSessionMetadata {
                panel_id,
                part_index,
                session_index,
                key,
                old,
                ..
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    match old {
                        Some(val) => {
                            session.metadata.insert(key.clone(), val.clone());
                        }
                        None => {
                            session.metadata.shift_remove(key);
                        }
                    }
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::ClearSessionMetadata {
                panel_id,
                part_index,
                session_index,
                key,
                old,
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    session.metadata.insert(key.clone(), old.clone());
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
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
                if let Some(panel) = schedule.panels.get_mut(panel_id) {
                    panel.credited_presenters = old.clone();
                    mark_panel_modified(panel);
                }
            }
            EditCommand::SetSessionPresenters {
                panel_id,
                part_index,
                session_index,
                old,
                ..
            } => {
                if let Some(session) =
                    get_session_mut(schedule, panel_id, *part_index, *session_index)
                {
                    session.credited_presenters = old.clone();
                    mark_session_modified(session);
                }
                mark_panel_chain_modified(schedule, panel_id, *part_index);
            }
            EditCommand::Batch(commands) => {
                for cmd in commands.iter().rev() {
                    cmd.undo(schedule);
                }
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────

fn panel_field_ref<'a>(
    panel: &'a mut crate::data::Panel,
    field: &PanelField,
) -> &'a mut Option<String> {
    match field {
        PanelField::Description => &mut panel.description,
        PanelField::Note => &mut panel.note,
        PanelField::Prereq => &mut panel.prereq,
        PanelField::Cost => &mut panel.cost,
        PanelField::Capacity => &mut panel.capacity,
        PanelField::Difficulty => &mut panel.difficulty,
        PanelField::PanelType => &mut panel.panel_type,
        PanelField::AltPanelist => &mut panel.alt_panelist,
    }
}

fn panel_bool_ref<'a>(panel: &'a mut crate::data::Panel, field_name: &str) -> &'a mut bool {
    match field_name {
        "is_free" => &mut panel.is_free,
        "is_kids" => &mut panel.is_kids,
        _ => unreachable!("Unknown panel bool field: {}", field_name),
    }
}

fn session_field_ref<'a>(
    session: &'a mut crate::data::PanelSession,
    field: &SessionField,
) -> &'a mut Option<String> {
    match field {
        SessionField::Description => &mut session.description,
        SessionField::Note => &mut session.note,
        SessionField::Prereq => &mut session.prereq,
        SessionField::AltPanelist => &mut session.alt_panelist,
        SessionField::Capacity => &mut session.capacity,
        SessionField::AvNotes => &mut session.av_notes,
        SessionField::StartTime => &mut session.start_time,
        SessionField::EndTime => &mut session.end_time,
    }
}

fn get_session_mut<'a>(
    schedule: &'a mut Schedule,
    panel_id: &str,
    part_index: usize,
    session_index: usize,
) -> Option<&'a mut crate::data::PanelSession> {
    schedule
        .panels
        .get_mut(panel_id)?
        .parts
        .get_mut(part_index)?
        .sessions
        .get_mut(session_index)
}

fn mark_panel_modified(panel: &mut crate::data::Panel) {
    if panel.change_state == ChangeState::Unchanged {
        panel.change_state = ChangeState::Modified;
    }
}

fn mark_session_modified(session: &mut crate::data::PanelSession) {
    if session.change_state == ChangeState::Unchanged {
        session.change_state = ChangeState::Modified;
    }
}

fn mark_panel_chain_modified(schedule: &mut Schedule, panel_id: &str, part_index: usize) {
    if let Some(panel) = schedule.panels.get_mut(panel_id) {
        if let Some(part) = panel.parts.get_mut(part_index) {
            if part.change_state == ChangeState::Unchanged {
                part.change_state = ChangeState::Modified;
            }
        }
        mark_panel_modified(panel);
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
