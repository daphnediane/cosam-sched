/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::BTreeSet;

use chrono::NaiveDate;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::panel::Panel;
use super::panel_set::PanelSet;
use super::panel_type::PanelType;
use super::presenter::{Presenter, PresenterGroup, PresenterMember};
use super::relationship::{GroupEdge, RelationshipManager};
use super::room::Room;
use super::source_info::{ChangeState, ImportedSheetPresence};
use super::time;
use super::timeline::TimelineEntry;

/// Lightweight struct for displaying a panel session in the editor UI
#[derive(Debug, Clone)]
pub struct SessionDisplayInfo {
    pub session_id: String,
    pub base_id: String,
    pub name: String,
    pub panel_type: Option<String>,
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
    pub room_ids: Vec<u32>,
    pub presenters: Vec<String>,
    pub is_full: bool,
    pub change_state: ChangeState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    pub generated: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_presenter_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConflictEventRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleConflict {
    pub event1: ConflictEventRef,
    pub event2: ConflictEventRef,
    #[serde(default)]
    pub presenter: Option<String>,
    #[serde(default)]
    pub room: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub conflict_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<ScheduleConflict>,
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timeline: Vec<TimelineEntry>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub panel_sets: IndexMap<String, PanelSet>,
    pub rooms: Vec<Room>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub panel_types: IndexMap<String, PanelType>,
    pub presenters: Vec<Presenter>,
    #[serde(skip)]
    pub relationships: RelationshipManager,
    #[serde(default, skip_serializing)]
    pub imported_sheets: ImportedSheetPresence,
}

impl Schedule {
    /// Iterate all panels across all panel sets.
    pub fn all_panels(&self) -> impl Iterator<Item = &Panel> {
        self.panel_sets.values().flat_map(|ps| ps.panels.iter())
    }

    /// Iterate all panels mutably across all panel sets.
    pub fn all_panels_mut(&mut self) -> impl Iterator<Item = &mut Panel> {
        self.panel_sets
            .values_mut()
            .flat_map(|ps| ps.panels.iter_mut())
    }

    /// Calculate schedule start and end times from panel_sets and timeline entries
    pub fn calculate_schedule_bounds(&mut self) {
        let mut min_time: Option<chrono::NaiveDateTime> = None;
        let mut max_time: Option<chrono::NaiveDateTime> = None;

        for panel in self.all_panels() {
            if let Some(start) = panel.timing.start_time() {
                if min_time.is_none() || Some(start) < min_time {
                    min_time = Some(start);
                }
                let end = panel.effective_end_time().unwrap_or_else(|| {
                    let start = panel.timing.start_time().unwrap();
                    start
                        + panel
                            .effective_duration()
                            .unwrap_or(chrono::Duration::minutes(60))
                });
                if max_time.is_none() || Some(end) > max_time {
                    max_time = Some(end);
                }
            }
        }

        // Check timeline entries
        for timeline_entry in &self.timeline {
            if let Some(start_time) = timeline_entry.start_time {
                if min_time.is_none() || Some(start_time) < min_time {
                    min_time = Some(start_time);
                }
            }
        }

        // Set meta fields
        if let Some(min_time) = min_time {
            self.meta.start_time = Some(time::format_storage_ts(min_time.and_utc()));
        }
        if let Some(max_time) = max_time {
            self.meta.end_time = Some(time::format_storage_ts(max_time.and_utc()));
        }

        // If still no times found, set reasonable defaults for Cosplay America
        // @todo assume the last weekend of the upcoming June.
        if self.meta.start_time.is_none() {
            self.meta.start_time = Some("2026-06-25T17:00:00Z".to_string()); // Thursday evening
        }
        if self.meta.end_time.is_none() {
            self.meta.end_time = Some("2026-06-28T18:00:00Z".to_string()); // Sunday evening
        }
    }

    #[must_use]
    pub fn days(&self) -> Vec<NaiveDate> {
        let mut dates: BTreeSet<NaiveDate> = BTreeSet::new();

        for panel in self.all_panels() {
            if let Some(start) = panel.timing.start_time() {
                dates.insert(start.date());
            }
        }

        dates.into_iter().collect()
    }

