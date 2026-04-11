/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Presenter entity implementation

use crate::entity::presenter_rank::PresenterRank;
use crate::EntityFields;
use serde::{Deserialize, Serialize};

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

/// Presenter entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(Presenter)]
pub struct Presenter {
    #[field(display = "Name", description = "Presenter's full name")]
    #[alias("name", "Name", "full_name", "display_name")]
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
            entity.rank = PresenterRank::parse_rank(&s);
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
        entity.sort_rank.as_ref().map(|sort_rank| {
            crate::field::FieldValue::List(sort_rank.to_tuple(entity.rank.priority()).into_iter().map(|x| crate::field::FieldValue::Integer(x as i64)).collect())
        })
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
    #[alias("bio", "Bio", "biography", "description")]
    pub bio: Option<String>,

    #[field(
        display = "Is Group",
        description = "Whether this presenter is a group"
    )]
    #[alias("is_group", "Is_Group", "group", "presenter_group")]
    pub is_group: bool,

    #[field(
        display = "Always Grouped",
        description = "Whether this presenter should always appear with their group"
    )]
    #[alias("always_grouped", "Always_Grouped", "stick_with_group")]
    pub always_grouped: bool,

    #[field(
        display = "Always Shown in Group",
        description = "Whether this presenter's group should always be shown as a group"
    )]
    #[alias("always_shown", "Always_Shown_In_Group", "show_as_group")]
    pub always_shown_in_group: bool,

    #[computed_field(
        display = "Groups",
        description = "All groups this presenter belongs to"
    )]
    #[alias("presenter_groups", "group_list")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::{InternalData, PresenterToGroupEntityType};
        let ids = PresenterToGroupEntityType::groups_of(&schedule.entities, entity.uuid());
        if ids.is_empty() {
            None
        } else {
            Some(crate::field::FieldValue::List(
                ids.into_iter()
                    .map(|id| crate::field::FieldValue::NonNilUuid(id.non_nil_uuid()))
                    .collect(),
            ))
        }
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::{InternalData, PresenterToGroupEntityType, PresenterToGroupId};
        use crate::schedule::TypedEdgeStorage;
        let member_uuid = entity.uuid();
        let old_edge_uuids: Vec<uuid::NonNilUuid> = PresenterToGroupEntityType::edge_index(&schedule.entities)
            .outgoing(member_uuid)
            .iter()
            .copied()
            .collect();
        for edge_uuid in old_edge_uuids {
            if let Some(data) = schedule.get_entity_by_uuid::<PresenterToGroupEntityType>(edge_uuid) {
                if !data.is_self_loop() {
                    schedule.remove_edge::<PresenterToGroupEntityType>(PresenterToGroupId::from_uuid(edge_uuid));
                }
            }
        }
        let new_group_uuids: Vec<uuid::NonNilUuid> = match value {
            crate::field::FieldValue::List(items) => items
                .into_iter()
                .filter_map(|v| if let crate::field::FieldValue::NonNilUuid(u) = v { Some(u) } else { None })
                .collect(),
            crate::field::FieldValue::NonNilUuid(u) => vec![u],
            _ => return Err(crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            )),
        };
        for group_uuid in new_group_uuids {
            let edge = crate::entity::PresenterToGroupData {
                entity_uuid: unsafe { uuid::NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) },
                member_uuid,
                group_uuid,
                always_shown_in_group: false,
                always_grouped: false,
            };
            schedule.add_edge::<PresenterToGroupEntityType>(edge)
                .map_err(|_| crate::field::FieldError::ConversionError(
                    crate::field::validation::ConversionError::InvalidFormat,
                ))?;
        }
        Ok(())
    })]
    pub groups: Vec<PresenterId>,

    #[computed_field(
        display = "Members",
        description = "All members of this presenter (if this presenter is a group)"
    )]
    #[alias("presenter_members", "member_list")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::{InternalData, PresenterToGroupEntityType};
        let ids = PresenterToGroupEntityType::members_of(&schedule.entities, entity.uuid());
        if ids.is_empty() {
            None
        } else {
            Some(crate::field::FieldValue::List(
                ids.into_iter()
                    .map(|id| crate::field::FieldValue::NonNilUuid(id.non_nil_uuid()))
                    .collect(),
            ))
        }
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::{InternalData, PresenterToGroupEntityType, PresenterToGroupId};
        use crate::schedule::TypedEdgeStorage;
        let group_uuid = entity.uuid();
        let old_edge_uuids: Vec<uuid::NonNilUuid> = PresenterToGroupEntityType::edge_index(&schedule.entities)
            .incoming(group_uuid)
            .iter()
            .copied()
            .collect();
        for edge_uuid in old_edge_uuids {
            if let Some(data) = schedule.get_entity_by_uuid::<PresenterToGroupEntityType>(edge_uuid) {
                if !data.is_self_loop() {
                    schedule.remove_edge::<PresenterToGroupEntityType>(PresenterToGroupId::from_uuid(edge_uuid));
                }
            }
        }
        let new_member_uuids: Vec<uuid::NonNilUuid> = match value {
            crate::field::FieldValue::List(items) => items
                .into_iter()
                .filter_map(|v| if let crate::field::FieldValue::NonNilUuid(u) = v { Some(u) } else { None })
                .collect(),
            crate::field::FieldValue::NonNilUuid(u) => vec![u],
            _ => return Err(crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            )),
        };
        for member_uuid in new_member_uuids {
            let edge = crate::entity::PresenterToGroupData {
                entity_uuid: unsafe { uuid::NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) },
                member_uuid,
                group_uuid,
                always_shown_in_group: false,
                always_grouped: false,
            };
            schedule.add_edge::<PresenterToGroupEntityType>(edge)
                .map_err(|_| crate::field::FieldError::ConversionError(
                    crate::field::validation::ConversionError::InvalidFormat,
                ))?;
        }
        Ok(())
    })]
    pub members: Vec<PresenterId>,

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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::{NonNilUuid, Uuid};

    fn test_nn() -> NonNilUuid {
        unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        }
    }

    #[test]
    fn presenter_id_from_uuid() {
        let nn = test_nn();
        let id = PresenterId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn presenter_id_try_from_nil_uuid_returns_none() {
        assert!(PresenterId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn presenter_id_display() {
        let id = PresenterId::from(test_nn());
        assert_eq!(
            id.to_string(),
            "presenter-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn presenter_id_serde_round_trip() {
        let id = PresenterId::from(test_nn());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"00000000-0000-0000-0000-000000000001\"");
        let back: PresenterId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
