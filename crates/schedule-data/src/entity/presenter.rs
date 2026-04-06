/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Presenter entity implementation

use crate::entity::presenter_rank::PresenterRank;
use crate::EntityFields;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Ordering key for a presenter, recording where it was first defined.
/// Matches schedule-core PresenterSortRank structure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PresenterSortRank {
    pub column_index: u32,
    pub row_index: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub member_index: u32,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

impl PresenterSortRank {
    pub fn new(column_index: u32, row_index: u32, member_index: u32) -> Self {
        Self {
            column_index,
            row_index,
            member_index,
        }
    }

    /// People table rank: column 0, given row, member_index 0.
    pub fn people(row_index: u32) -> Self {
        Self::new(0, row_index, 0)
    }

    /// Schedule column rank for a group entry.
    pub fn schedule_group(column_index: u32, row_index: u32) -> Self {
        Self::new(column_index, row_index, 0)
    }

    /// Schedule column rank for an individual member entry.
    pub fn schedule_member(column_index: u32, row_index: u32) -> Self {
        Self::new(column_index, row_index, 1)
    }

    /// Convert to tuple format for computed field: [rank_priority, column, row, member]
    pub fn to_tuple(&self, rank_priority: u8) -> Vec<u64> {
        vec![
            rank_priority as u64,
            self.column_index as u64,
            self.row_index as u64,
            self.member_index as u64,
        ]
    }

    /// Convert from tuple format
    pub fn from_tuple(values: &[u64]) -> Option<Self> {
        if values.len() >= 4 {
            Some(Self::new(
                values[1] as u32, // column_index
                values[2] as u32, // row_index
                values[3] as u32, // member_index
            ))
        } else {
            None
        }
    }
}

/// Presenter ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresenterId(u64);

impl fmt::Display for PresenterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "presenter-{}", self.0)
    }
}

/// Presenter entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
pub struct Presenter {
    #[field(display = "Name", description = "Presenter's full name")]
    #[alias("name", "full_name", "display_name")]
    #[indexable(priority = 200)]
    #[required]
    pub name: String,

    /// Internal rank field - not exposed to users directly

    #[computed_field(
        display = "Classification",
        description = "Presenter's classification or rank"
    )]
    #[alias("classification", "rank", "class", "presenter_rank")]
    #[read(|entity: &PresenterData| {
        Some(crate::field::FieldValue::String(entity.rank.to_string()))
    })]
    #[write(|entity: &mut PresenterData, value: crate::field::FieldValue| {
        if let crate::field::FieldValue::String(s) = value {
            entity.rank = PresenterRank::from_str(&s);
            Ok(())
        } else {
            Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
        }
    })]
    pub rank: PresenterRank,

    #[computed_field(
        display = "Index Rank",
        description = "Presenter ordering information [rank_priority, column, row, member]"
    )]
    #[alias("index_rank", "sort_rank")]
    #[read(|entity: &PresenterData| {
        if let Some(sort_rank) = &entity.sort_rank {
            Some(crate::field::FieldValue::List(sort_rank.to_tuple(entity.rank.priority()).into_iter().map(|x| crate::field::FieldValue::Integer(x as i64)).collect()))
        } else {
            None
        }
    })]
    #[write(|entity: &mut PresenterData, value: crate::field::FieldValue| {
        if let crate::field::FieldValue::List(values) = value {
            let int_values: Vec<u64> = values.iter().filter_map(|x| {
                if let crate::field::FieldValue::Integer(i) = x {
                    Some(*i as u64)
                } else {
                    None
                }
            }).collect();
            entity.sort_rank = PresenterSortRank::from_tuple(&int_values);
            Ok(())
        } else {
            Err(crate::field::FieldError::ConversionError(crate::field::validation::ConversionError::InvalidFormat))
        }
    })]
    pub sort_rank: Option<PresenterSortRank>,

    #[field(display = "Bio", description = "Presenter's biography")]
    #[alias("bio", "biography", "description")]
    pub bio: Option<String>,

    // Group-related fields for presenter-group relationships
    #[field(display = "UID", description = "Unique identifier for the presenter")]
    #[alias("uid", "id")]
    #[required]
    #[indexable(priority = 210)]
    pub uid: String,

    #[field(
        display = "Is Group",
        description = "Whether this presenter is a group"
    )]
    #[alias("is_group", "group", "presenter_group")]
    pub is_group: bool,

    #[field(
        display = "Always Grouped",
        description = "Whether this presenter should always appear with their group"
    )]
    #[alias("always_grouped", "stick_with_group")]
    pub always_grouped: bool,

    #[field(
        display = "Always Shown in Group",
        description = "Whether this presenter's group should always be shown as a group"
    )]
    #[alias("always_shown", "show_as_group")]
    pub always_shown_in_group: bool,

    #[computed_field(
        display = "Groups",
        description = "All groups this presenter belongs to"
    )]
    #[alias("presenter_groups", "group_list")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        let group_ids = schedule.get_presenter_groups(entity.entity_id);
        Some(crate::field::FieldValue::List(
            schedule.get_entity_names::<crate::entity::PresenterEntityType>(&group_ids)
                .into_iter()
                .map(crate::field::FieldValue::String)
                .collect()
        ))
    })]
    pub groups: Vec<crate::entity::EntityId>,

    #[computed_field(
        display = "Members",
        description = "All members of this presenter (if this presenter is a group)"
    )]
    #[alias("presenter_members", "member_list")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        let member_ids = schedule.get_presenter_members(entity.entity_id);
        Some(crate::field::FieldValue::List(
            schedule.get_entity_names::<crate::entity::PresenterEntityType>(&member_ids)
                .into_iter()
                .map(crate::field::FieldValue::String)
                .collect()
        ))
    })]
    pub members: Vec<crate::entity::EntityId>,

    #[computed_field(
        display = "Panels",
        description = "All panels this presenter participates in"
    )]
    #[alias("presenter_panels", "panel_list")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        let panel_ids = schedule.get_presenter_panels(entity.entity_id);
        Some(crate::field::FieldValue::List(
            schedule.get_entity_names::<crate::entity::PanelEntityType>(&panel_ids)
                .into_iter()
                .map(crate::field::FieldValue::String)
                .collect()
        ))
    })]
    pub panels: Vec<crate::entity::EntityId>,

    // @TODO: Not currently in the spreadsheets, Windsurf thought this was a good idea
    // I agree but we currently don't have the data
    #[field(display = "Pronouns", description = "Presenter's preferred pronouns")]
    #[alias("pronouns", "preferred_pronouns")]
    pub pronouns: Option<String>,

    // @TODO: Not currently in the spreadsheets, Windsurf thought this was a good idea
    // I agree but we currently don't have the data
    #[field(display = "Website", description = "Presenter's website")]
    #[alias("website", "url", "web", "site")]
    pub website: Option<String>,
}
