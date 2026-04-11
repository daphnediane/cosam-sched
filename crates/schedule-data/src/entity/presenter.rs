/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Presenter entity implementation

use crate::entity::presenter_rank::PresenterRank;
use crate::schedule::LookupError;
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
        display = "Is Explicit Group",
        description = "True when this presenter was explicitly declared as a group"
    )]
    #[alias("is_explicit_group")]
    pub is_explicit_group: bool,

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

    /// Backing storage for group membership (owned forward side).
    /// Updated by the `groups` computed field write closure and membership helpers.
    pub group_ids: Vec<PresenterId>,

    #[computed_field(
        display = "Groups",
        description = "All groups this presenter belongs to"
    )]
    #[alias("presenter_groups", "group_list")]
    #[read(|_schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        if entity.group_ids.is_empty() {
            None
        } else {
            Some(crate::field::FieldValue::List(
                entity.group_ids.iter()
                    .map(|id| crate::field::FieldValue::NonNilUuid(id.non_nil_uuid()))
                    .collect(),
            ))
        }
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::{InternalData, PresenterToGroupEntityType};
        let member_uuid = entity.uuid();
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
        entity.group_ids = new_group_uuids
            .iter()
            .map(|&u| PresenterId::from_uuid(u))
            .collect();
        PresenterToGroupEntityType::set_groups(&mut schedule.entities, member_uuid, &new_group_uuids)
            .map_err(|_| crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            ))
    })]
    pub groups: Vec<PresenterId>,

    /// Whether this presenter is a group (explicit or implicit).
    ///
    /// Read: true if `is_explicit_group` or if this presenter has any members.
    /// Write true: sets `is_explicit_group = true`.
    /// Write false: clears `is_explicit_group` AND removes all members so the
    ///   field stays coherent (has_members would otherwise keep the read as true).
    #[computed_field(
        display = "Is Group",
        description = "Whether this presenter is a group (explicit or implicit)"
    )]
    #[alias("is_group", "Is_Group", "group", "presenter_group")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::{InternalData, PresenterToGroupEntityType};
        let is_grp = entity.is_explicit_group
            || !PresenterToGroupEntityType::members_of(&schedule.entities, entity.uuid()).is_empty();
        Some(crate::field::FieldValue::Boolean(is_grp))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::{InternalData, PresenterEntityType};
        let flag = match value {
            crate::field::FieldValue::Boolean(b) => b,
            crate::field::FieldValue::String(ref s) => !matches!(s.to_lowercase().as_str(), "false" | "0" | ""),
            _ => return Err(crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            )),
        };
        entity.is_explicit_group = flag;
        if !flag {
            PresenterEntityType::clear_member_edges(&mut schedule.entities, entity.uuid());
        }
        Ok(())
    })]
    pub is_group: bool,

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
        use crate::entity::{InternalData, PresenterId, PresenterToGroupEntityType};
        let group_uuid = entity.uuid();
        let group_id = PresenterId::from_uuid(group_uuid);
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
        let old_member_uuids: Vec<uuid::NonNilUuid> =
            PresenterToGroupEntityType::members_of(&schedule.entities, group_uuid)
                .iter()
                .map(|id| id.non_nil_uuid())
                .collect();
        for &old_uuid in &old_member_uuids {
            if !new_member_uuids.contains(&old_uuid) {
                if let Some(member_data) = schedule.entities.presenters.get_mut(&old_uuid) {
                    member_data.group_ids.retain(|id| id.non_nil_uuid() != group_uuid);
                }
            }
        }
        for &new_uuid in &new_member_uuids {
            if !old_member_uuids.contains(&new_uuid) {
                if let Some(member_data) = schedule.entities.presenters.get_mut(&new_uuid) {
                    if !member_data.group_ids.contains(&group_id) {
                        member_data.group_ids.push(group_id);
                    }
                }
            }
        }
        PresenterToGroupEntityType::set_members(&mut schedule.entities, group_uuid, &new_member_uuids)
            .map_err(|_| crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            ))
    })]
    pub members: Vec<PresenterId>,

    /// All panels this presenter is directly assigned to (via PanelToPresenter edges).
    #[computed_field(
        display = "Panels",
        description = "All panels this presenter is directly assigned to"
    )]
    #[alias("panel")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::{InternalData, PanelToPresenterEntityType};
        let ids = PanelToPresenterEntityType::panels_of(&schedule.entities, entity.uuid());
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
        use crate::entity::{InternalData, PanelToPresenterEntityType, PresenterId};
        let presenter_uuid = entity.uuid();
        let presenter_id = PresenterId::from_uuid(presenter_uuid);
        let new_panel_uuids: Vec<uuid::NonNilUuid> = match value {
            crate::field::FieldValue::List(items) => items
                .into_iter()
                .filter_map(|v| if let crate::field::FieldValue::NonNilUuid(u) = v { Some(u) } else { None })
                .collect(),
            crate::field::FieldValue::NonNilUuid(u) => vec![u],
            _ => return Err(crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            )),
        };
        let old_panel_uuids: Vec<uuid::NonNilUuid> =
            PanelToPresenterEntityType::panels_of(&schedule.entities, presenter_uuid)
                .iter()
                .map(|id| id.non_nil_uuid())
                .collect();
        for &old_uuid in &old_panel_uuids {
            if !new_panel_uuids.contains(&old_uuid) {
                if let Some(panel_data) = schedule.entities.panels.get_mut(&old_uuid) {
                    panel_data.presenter_ids.retain(|id| id.non_nil_uuid() != presenter_uuid);
                }
            }
        }
        for &new_uuid in &new_panel_uuids {
            if !old_panel_uuids.contains(&new_uuid) {
                if let Some(panel_data) = schedule.entities.panels.get_mut(&new_uuid) {
                    if !panel_data.presenter_ids.contains(&presenter_id) {
                        panel_data.presenter_ids.push(presenter_id);
                    }
                }
            }
        }
        PanelToPresenterEntityType::set_panels_for_presenter(&mut schedule.entities, presenter_uuid, &new_panel_uuids)
            .map_err(|_| crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            ))
    })]
    pub panels: Vec<crate::entity::PanelId>,

    /// Add panels to this presenter without replacing existing ones.
    /// Write-only computed field that accepts a single UUID or list of UUIDs.
    #[computed_field(
        display = "Add Panels",
        description = "Add panels to this presenter (append mode)"
    )]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::{InternalData, PanelEntityType, PanelToPresenterEntityType, PresenterId};
        let presenter_uuid = entity.uuid();
        let presenter_id = PresenterId::from_uuid(presenter_uuid);
        let values: Vec<crate::field::FieldValue> = match value {
            crate::field::FieldValue::List(items) => items,
            single => vec![single],
        };
        for value in values {
            if let Ok(panel_id) = PanelEntityType::resolve_field_value(&schedule.entities, value) {
                let panel_uuid = panel_id.non_nil_uuid();
                let already = PanelToPresenterEntityType::panels_of(&schedule.entities, presenter_uuid)
                    .iter()
                    .any(|id| id.non_nil_uuid() == panel_uuid);
                if !already {
                    if let Some(panel_data) = schedule.entities.panels.get_mut(&panel_uuid) {
                        if !panel_data.presenter_ids.contains(&presenter_id) {
                            panel_data.presenter_ids.push(presenter_id);
                        }
                    }
                    let edge = crate::entity::PanelToPresenterData {
                        entity_uuid: unsafe { uuid::NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) },
                        panel_uuid,
                        presenter_uuid,
                    };
                    let _ = schedule.entities.add_edge::<PanelToPresenterEntityType>(edge);
                }
            }
        }
        Ok(())
    })]
    pub add_panels: Vec<crate::entity::PanelId>,

    /// Remove panels from this presenter.
    /// Write-only computed field that accepts a single UUID or list of UUIDs.
    #[computed_field(
        display = "Remove Panels",
        description = "Remove panels from this presenter"
    )]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::{InternalData, PanelToPresenterEntityType};
        let presenter_uuid = entity.uuid();
        let panel_uuids: Vec<uuid::NonNilUuid> = match value {
            crate::field::FieldValue::List(items) => items
                .into_iter()
                .filter_map(|v| if let crate::field::FieldValue::NonNilUuid(u) = v { Some(u) } else { None })
                .collect(),
            crate::field::FieldValue::NonNilUuid(u) => vec![u],
            _ => return Err(crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            )),
        };
        for &panel_uuid in &panel_uuids {
            if let Some(panel_data) = schedule.entities.panels.get_mut(&panel_uuid) {
                panel_data.presenter_ids.retain(|id| id.non_nil_uuid() != presenter_uuid);
            }
        }
        PanelToPresenterEntityType::remove_panels_for_presenter(&mut schedule.entities, presenter_uuid, &panel_uuids);
        Ok(())
    })]
    pub remove_panels: Vec<crate::entity::PanelId>,

    /// Transitive closure: all panels this presenter is on, directly or via group membership.
    #[computed_field(
        display = "Inclusive Panels",
        description = "All panels this presenter is on, directly or via group membership"
    )]
    #[alias("inclusive_panel")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::{InternalData, PanelToPresenterEntityType, PresenterToGroupEntityType};
        use std::collections::HashSet;

        let presenter_uuid = entity.uuid();
        let direct = PanelToPresenterEntityType::panels_of(&schedule.entities, presenter_uuid);

        let mut result = Vec::new();
        let mut seen = HashSet::new();

        // Add direct panels
        for panel_id in &direct {
            let panel_uuid = panel_id.non_nil_uuid();
            if seen.insert(panel_uuid) {
                result.push(crate::field::FieldValue::NonNilUuid(panel_uuid));
            }
        }

        // Add panels for all inclusive groups (upward)
        for group_id in PresenterToGroupEntityType::inclusive_groups_of(&schedule.entities, presenter_uuid) {
            let group_uuid = group_id.non_nil_uuid();
            for panel_id in PanelToPresenterEntityType::panels_of(&schedule.entities, group_uuid) {
                let panel_uuid = panel_id.non_nil_uuid();
                if seen.insert(panel_uuid) {
                    result.push(crate::field::FieldValue::NonNilUuid(panel_uuid));
                }
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(crate::field::FieldValue::List(result))
        }
    })]
    pub inclusive_panels: Vec<crate::entity::PanelId>,

    /// Transitive closure: all groups this presenter belongs to (upward).
    #[computed_field(
        display = "Inclusive Groups",
        description = "All groups this presenter belongs to, directly or transitively"
    )]
    #[alias("inclusive_group")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::{InternalData, PresenterToGroupEntityType};
        let ids = PresenterToGroupEntityType::inclusive_groups_of(&schedule.entities, entity.uuid());
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
    pub inclusive_groups: Vec<PresenterId>,

    /// Transitive closure: all members if this presenter is a group (downward).
    #[computed_field(
        display = "Inclusive Members",
        description = "All members of this presenter if it is a group, directly or transitively"
    )]
    #[alias("inclusive_member")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::{InternalData, PresenterToGroupEntityType};
        let ids = PresenterToGroupEntityType::inclusive_members_of(&schedule.entities, entity.uuid());
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
    pub inclusive_members: Vec<PresenterId>,

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

