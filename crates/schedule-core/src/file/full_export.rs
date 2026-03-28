/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use serde::{Deserialize, Serialize};

use anyhow::{Context, Result};
use std::path::Path;

use crate::data::panel::ExtraFields;
use crate::data::panel_set::PanelSet;
use crate::data::panel_type::PanelType;
use crate::data::presenter::{Presenter, PresenterRank, PresenterSortRank};
use crate::data::relationship::RelationshipManager;
use crate::data::room::Room;
use crate::data::schedule::{Meta, Schedule, ScheduleConflict};
use crate::data::time;
use crate::data::timeline::TimelineEntry;
use crate::edit::history::EditHistory;

/// Full-format presenter for JSON export with flat relationship fields.
///
/// This struct mirrors the current Presenter serialization format but uses
/// flat fields instead of enum-based PresenterMember/PresenterGroup.
/// It queries the RelationshipManager for group membership data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullPresenter {
    pub name: String,
    pub rank: PresenterRank,
    /// Flat field indicating if this presenter is a group
    pub is_group: bool,
    /// Flat field listing direct members (only populated for groups)
    pub members: Vec<String>,
    /// Flat field listing direct groups this presenter belongs to
    pub groups: Vec<String>,
    /// Flat field indicating if this presenter should always be grouped with its groups
    pub always_grouped: bool,
    /// Flat field indicating if this group should always be shown as a group
    pub always_shown: bool,
    /// Ordering key recording where this presenter was first defined
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_rank: Option<PresenterSortRank>,
    /// Additional metadata fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ExtraFields>,
}

impl FullPresenter {
    /// Create a FullPresenter from a Presenter and RelationshipManager.
    ///
    /// This converts from the internal enum-based relationship storage
    /// to flat fields suitable for JSON serialization.
    pub fn from_presenter(presenter: &Presenter, relationships: &RelationshipManager) -> Self {
        Self {
            name: presenter.name.clone(),
            rank: presenter.rank.clone(),
            is_group: relationships.is_group(&presenter.name),
            members: relationships.direct_members_of(&presenter.name).to_vec(),
            groups: relationships.direct_groups_of(&presenter.name).to_vec(),
            always_grouped: relationships.is_any_always_grouped(&presenter.name),
            always_shown: relationships.is_always_shown(&presenter.name),
            sort_rank: presenter.sort_rank.clone(),
            metadata: presenter.metadata.clone(),
        }
    }

    /// Convert a slice of Presenters to FullPresenters using RelationshipManager.
    pub fn from_presenters(
        presenters: &[Presenter],
        relationships: &RelationshipManager,
    ) -> Vec<Self> {
        presenters
            .iter()
            .map(|p| Self::from_presenter(p, relationships))
            .collect()
    }
}

/// Full-format schedule for JSON export with flat relationship fields.
///
/// This struct mirrors the current Schedule serialization format but uses
/// FullPresenter with flat relationship fields instead of the enum-based
/// Presenter struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullSchedule {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<ScheduleConflict>,
    pub meta: Meta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timeline: Vec<TimelineEntry>,
    #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
    pub panel_sets: indexmap::IndexMap<String, PanelSet>,
    pub rooms: Vec<Room>,
    #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
    pub panel_types: indexmap::IndexMap<String, PanelType>,
    pub presenters: Vec<FullPresenter>,
}

impl FullSchedule {
    /// Build the RelationshipManager from the flat presenter relationship fields.
    ///
    /// This converts the v10 flat relationship fields back into the internal
    /// RelationshipManager format for in-memory operations.
    pub fn build_relationships_from_presenters(&self) -> RelationshipManager {
        use crate::data::relationship::GroupEdge;

        let mut relationships = RelationshipManager::new();

        for presenter in &self.presenters {
            let presenter_name = &presenter.name;

            // Add group edges for members (if this is a group)
            if presenter.is_group {
                // If this group should always be shown, add a group-only edge
                if presenter.always_shown {
                    relationships.add_edge(GroupEdge::group_only(presenter_name.clone(), true));
                }

                // Add member edges
                for member_name in &presenter.members {
                    relationships.add_edge(GroupEdge::new(
                        member_name.clone(),
                        presenter_name.clone(),
                        presenter.always_grouped,
                        false, // members don't set always_shown on the group edge
                    ));
                }
            }

            // Add group edges for groups (if this is an individual)
            if !presenter.is_group {
                for group_name in &presenter.groups {
                    relationships.add_edge(GroupEdge::new(
                        presenter_name.clone(),
                        group_name.clone(),
                        presenter.always_grouped,
                        false, // individual members don't set always_shown on groups
                    ));
                }
            }
        }

        relationships
    }