    /// Returns panel display info for a given day from the flat panel_sets model.
    #[must_use]
    pub fn sessions_for_day(&self, day: &NaiveDate) -> Vec<SessionDisplayInfo> {
        let mut results = Vec::new();
        for panel in self.all_panels() {
            let start_dt = panel.timing.start_time();
            if let Some(start) = start_dt {
                if start.date() == *day {
                    let end_dt = panel.effective_end_time().unwrap_or_else(|| {
                        start
                            + panel
                                .effective_duration()
                                .unwrap_or(chrono::Duration::minutes(60))
                    });

                    results.push(SessionDisplayInfo {
                        session_id: panel.id.clone(),
                        base_id: panel.base_id.clone(),
                        name: panel.name.clone(),
                        panel_type: panel.panel_type.clone(),
                        start_time: start,
                        end_time: end_dt,
                        room_ids: panel.room_ids.clone(),
                        presenters: panel.credited_presenters.clone(),
                        is_full: panel.is_full,
                        change_state: panel.change_state,
                    });
                }
            }
        }
        results.sort_by_key(|s| s.start_time);
        results
    }

    #[must_use]
    pub fn room_by_id(&self, uid: u32) -> Option<&Room> {
        self.rooms.iter().find(|r| r.uid == uid)
    }

    #[must_use]
    pub fn sorted_rooms(&self) -> Vec<&Room> {
        let mut rooms: Vec<&Room> = self.rooms.iter().collect();
        rooms.sort_by_key(|r| r.sort_key);
        rooms
    }

    #[must_use]
    pub fn panel_type_by_prefix(&self, prefix: &str) -> Option<&PanelType> {
        self.panel_types.get(prefix)
    }

    /// Populate `self.relationships` from the legacy `is_member`/`is_grouped`
    /// fields on each `Presenter`.  Call after deserialization to bridge old
    /// data into the new edge-based model.
    pub fn build_relationships_from_presenters(&mut self) {
        self.relationships = RelationshipManager::new();
        for presenter in &self.presenters {
            match &presenter.is_grouped {
                PresenterGroup::IsGroup(members, always_shown) => {
                    if members.is_empty() {
                        // Group-only edge (group exists but has no members listed here)
                        self.relationships
                            .add_edge(GroupEdge::group_only(presenter.name.clone(), *always_shown));
                    } else {
                        for member in members {
                            // Preserve always_grouped from any existing edge
                            let existing_ag = self
                                .relationships
                                .find_edge(member, &presenter.name)
                                .map(|e| e.always_grouped)
                                .unwrap_or(false);
                            self.relationships.add_edge(GroupEdge {
                                member: member.clone(),
                                group: presenter.name.clone(),
                                always_grouped: existing_ag,
                                always_shown: *always_shown,
                            });
                        }
                    }
                }
                PresenterGroup::NotGroup => {}
            }
            match &presenter.is_member {
                PresenterMember::IsMember(groups, always_grouped) => {
                    for group in groups {
                        // Preserve always_shown from any existing edge
                        let existing_as = self
                            .relationships
                            .find_edge(&presenter.name, group)
                            .map(|e| e.always_shown)
                            .unwrap_or(false);
                        self.relationships.add_edge(GroupEdge {
                            member: presenter.name.clone(),
                            group: group.clone(),
                            always_grouped: *always_grouped,
                            always_shown: existing_as,
                        });
                    }
                }
                PresenterMember::NotMember => {}
            }
        }
    }

    /// Update each `Presenter`'s `is_member`/`is_grouped` fields from the
    /// current `RelationshipManager` state.  Call before serialization to
    /// ensure that changes made via `AddRelationship`/`RemoveRelationship`
    /// commands are reflected in the serialized output.
    pub fn sync_presenters_from_relationships(&mut self) {
        for presenter in &mut self.presenters {
            let name = &presenter.name;
            let groups: std::collections::BTreeSet<String> = self
                .relationships
                .direct_groups_of(name)
                .iter()
                .cloned()
                .collect();
            let members: std::collections::BTreeSet<String> = self
                .relationships
                .direct_members_of(name)
                .iter()
                .cloned()
                .collect();
            let is_group = self.relationships.is_group(name);
            let always_grouped = self.relationships.is_any_always_grouped(name);
            let always_shown = self.relationships.is_always_shown(name);

            presenter.is_member = if groups.is_empty() {
                PresenterMember::NotMember
            } else {
                PresenterMember::IsMember(groups, always_grouped)
            };

            presenter.is_grouped = if is_group {
                PresenterGroup::IsGroup(members, always_shown)
            } else {
                PresenterGroup::NotGroup
            };
        }
    }