impl PresenterEntityType {
    // -----------------------------------------------------------------------
    // Group status helpers
    // -----------------------------------------------------------------------

    /// Whether a presenter is a group (explicit or implicit).
    ///
    /// Returns `true` when `is_explicit_group` is set, or when at least one other
    /// presenter lists this one in their `groups` (i.e., it has members via the
    /// edge map, which is kept in sync until Phase 4).
    pub fn is_group(
        storage: &crate::schedule::EntityStorage,
        presenter_uuid: uuid::NonNilUuid,
    ) -> bool {
        use crate::entity::PresenterToGroupEntityType;
        storage
            .presenters
            .get(&presenter_uuid)
            .is_some_and(|d| d.is_explicit_group)
            || !PresenterToGroupEntityType::members_of(storage, presenter_uuid).is_empty()
    }

    /// Set `is_explicit_group` on a presenter entity stored in `EntityStorage`.
    ///
    /// No-op if the presenter UUID is unknown.
    pub fn set_explicit_group(
        storage: &mut crate::schedule::EntityStorage,
        presenter_uuid: uuid::NonNilUuid,
        value: bool,
    ) {
        if let Some(data) = storage.presenters.get_mut(&presenter_uuid) {
            data.is_explicit_group = value;
        }
    }

    /// Remove all membership edges from `group_uuid` and clear the matching
    /// entry from each member's `group_ids` backing field.
    ///
    /// This does **not** touch `group_uuid`'s own `is_explicit_group`; callers
    /// are responsible for clearing that separately (needed because the
    /// entity may be temporarily extracted from storage during field writes).
    pub fn clear_member_edges(
        storage: &mut crate::schedule::EntityStorage,
        group_uuid: uuid::NonNilUuid,
    ) {
        use crate::entity::PresenterToGroupEntityType;
        let member_uuids: Vec<uuid::NonNilUuid> =
            PresenterToGroupEntityType::members_of(storage, group_uuid)
                .into_iter()
                .map(|id| id.non_nil_uuid())
                .collect();
        for member_uuid in member_uuids {
            if let Some(data) = storage.presenters.get_mut(&member_uuid) {
                data.group_ids.retain(|id| id.non_nil_uuid() != group_uuid);
            }
        }
        let _ = PresenterToGroupEntityType::set_members(storage, group_uuid, &[]);
    }