    /// Convert this FullSchedule back to a Schedule with proper relationships.
    pub fn to_schedule(&self) -> Result<Schedule> {
        use crate::data::presenter::Presenter;

        let relationships = self.build_relationships_from_presenters();

        // Convert FullPresenters back to Presenters with enum-based fields
        let presenters: Result<Vec<Presenter>> = self
            .presenters
            .iter()
            .map(|fp| {
                Ok(Presenter {
                    id: None, // IDs are not stored in v10 format
                    name: fp.name.clone(),
                    rank: fp.rank.clone(),
                    sort_rank: fp.sort_rank.clone(),
                    metadata: fp.metadata.clone(),
                    source: None, // Source info is not stored in v10 format
                    change_state: Default::default(), // Use default for loaded files
                })
            })
            .collect();

        let presenters = presenters?;

        Ok(Schedule {
            conflicts: self.conflicts.clone(),
            meta: self.meta.clone(),
            timeline: self.timeline.clone(),
            panel_sets: self.panel_sets.clone(),
            rooms: self.rooms.clone(),
            panel_types: self.panel_types.clone(),
            presenters,
            relationships,
            imported_sheets: crate::data::source_info::ImportedSheetPresence::default(), // Not stored in v10
        })
    }
}

impl crate::data::schedule::Schedule {
    /// Export the schedule as a full v10 JSON string with flat presenter relationship fields.
    ///
    /// This function converts the internal Presenter + RelationshipManager format
    /// to the v10 full format with flat relationship fields suitable for JSON serialization.
    /// It also handles metadata updates, schedule processing, and changeLog insertion.
    pub fn export_full_json_string(&self, history: &EditHistory) -> Result<String> {
        // Create a mutable copy for processing
        let mut schedule_clone = self.clone();

        // Apply schedule processing
        crate::data::post_process::apply_schedule_parity(&mut schedule_clone);

        // Calculate bounds and update meta for export
        let (min_time, max_time) = schedule_clone.calculate_schedule_bounds();
        if let Some(min_time) = min_time {
            schedule_clone.meta.start_time = Some(time::format_storage_ts(min_time.and_utc()));
        }
        if let Some(max_time) = max_time {
            schedule_clone.meta.end_time = Some(time::format_storage_ts(max_time.and_utc()));
        }

        // If still no times found, set reasonable defaults for Cosplay America
        if schedule_clone.meta.start_time.is_none() {
            schedule_clone.meta.start_time = Some("2026-06-25T17:00:00Z".to_string()); // Thursday evening
        }
        if schedule_clone.meta.end_time.is_none() {
            schedule_clone.meta.end_time = Some("2026-06-28T18:00:00Z".to_string()); // Sunday evening
        }

        // TODO: Build conflicts here using a resolve_panel_conflicts function
        // let conflicts = resolve_panel_conflicts(&schedule_clone.panel_sets);
        // schedule_clone.conflicts = conflicts;

        let full_presenters = FullPresenter::from_presenters(
            &schedule_clone.presenters,
            &schedule_clone.relationships,
        );

        let mut meta = schedule_clone.meta.clone();
        meta.generated = time::format_storage_ts(chrono::Utc::now());
        meta.version = Some(10);
        if meta.variant.is_none() {
            meta.variant = Some("full".to_string());
        }
        meta.generator = Some(format!("cosam-sched {}", env!("CARGO_PKG_VERSION")));

        let full_schedule = FullSchedule {
            conflicts: schedule_clone.conflicts.clone(),
            meta,
            timeline: schedule_clone.timeline.clone(),
            panel_sets: schedule_clone.panel_sets.clone(),
            rooms: schedule_clone.rooms.clone(),
            panel_types: schedule_clone.panel_types.clone(),
            presenters: full_presenters,
        };

        let json = serde_json::to_string_pretty(&full_schedule)
            .context("Failed to serialize full schedule to JSON")?;

        // Add changeLog if history is non-empty
        let final_json = if !history.is_empty() {
            let mut obj: serde_json::Value = serde_json::from_str(&json)
                .context("Failed to parse JSON for changeLog insertion")?;

            let cl = serde_json::to_value(history).context("Failed to serialize change log")?;

            if let Some(map) = obj.as_object_mut() {
                map.insert("changeLog".to_string(), cl);
            }

            serde_json::to_string_pretty(&obj).context("Failed to format JSON with changeLog")?
        } else {
            json
        };

        Ok(final_json)
    }