    pub fn populate_panel_type_prefixes(&mut self) {
        for (prefix, panel_type) in &mut self.panel_types {
            panel_type.prefix = prefix.clone();
        }
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            conflicts: Vec::new(),
            meta: Meta::default(),
            timeline: Vec::new(),
            panel_sets: IndexMap::new(),
            rooms: Vec::new(),
            panel_types: IndexMap::new(),
            presenters: Vec::new(),
            relationships: RelationshipManager::new(),
            imported_sheets: ImportedSheetPresence::default(),
        }
    }
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            title: "Event Schedule".to_string(),
            generated: time::format_storage_ts(chrono::Utc::now()),
            version: Some(7),
            variant: None,
            generator: Some("cosam-sched".to_string()),
            start_time: None,
            end_time: None,
            next_presenter_id: None,
            creator: None,
            last_modified_by: None,
            modified: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_minimal_json() -> &'static str {
        r#"{"meta":{"title":"Test","generated":"2026-01-01T00:00:00Z"},"rooms":[],"presenters":[]}"#
    }

    #[test]
    fn test_roundtrip_serialization() {
        let schedule: Schedule = serde_json::from_str(make_minimal_json()).unwrap();
        let json = serde_json::to_string_pretty(&schedule).unwrap();
        let schedule2: Schedule = serde_json::from_str(&json).unwrap();
        assert_eq!(schedule, schedule2);
    }

    #[test]
    fn test_unknown_fields_ignored() {
        let json = r#"{"meta":{"title":"T","generated":"2026-01-01T00:00:00Z"},"rooms":[],"presenters":[],"changeLog":{"undoStack":[],"redoStack":[],"maxDepth":50}}"#;
        let schedule: Schedule = serde_json::from_str(json).unwrap();
        assert_eq!(schedule.meta.title, "T");
    }

    #[test]
    fn test_malformed_json_fails() {
        let result: Result<Schedule, _> = serde_json::from_str("{ not valid json }");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_relationships_from_presenters() {
        use crate::data::presenter::{PresenterGroup, PresenterMember, PresenterRank};

        let mut schedule = Schedule::default();
        schedule.presenters = vec![
            Presenter {
                id: Some(1),
                name: "Pros and Cons Cosplay".to_string(),
                rank: PresenterRank::Guest,
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::IsGroup(
                    ["Pro".to_string(), "Con".to_string()].into_iter().collect(),
                    false,
                ),
                sort_rank: None,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                id: Some(2),
                name: "Pro".to_string(),
                rank: PresenterRank::Guest,
                is_member: PresenterMember::IsMember(
                    ["Pros and Cons Cosplay".to_string()].into_iter().collect(),
                    true,
                ),
                is_grouped: PresenterGroup::NotGroup,
                sort_rank: None,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                id: Some(3),
                name: "Con".to_string(),
                rank: PresenterRank::Guest,
                is_member: PresenterMember::IsMember(
                    ["Pros and Cons Cosplay".to_string()].into_iter().collect(),
                    false,
                ),
                is_grouped: PresenterGroup::NotGroup,
                sort_rank: None,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
        ];

        schedule.build_relationships_from_presenters();

        // Group should be recognized
        assert!(schedule.relationships.is_group("Pros and Cons Cosplay"));
        assert!(!schedule.relationships.is_group("Pro"));

        // Direct members
        let members = schedule
            .relationships
            .get_direct_members("Pros and Cons Cosplay");
        assert!(members.contains(&"Pro".to_string()));
        assert!(members.contains(&"Con".to_string()));

        // Direct groups
        let groups = schedule.relationships.get_direct_groups("Pro");
        assert!(groups.contains(&"Pros and Cons Cosplay".to_string()));

        // always_grouped flag (Pro has it, Con doesn't)
        assert!(schedule.relationships.is_any_always_grouped("Pro"));
        assert!(!schedule.relationships.is_any_always_grouped("Con"));
    }
}