    // -----------------------------------------------------------------------
    // Tag-string lookup / find-or-create
    // -----------------------------------------------------------------------

    /// Look up a presenter by a tagged credit string, or find-or-create one.
    ///
    /// See [`crate::schedule::Schedule::lookup_tagged_presenter`] for the full
    /// format documentation. This associated function owns the implementation;
    /// `Schedule::lookup_tagged_presenter` delegates here.
    #[must_use = "returns the presenter/group ID; check for errors"]
    pub fn lookup_tagged(
        storage: &mut crate::schedule::EntityStorage,
        input: &str,
    ) -> Result<PresenterId, LookupError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(LookupError::Empty);
        }

        // --- UUID forms: "presenter-<uuid>" or bare UUID string ---------------
        let uuid_str = if let Some(rest) = input.strip_prefix("presenter-") {
            Some(rest)
        } else if Self::looks_like_uuid(input) {
            Some(input)
        } else {
            None
        };

        if let Some(uuid_str) = uuid_str {
            let raw = uuid_str
                .parse::<uuid::Uuid>()
                .map_err(|_| LookupError::InvalidUuid(uuid_str.to_string()))?;
            let nn = uuid::NonNilUuid::new(raw)
                .ok_or_else(|| LookupError::InvalidUuid(uuid_str.to_string()))?;
            if storage.presenters.contains_key(&nn) {
                return Ok(PresenterId::from_uuid(nn));
            }
            return Err(LookupError::UuidNotFound(raw));
        }

        // --- Tag prefix: one or more rank chars followed by ':' ---------------
        if let Some((rank, rest)) = Self::parse_tag_flags(input) {
            return Self::process_tagged(storage, &rest, rank);
        }

        // --- Bare name: exact case-insensitive lookup, no auto-create ---------
        if let Some((&uuid, _)) = storage
            .presenters
            .iter()
            .find(|(_, d)| d.name.eq_ignore_ascii_case(input))
        {
            return Ok(PresenterId::from_uuid(uuid));
        }
        Err(LookupError::NameNotFound(input.to_string()))
    }

    /// Find a presenter by exact case-insensitive name, or create a new one
    /// with the given rank and a fresh UUID.
    pub fn find_or_create_by_name(
        storage: &mut crate::schedule::EntityStorage,
        name: &str,
        rank: PresenterRank,
    ) -> PresenterId {
        use uuid::NonNilUuid;
        if let Some((&uuid, _)) = storage
            .presenters
            .iter()
            .find(|(_, d)| d.name.eq_ignore_ascii_case(name))
        {
            return PresenterId::from_uuid(uuid);
        }
        let uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
        let data = PresenterData {
            entity_uuid: uuid,
            name: name.to_string(),
            rank,
            sort_rank: None,
            bio: None,
            is_explicit_group: false,
            always_grouped: false,
            always_shown_in_group: false,
            pronouns: None,
            website: None,
            group_ids: Default::default(),
            groups: Default::default(),
            is_group: Default::default(),
            members: Default::default(),
            panels: Default::default(),
            add_panels: Default::default(),
            remove_panels: Default::default(),
            inclusive_panels: Default::default(),
            inclusive_groups: Default::default(),
            inclusive_members: Default::default(),
        };
        let _ = storage.add_entity::<PresenterEntityType>(data);
        PresenterId::from_uuid(uuid)
    }

    /// Returns `true` if `s` looks like a raw UUID (8-4-4-4-12 hex groups).
    fn looks_like_uuid(s: &str) -> bool {
        s.len() == 36
            && s.as_bytes().get(8) == Some(&b'-')
            && s.as_bytes().get(13) == Some(&b'-')
            && s.as_bytes().get(18) == Some(&b'-')
            && s.as_bytes().get(23) == Some(&b'-')
    }

    /// Parse a flag prefix: one or more rank characters followed by `:`.
    /// Returns the highest-priority rank and the remainder of the string.
    fn parse_tag_flags(input: &str) -> Option<(PresenterRank, String)> {
        let colon_pos = input.find(':')?;
        let flag_str = &input[..colon_pos];
        if flag_str.is_empty() {
            return None;
        }
        let mut best: Option<PresenterRank> = None;
        for c in flag_str.chars() {
            let rank = PresenterRank::from_prefix_char(c)?;
            best = Some(match best {
                None => rank,
                Some(b) if rank.priority() < b.priority() => rank,
                Some(b) => b,
            });
        }
        let rest = input[colon_pos + 1..].trim().to_string();
        Some((best?, rest))
    }

    /// Process the portion after the tag prefix: handles `<`, `=`, `==`
    /// syntax and finds-or-creates the presenter and optional group.
    fn process_tagged(
        storage: &mut crate::schedule::EntityStorage,
        rest: &str,
        rank: PresenterRank,
    ) -> Result<PresenterId, LookupError> {
        use crate::entity::PresenterToGroupEntityType;
        let rest = rest.trim();
        if rest.is_empty() {
            return Err(LookupError::Empty);
        }
        if rest.eq_ignore_ascii_case("other") {
            return Err(LookupError::OtherSentinel);
        }

        let (name_raw, group_raw) = if let Some(eq_pos) = rest.find('=') {
            let name_part = rest[..eq_pos].trim().to_string();
            let group_part = rest[eq_pos + 1..].trim().to_string();
            (
                name_part,
                if group_part.is_empty() {
                    None
                } else {
                    Some(group_part)
                },
            )
        } else {
            (rest.to_string(), None)
        };

        let (presenter_name, always_grouped) = if let Some(stripped) = name_raw.strip_prefix('<') {
            (stripped.trim().to_string(), true)
        } else {
            (name_raw, false)
        };

        let (group_name, always_shown) = match group_raw {
            Some(g) => {
                if let Some(stripped) = g.strip_prefix('=') {
                    let gn = stripped.trim().to_string();
                    (if gn.is_empty() { None } else { Some(gn) }, true)
                } else {
                    (Some(g), false)
                }
            }
            None => (None, false),
        };

        let group_id: Option<PresenterId> = if let Some(ref gname) = group_name {
            let gid = Self::find_or_create_by_name(storage, gname, rank.clone());
            let gid_uuid = gid.non_nil_uuid();
            let _ = PresenterToGroupEntityType::mark_group(storage, gid_uuid);
            Self::set_explicit_group(storage, gid_uuid, true);
            if always_shown {
                PresenterToGroupEntityType::set_group_marker_shown(storage, gid_uuid, true);
                if let Some(gdata) = storage.presenters.get_mut(&gid_uuid) {
                    gdata.always_shown_in_group = true;
                }
            }
            Some(gid)
        } else {
            None
        };

        let effective = if presenter_name.is_empty()
            || group_name
                .as_deref()
                .is_some_and(|g| g.eq_ignore_ascii_case(&presenter_name))
        {
            group_id.ok_or(LookupError::Empty)?
        } else {
            let pid = Self::find_or_create_by_name(storage, &presenter_name, rank);
            if let Some(gid) = group_id {
                let pid_uuid = pid.non_nil_uuid();
                let gid_uuid = gid.non_nil_uuid();
                if always_grouped {
                    let _ =
                        PresenterToGroupEntityType::add_grouped_member(storage, pid_uuid, gid_uuid);
                    if let Some(pdata) = storage.presenters.get_mut(&pid_uuid) {
                        pdata.always_grouped = true;
                        if !pdata.group_ids.contains(&gid) {
                            pdata.group_ids.push(gid);
                        }
                    }
                } else {
                    let _ = PresenterToGroupEntityType::add_member(storage, pid_uuid, gid_uuid);
                    if let Some(pdata) = storage.presenters.get_mut(&pid_uuid) {
                        if !pdata.group_ids.contains(&gid) {
                            pdata.group_ids.push(gid);
                        }
                    }
                }
            }
            pid
        };

        Ok(effective)
    }

    /// Resolve a FieldValue to a PresenterId.
    ///
    /// Supports:
    /// - `FieldValue::NonNilUuid(u)` -> lookup by UUID
    /// - `FieldValue::String(s)` -> treat as tagged string (e.g., "G:Alice", "presenter-<uuid>")
    /// - `FieldValue::OptionalString(Some(s))` -> same as String
    pub fn resolve_field_value(
        storage: &mut crate::schedule::EntityStorage,
        value: crate::field::FieldValue,
    ) -> Result<PresenterId, crate::schedule::LookupError> {
        match value {
            crate::field::FieldValue::NonNilUuid(uuid) => {
                if storage.presenters.contains_key(&uuid) {
                    Ok(PresenterId::from_uuid(uuid))
                } else {
                    Err(crate::schedule::LookupError::UuidNotFound(uuid.into()))
                }
            }
            crate::field::FieldValue::String(s) => Self::lookup_tagged(storage, &s),
            crate::field::FieldValue::OptionalString(Some(s)) => Self::lookup_tagged(storage, &s),
            _ => Err(crate::schedule::LookupError::Empty),
        }
    }

    /// Resolve a list of FieldValues to PresenterIds.
    ///
    /// Returns Ok with the list of resolved IDs, or Err if any resolution fails.
    pub fn resolve_field_values(
        storage: &mut crate::schedule::EntityStorage,
        values: Vec<crate::field::FieldValue>,
    ) -> Result<Vec<PresenterId>, crate::schedule::LookupError> {
        values
            .into_iter()
            .map(|v| Self::resolve_field_value(storage, v))
            .collect()
    }
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
