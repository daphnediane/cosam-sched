/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use serde::{Deserialize, Serialize};

use crate::data::panel::ExtraFields;
use crate::data::presenter::{Presenter, PresenterRank, PresenterSortRank};
use crate::data::relationship::RelationshipManager;
use crate::data::schedule::Meta;
use crate::data::source_info::{ChangeState, SourceInfo};

/// Full-format presenter for JSON export with flat relationship fields.
///
/// This struct mirrors the current Presenter serialization format but uses
/// flat fields instead of enum-based PresenterMember/PresenterGroup.
/// It queries the RelationshipManager for group membership data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullPresenter {
    pub id: Option<u32>,
    pub name: String,
    pub rank: PresenterRank,
    /// Flat field indicating if this presenter is a group
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_group: bool,
    /// Flat field listing direct members (only populated for groups)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    /// Flat field listing direct groups this presenter belongs to
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,
    /// Flat field indicating if this presenter should always be grouped with its groups
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub always_grouped: bool,
    /// Flat field indicating if this group should always be shown as a group
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub always_shown: bool,
    /// Ordering key recording where this presenter was first defined
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_rank: Option<PresenterSortRank>,
    /// Additional metadata fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ExtraFields>,
    /// Source information for tracking data origins
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceInfo>,
    /// Change tracking state
    pub change_state: ChangeState,
}

impl FullPresenter {
    /// Create a FullPresenter from a Presenter and RelationshipManager.
    ///
    /// This converts from the internal enum-based relationship storage
    /// to flat fields suitable for JSON serialization.
    pub fn from_presenter(presenter: &Presenter, relationships: &RelationshipManager) -> Self {
        Self {
            id: presenter.id,
            name: presenter.name.clone(),
            rank: presenter.rank.clone(),
            is_group: relationships.is_group(&presenter.name),
            members: relationships.direct_members_of(&presenter.name).to_vec(),
            groups: relationships.direct_groups_of(&presenter.name).to_vec(),
            always_grouped: relationships.is_any_always_grouped(&presenter.name),
            always_shown: relationships.is_always_shown(&presenter.name),
            sort_rank: presenter.sort_rank.clone(),
            metadata: presenter.metadata.clone(),
            source: presenter.source.clone(),
            change_state: presenter.change_state.clone(),
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
    pub meta: Meta,
    pub presenters: Vec<FullPresenter>,
    // Note: In a full implementation, this would also include panels, rooms,
    // panel_types, timeline, etc. For now we're focusing on the presenter
    // conversion for phase 8 groundwork.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::relationship::{GroupEdge, RelationshipManager};

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
                is_member: crate::data::presenter::PresenterMember::NotMember,
                is_grouped: crate::data::presenter::PresenterGroup::NotGroup,
                sort_rank: Some(PresenterSortRank::people(0)),
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            Presenter {
                id: Some(2),
                name: "Member1".to_string(),
                rank: PresenterRank::FanPanelist,
                is_member: crate::data::presenter::PresenterMember::NotMember,
                is_grouped: crate::data::presenter::PresenterGroup::NotGroup,
                sort_rank: Some(PresenterSortRank::people(1)),
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            Presenter {
                id: Some(4),
                name: "Member2".to_string(),
                rank: PresenterRank::FanPanelist,
                is_member: crate::data::presenter::PresenterMember::NotMember,
                is_grouped: crate::data::presenter::PresenterGroup::NotGroup,
                sort_rank: Some(PresenterSortRank::people(2)),
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
        ];

        let full_presenters = FullPresenter::from_presenters(&presenters, &relationships);

        // Verify group conversion
        let group_fp = full_presenters.iter().find(|p| p.name == "Group1").unwrap();
        assert_eq!(group_fp.id, Some(1));
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
        assert_eq!(member_fp.id, Some(2));
        assert_eq!(member_fp.rank, PresenterRank::FanPanelist);
        assert!(!member_fp.is_group);
        assert!(member_fp.groups.contains(&"Group1".to_string()));
        assert!(!member_fp.always_grouped);
        assert!(!member_fp.always_shown);
    }
}