    /// Save the schedule as a full v10 JSON file.
    ///
    /// This method calls export_full_json_string which handles all processing
    /// including relationship sync, parity checks, bounds calculation, and metadata.
    pub fn save_json(&mut self, path: &Path, history: &EditHistory) -> Result<()> {
        let json = self.export_full_json_string(history)?;

        std::fs::write(path, json.as_bytes())
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::relationship::{GroupEdge, RelationshipManager};
    use crate::data::schedule::Schedule;
    use crate::edit::history::EditHistory;

    #[test]
    fn test_full_presenter_conversion() {
        let mut relationships = RelationshipManager::new();

        // Add some test relationships - member first, group second
        relationships.add_edge(GroupEdge::new(
            "Member1".to_string(),
            "Group1".to_string(),
            false,
            false,
        ));
        relationships.add_edge(GroupEdge::new(
            "Member2".to_string(),
            "Group1".to_string(),
            false,
            false,
        ));

        // Create test presenters
        let presenters = vec![
            Presenter {
                id: Some(1),
                name: "Group1".to_string(),
                rank: PresenterRank::Guest,
                sort_rank: Some(PresenterSortRank::people(0)),
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            Presenter {
                id: Some(2),
                name: "Member1".to_string(),
                rank: PresenterRank::FanPanelist,
                sort_rank: Some(PresenterSortRank::people(1)),
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            Presenter {
                id: Some(4),
                name: "Member2".to_string(),
                rank: PresenterRank::FanPanelist,
                sort_rank: Some(PresenterSortRank::people(2)),
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
        ];

        let full_presenters = FullPresenter::from_presenters(&presenters, &relationships);

        // Verify group conversion
        let group_fp = full_presenters.iter().find(|p| p.name == "Group1").unwrap();
        assert_eq!(group_fp.rank, PresenterRank::Guest);
        assert!(group_fp.is_group);
        assert_eq!(group_fp.members.len(), 2); // Member1 and Member2
        assert!(group_fp.members.contains(&"Member1".to_string()));
        assert!(group_fp.members.contains(&"Member2".to_string()));
        assert!(!group_fp.always_grouped);
        assert!(!group_fp.always_shown);

        // Verify member conversion
        let member_fp = full_presenters
            .iter()
            .find(|p| p.name == "Member1")
            .unwrap();
        assert_eq!(member_fp.rank, PresenterRank::FanPanelist);
        assert!(!member_fp.is_group);
        assert!(member_fp.groups.contains(&"Group1".to_string()));
        assert!(!member_fp.always_grouped);
        assert!(!member_fp.always_shown);
    }

    #[test]
    fn test_export_full_json_string() {
        let mut schedule = Schedule::default();

        // Set up basic metadata
        schedule.meta.title = "Test Schedule".to_string();
        schedule.meta.version = Some(9); // Should be updated to 10

        // Add some test presenters
        schedule.presenters.push(Presenter {
            id: Some(1),
            name: "Test Presenter".to_string(),
            rank: PresenterRank::Guest,
            sort_rank: None,
            metadata: None,
            source: None,
            change_state: Default::default(),
        });

        let history = EditHistory::new();

        // Test export
        let json_result = schedule.export_full_json_string(&history);
        assert!(json_result.is_ok(), "Export should succeed");

        let json_str = json_result.unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("Export should produce valid JSON");

        // Check version was updated to 10
        assert_eq!(parsed["meta"]["version"], 10);
        assert_eq!(parsed["meta"]["variant"], "full");
        assert_eq!(parsed["meta"]["title"], "Test Schedule");

        // Check presenters array exists and has our test presenter
        assert!(parsed["presenters"].is_array());
        let presenters = parsed["presenters"].as_array().unwrap();
        assert_eq!(presenters.len(), 1);

        let presenter = &presenters[0];
        assert_eq!(presenter["name"], "Test Presenter");
        assert_eq!(presenter["rank"], "guest");
        // isGroup should be false, but might be skipped due to skip_serializing_if
        assert!(presenter["isGroup"].is_null() || presenter["isGroup"] == false);

        // Verify no changeLog for empty history
        assert!(parsed.get("changeLog").is_none());
    }
}
