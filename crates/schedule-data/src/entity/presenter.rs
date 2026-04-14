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
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::InternalData;
        let presenter_id = entity.id();
        let ids = PresenterEntityType::groups_of(&schedule.entities, presenter_id);
        Some(crate::field::FieldValue::presenter_list(ids))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let presenter_id = entity.id();
        let group_ids = PresenterId::from_field_values(value, schedule)?;
        PresenterEntityType::set_groups(&mut schedule.entities, presenter_id, group_ids)
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
        use crate::entity::InternalData;
        let presenter_id = entity.id();
        let is_grp = entity.is_explicit_group
            || !schedule.entities.presenter_group_members.by_left(&presenter_id).is_empty();
        Some(crate::field::FieldValue::Boolean(is_grp))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let flag = value.as_bool();
        let presenter_id = entity.id();
        PresenterEntityType::set_explicit_group(&mut schedule.entities, presenter_id, flag);
        Ok(())
    })]
    pub is_group: bool,

    #[computed_field(display = "Members", description = "Presenters in this group")]
    #[alias("member_ids", "group_members")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::InternalData;
        let group_id = entity.id();
        let ids = PresenterEntityType::members_of(&schedule.entities, group_id);
        Some(crate::field::FieldValue::presenter_list(ids))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let group_id = entity.id();
        let member_ids = PresenterId::from_field_values(value, schedule)?;
        PresenterEntityType::set_members(&mut schedule.entities, group_id, member_ids)
    })]
    pub members: Vec<PresenterId>,

    /// All panels this presenter is directly assigned to (via PanelToPresenter edges).
    #[computed_field(
        display = "Panels",
        description = "All panels this presenter is directly assigned to"
    )]
    #[alias("panel")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::InternalData;
        let presenter_id = entity.id();
        let ids = crate::entity::PanelEntityType::panels_of_presenter(&schedule.entities, presenter_id);
        Some(crate::field::FieldValue::panel_list(ids))
    })]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let presenter_id = entity.id();
        let panel_ids = crate::entity::PanelId::from_field_values(value, schedule)?;
        crate::entity::PanelEntityType::set_panels_of_presenter(&mut schedule.entities, presenter_id, panel_ids)
    })]
    pub panels: Vec<crate::entity::PanelId>,

    /// Add panels to this presenter without replacing existing ones.
    /// Write-only computed field that accepts a single UUID or list of UUIDs.
    #[computed_field(
        display = "Add Panels",
        description = "Add panels to this presenter (append mode)"
    )]
    #[write(|schedule: &mut crate::schedule::Schedule, entity: &mut PresenterData, value: crate::field::FieldValue| {
        use crate::entity::InternalData;
        let presenter_id = entity.id();
        let panel_ids = crate::entity::PanelId::from_field_values(value, schedule)?;
        for panel_id in panel_ids {
            crate::entity::PanelEntityType::add_panel_to_presenter(&mut schedule.entities, presenter_id, panel_id);
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
        use crate::entity::InternalData;
        let presenter_id = entity.id();
        let panel_ids = crate::entity::PanelId::from_field_values(value, schedule)?;
        for panel_id in panel_ids {
            crate::entity::PanelEntityType::remove_panel_from_presenter(&mut schedule.entities, presenter_id, panel_id);
        }
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
        use crate::entity::InternalData;
        let presenter_id = entity.id();
        let ids = PresenterEntityType::inclusive_panels_of(&schedule.entities, presenter_id);
        Some(crate::field::FieldValue::panel_list(ids))
    })]
    pub inclusive_panels: Vec<crate::entity::PanelId>,

    /// Transitive closure: all groups this presenter belongs to (upward).
    #[computed_field(
        display = "Inclusive Groups",
        description = "All groups this presenter belongs to, directly or transitively"
    )]
    #[alias("inclusive_group")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::InternalData;
        let presenter_id = entity.id();
        let ids = PresenterEntityType::inclusive_groups_of(&schedule.entities, presenter_id);
        Some(crate::field::FieldValue::presenter_list(ids))
    })]
    pub inclusive_groups: Vec<PresenterId>,

    /// Transitive closure: all members if this presenter is a group (downward).
    #[computed_field(
        display = "Inclusive Members",
        description = "All members of this presenter if it is a group, directly or transitively"
    )]
    #[alias("inclusive_member")]
    #[read(|schedule: &crate::schedule::Schedule, entity: &PresenterData| {
        use crate::entity::InternalData;
        let group_id = entity.id();
        let ids = PresenterEntityType::inclusive_members_of(&schedule.entities, group_id);
        Some(crate::field::FieldValue::presenter_list(ids))
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
    pub fn is_group(storage: &crate::schedule::EntityStorage, presenter_id: PresenterId) -> bool {
        storage
            .presenters
            .get(presenter_id)
            .is_some_and(|d| d.is_explicit_group)
            || !storage
                .presenter_group_members
                .by_left(&presenter_id)
                .is_empty()
    }

    /// Set the explicit group flag for a presenter.
    ///
    /// If `value` is false, also clears all members from the group to keep the
    /// computed `is_group` field coherent. This is the public API for setting
    /// the computed `is_group` field, which internally sets the explicit flag.
    pub fn set_explicit_group(
        storage: &mut crate::schedule::EntityStorage,
        presenter_id: PresenterId,
        value: bool,
    ) {
        if let Some(data) = storage.presenters.get_mut(presenter_id) {
            data.is_explicit_group = value;
        }
        if !value {
            Self::clear_members(storage, presenter_id);
        }
    }

    /// Remove all membership edges from `group_id` and clear the matching
    /// entry from each member's `group_ids` backing field.
    ///
    /// This does **not** touch `group_id`'s own `is_explicit_group`; callers
    /// are responsible for clearing that separately (needed because the
    /// entity may be temporarily extracted from storage during field writes).
    pub fn clear_members(storage: &mut crate::schedule::EntityStorage, group_id: PresenterId) {
        let member_ids: Vec<PresenterId> =
            storage.presenter_group_members.by_left(&group_id).to_vec();
        for member_id in member_ids {
            if let Some(data) = storage.presenters.get_mut(member_id) {
                data.group_ids.retain(|id| *id != group_id);
            }
        }
        storage.presenter_group_members.clear_by_left(&group_id);
    }

    // -----------------------------------------------------------------------
    // Tag-string lookup / find-or-create
    // -----------------------------------------------------------------------

    /// Find or create a presenter from a tagged credit string.
    ///
    /// See [`crate::schedule::Schedule::find_or_create_tagged_presenter`] for
    /// the full format documentation. This associated function owns the
    /// implementation; the `Schedule` method delegates here.
    ///
    /// Bare names (no tag prefix) auto-create with `Panelist` rank if not
    /// already present.
    #[must_use = "returns the presenter/group ID; check for errors"]
    pub fn find_or_create_tagged(
        storage: &mut crate::schedule::EntityStorage,
        input: &str,
    ) -> Result<PresenterId, LookupError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(LookupError::Empty);
        }

        // --- UUID forms: use trait resolve_uuid_string -----------------------
        use crate::entity::EntityType;
        if let Some(id) = Self::resolve_uuid_string(storage, input) {
            return Ok(id);
        }

        // --- Tag prefix: one or more rank chars followed by ':' ---------------
        if let Some((rank, rest)) = Self::parse_tag_flags(input) {
            return Self::process_tagged(storage, &rest, Some(rank));
        }

        // --- No tag prefix but contains group/flag syntax (=, <) --------------
        if input.contains('=') || input.starts_with('<') {
            return Self::process_tagged(storage, input, None);
        }

        // --- Plain bare name: find existing or create with default Panelist rank
        //     No explicit rank → won't upgrade an existing presenter's rank.
        Ok(Self::find_or_create_by_name(storage, input, None))
    }

    /// Find a presenter by exact case-insensitive name, or create a new one.
    ///
    /// `rank` controls both creation default and upgrade behavior:
    /// - `Some(r)`: new presenters get rank `r`; existing presenters are
    ///   upgraded when `r` has higher priority (lower number) than current.
    /// - `None`: new presenters get `Panelist`; existing presenters keep
    ///   their current rank unchanged (used for untagged bare-name lookups).
    pub fn find_or_create_by_name(
        storage: &mut crate::schedule::EntityStorage,
        name: &str,
        rank: Option<PresenterRank>,
    ) -> PresenterId {
        use uuid::NonNilUuid;
        let existing = storage
            .presenters
            .iter()
            .find(|(_, d)| d.name.eq_ignore_ascii_case(name))
            .map(|(id, _)| id);
        if let Some(id) = existing {
            // Upgrade rank only when an explicit rank was supplied
            if let Some(ref new_rank) = rank {
                if let Some(data) = storage.presenters.get_mut(id) {
                    if new_rank.priority() < data.rank.priority() {
                        data.rank = new_rank.clone();
                    }
                }
            }
            return id;
        }
        let effective_rank = rank.unwrap_or(PresenterRank::Panelist);
        let uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
        let id = PresenterId::from_uuid(uuid);
        let data = PresenterData {
            entity_id: id,
            name: name.to_string(),
            rank: effective_rank,
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
        rank: Option<PresenterRank>,
    ) -> Result<PresenterId, LookupError> {
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
            Self::set_explicit_group(storage, gid, true);
            if always_shown {
                if let Some(gdata) = storage.presenters.get_mut(gid) {
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
                if let Some(pdata) = storage.presenters.get_mut(pid) {
                    if always_grouped {
                        pdata.always_grouped = true;
                    }
                    if !pdata.group_ids.contains(&gid) {
                        pdata.group_ids.push(gid);
                    }
                }
                // Update reverse index: add pid to group's member list
                storage.presenter_group_members.add(gid, pid);
            }
            pid
        };

        Ok(effective)
    }

    /// Add `member` to `group` with default flags (`always_shown_in_group = false`,
    /// `always_grouped = false`).
    ///
    /// No-op if already a member (flags are not changed).
    /// Updates `member.group_ids` backing field and `presenters_by_group` reverse index.
    pub fn add_member(
        storage: &mut crate::schedule::EntityStorage,
        member: PresenterId,
        group: PresenterId,
    ) -> Result<(), crate::schedule::InsertError> {
        if storage.presenter_group_members.contains(&group, &member) {
            return Ok(());
        }
        storage.presenter_group_members.add(group, member);
        if let Some(data) = storage.presenters.get_mut(member) {
            if !data.group_ids.contains(&group) {
                data.group_ids.push(group);
            }
        }
        Ok(())
    }

    /// Add `member` to `group` and set `always_grouped = true`.
    ///
    /// If already a member, updates the flag without duplicating the entry.
    /// Updates `member.always_grouped` and `member.group_ids` backing fields.
    pub fn add_grouped_member(
        storage: &mut crate::schedule::EntityStorage,
        member: PresenterId,
        group: PresenterId,
    ) -> Result<(), crate::schedule::InsertError> {
        if !storage.presenter_group_members.contains(&group, &member) {
            storage.presenter_group_members.add(group, member);
        }
        if let Some(data) = storage.presenters.get_mut(member) {
            data.always_grouped = true;
            if !data.group_ids.contains(&group) {
                data.group_ids.push(group);
            }
        }
        Ok(())
    }

    /// Add `member` to `group` and set `always_shown_in_group = true`.
    ///
    /// If already a member, updates the flag without duplicating the entry.
    /// Updates `member.always_shown_in_group` and `member.group_ids` backing fields.
    pub fn add_shown_member(
        storage: &mut crate::schedule::EntityStorage,
        member: PresenterId,
        group: PresenterId,
    ) -> Result<(), crate::schedule::InsertError> {
        if !storage.presenter_group_members.contains(&group, &member) {
            storage.presenter_group_members.add(group, member);
        }
        if let Some(data) = storage.presenters.get_mut(member) {
            data.always_shown_in_group = true;
            if !data.group_ids.contains(&group) {
                data.group_ids.push(group);
            }
        }
        Ok(())
    }

    /// Remove `member` from `group`.
    ///
    /// Updates `presenters_by_group` reverse index and `member.group_ids` backing field.
    /// Returns `true` if the membership existed and was removed.
    pub fn remove_member(
        storage: &mut crate::schedule::EntityStorage,
        member: PresenterId,
        group: PresenterId,
    ) -> bool {
        let was_member = storage.presenter_group_members.contains(&group, &member);
        if was_member {
            storage.presenter_group_members.remove(&group, &member);
            if let Some(data) = storage.presenters.get_mut(member) {
                data.group_ids.retain(|id| *id != group);
            }
        }
        was_member
    }

    /// Get all groups for this presenter.
    pub fn groups_of(
        storage: &crate::schedule::EntityStorage,
        presenter_id: PresenterId,
    ) -> Vec<PresenterId> {
        storage
            .presenters
            .get(presenter_id)
            .map(|d| d.group_ids.clone())
            .unwrap_or_default()
    }

    /// Set the groups for this presenter.
    ///
    /// Updates both the forward backing field and reverse indexes.
    pub fn set_groups(
        storage: &mut crate::schedule::EntityStorage,
        presenter_id: PresenterId,
        group_ids: Vec<PresenterId>,
    ) -> Result<(), crate::field::FieldError> {
        let entity = storage.presenters.get_mut(presenter_id).ok_or(
            crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            ),
        )?;

        let old_group_ids = entity.group_ids.clone();
        entity.group_ids = group_ids.clone();

        // Remove presenter from old groups' reverse indexes
        for old_id in &old_group_ids {
            storage
                .presenter_group_members
                .remove(old_id, &presenter_id);
        }

        // Add presenter to new groups' reverse indexes
        for new_id in &group_ids {
            storage.presenter_group_members.add(*new_id, presenter_id);
        }

        Ok(())
    }

    /// Get all members of this group.
    pub fn members_of(
        storage: &crate::schedule::EntityStorage,
        group_id: PresenterId,
    ) -> Vec<PresenterId> {
        storage.presenter_group_members.by_left(&group_id).to_vec()
    }

    /// Set the members of this group.
    ///
    /// Updates both the forward reverse index and member group_ids backing fields.
    pub fn set_members(
        storage: &mut crate::schedule::EntityStorage,
        group_id: PresenterId,
        member_ids: Vec<PresenterId>,
    ) -> Result<(), crate::field::FieldError> {
        // Collect old members from reverse index
        let old_member_ids: Vec<PresenterId> =
            storage.presenter_group_members.by_left(&group_id).to_vec();

        // Remove group from departing members' group_ids
        for old_id in &old_member_ids {
            if !member_ids.contains(old_id) {
                if let Some(member_data) = storage.presenters.get_mut(*old_id) {
                    member_data.group_ids.retain(|id| *id != group_id);
                }
            }
        }

        // Add group to new members' group_ids
        for new_id in &member_ids {
            if !old_member_ids.contains(new_id) {
                if let Some(member_data) = storage.presenters.get_mut(*new_id) {
                    if !member_data.group_ids.contains(&group_id) {
                        member_data.group_ids.push(group_id);
                    }
                }
            }
        }

        // Replace reverse index entry
        storage
            .presenter_group_members
            .update_by_left(group_id, &member_ids);

        Ok(())
    }

    /// Remove the explicit group marker from a presenter.
    ///
    /// Sets `is_explicit_group = false`.
    /// Does **not** remove members — use `clear_members` for that.
    ///
    /// Returns `true` if the entity was previously marked as an explicit group.
    pub fn unmark_explicit_group(
        storage: &mut crate::schedule::EntityStorage,
        presenter_id: PresenterId,
    ) -> bool {
        let was_explicit = storage
            .presenters
            .get(presenter_id)
            .is_some_and(|d| d.is_explicit_group);
        // Set without clearing members - use internal flag only
        if let Some(data) = storage.presenters.get_mut(presenter_id) {
            data.is_explicit_group = false;
        }
        was_explicit
    }

    /// Get all panels this presenter is on, directly or via group membership.
    ///
    /// Transitive closure that includes panels from the presenter and all groups
    /// the presenter belongs to (upward transitive via group_ids).
    pub fn inclusive_panels_of(
        storage: &crate::schedule::EntityStorage,
        presenter_id: PresenterId,
    ) -> Vec<crate::entity::PanelId> {
        use std::collections::{HashSet, VecDeque};

        let mut result = Vec::new();
        let mut seen = HashSet::new();

        // Add direct panels
        for panel_id in storage.panels_by_presenter.by_left(&presenter_id) {
            if seen.insert(panel_id.non_nil_uuid()) {
                result.push(*panel_id);
            }
        }

        // Add panels for all inclusive groups (transitive upward via group_ids)
        let mut group_queue: VecDeque<uuid::NonNilUuid> = VecDeque::new();
        if let Some(data) = storage.presenters.get(presenter_id) {
            for gid in &data.group_ids {
                group_queue.push_back(gid.non_nil_uuid());
            }
        }
        let mut seen_groups = HashSet::new();
        while let Some(group_uuid) = group_queue.pop_front() {
            if !seen_groups.insert(group_uuid) {
                continue;
            }
            for panel_id in storage
                .panels_by_presenter
                .by_left(&PresenterId::from_uuid(group_uuid))
            {
                if seen.insert(panel_id.non_nil_uuid()) {
                    result.push(*panel_id);
                }
            }
            if let Some(data) = storage.presenters.get(PresenterId::from_uuid(group_uuid)) {
                for gid in &data.group_ids {
                    group_queue.push_back(gid.non_nil_uuid());
                }
            }
        }

        result
    }

    /// Get all groups this presenter belongs to, directly or transitively.
    ///
    /// Transitive closure that includes groups the presenter is in and all
    /// groups those groups are in (upward transitive via group_ids).
    pub fn inclusive_groups_of(
        storage: &crate::schedule::EntityStorage,
        presenter_id: PresenterId,
    ) -> Vec<PresenterId> {
        use std::collections::{HashSet, VecDeque};

        let mut result = Vec::new();
        let mut seen = HashSet::new();
        let mut queue: VecDeque<uuid::NonNilUuid> = VecDeque::new();

        if let Some(data) = storage.presenters.get(presenter_id) {
            for gid in &data.group_ids {
                queue.push_back(gid.non_nil_uuid());
            }
        }

        while let Some(group_uuid) = queue.pop_front() {
            if seen.insert(group_uuid) {
                result.push(PresenterId::from_uuid(group_uuid));
                if let Some(data) = storage.presenters.get(PresenterId::from_uuid(group_uuid)) {
                    for gid in &data.group_ids {
                        queue.push_back(gid.non_nil_uuid());
                    }
                }
            }
        }

        result
    }

    /// Get all members of this presenter if it is a group, directly or transitively.
    ///
    /// Transitive closure that includes direct members and all members of subgroups
    /// (downward transitive via presenters_by_group).
    pub fn inclusive_members_of(
        storage: &crate::schedule::EntityStorage,
        group_id: PresenterId,
    ) -> Vec<PresenterId> {
        use std::collections::{HashSet, VecDeque};

        let mut result = Vec::new();
        let mut seen = HashSet::new();
        let mut queue: VecDeque<uuid::NonNilUuid> = VecDeque::new();

        for m_id in storage.presenter_group_members.by_left(&group_id) {
            queue.push_back(m_id.non_nil_uuid());
        }

        while let Some(m_uuid) = queue.pop_front() {
            if seen.insert(m_uuid) {
                result.push(PresenterId::from_uuid(m_uuid));
                for sm_id in storage
                    .presenter_group_members
                    .by_left(&PresenterId::from_uuid(m_uuid))
                {
                    queue.push_back(sm_id.non_nil_uuid());
                }
            }
        }

        result
    }
}

/// Custom EntityResolver implementation for PresenterEntityType.
///
/// This overrides the default resolution to support tagged presenter auto-creation.
/// Tags like "P:Name", "G:Name=Team" automatically create presenters with appropriate
/// ranks and group memberships.
impl crate::entity::EntityResolver for PresenterEntityType {
    /// Overrides default resolution to support tagged auto-creation.
    ///
    /// First tries UUID string parsing, then falls back to
    /// `find_or_create_tagged` which handles tags like "P:Name",
    /// "G:Name=Team", "I:Name", and bare names, creating new presenters
    /// with appropriate rank and group membership.
    fn resolve_string(
        storage: &mut crate::schedule::EntityStorage,
        input: &str,
    ) -> Result<Self::Id, crate::field::FieldError> {
        use crate::entity::EntityType;
        // Try UUID string parsing first
        if let Some(id) = Self::resolve_uuid_string(storage, input) {
            return Ok(id);
        }
        // Fall back to tagged presenter lookup with auto-creation
        Self::find_or_create_tagged(storage, input).map_err(|_| {
            crate::field::FieldError::ConversionError(
                crate::field::validation::ConversionError::InvalidFormat,
            )
        })
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

    use crate::entity::EntityResolver;
    use crate::field::FieldValue;

    fn fv(s: &str) -> FieldValue {
        FieldValue::String(s.to_string())
    }

    // --- Bare-name behavior (no tag prefix) ---

    // Bare name auto-creates with default Panelist rank
    #[test]
    fn resolve_bare_name_creates_panelist() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("Jane Doe")).unwrap();
        let data = storage.presenters.get(id).unwrap();
        assert_eq!(data.name, "Jane Doe");
        assert_eq!(data.rank, PresenterRank::Panelist);
        assert!(!data.is_explicit_group);
    }

    // Bare name finds existing (created via tag) without creating a duplicate
    #[test]
    fn resolve_bare_name_finds_existing() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id1 = PresenterEntityType::resolve_field_value(&mut storage, fv("G:Jane")).unwrap();
        let id2 = PresenterEntityType::resolve_field_value(&mut storage, fv("Jane")).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(storage.presenters.len(), 1);
    }

    // --- Tag prefix and rank ---

    // P: creates Panelist
    #[test]
    fn resolve_tagged_panelist() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::Panelist
        );
    }

    // G: is Guest rank, not a group flag
    #[test]
    fn resolve_guest_tag_creates_guest_rank() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("G:Alice")).unwrap();
        let data = storage.presenters.get(id).unwrap();
        assert_eq!(data.rank, PresenterRank::Guest);
        assert!(!data.is_explicit_group);
    }

    // --- Rank upgrade / no-downgrade ---

    // Bare name (Panelist) then I: tag upgrades to InvitedGuest
    #[test]
    fn resolve_rank_upgrade_bare_then_tagged() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id1 = PresenterEntityType::resolve_field_value(&mut storage, fv("Jane Doe")).unwrap();
        assert_eq!(
            storage.presenters.get(id1).unwrap().rank,
            PresenterRank::Panelist
        );

        let id2 = PresenterEntityType::resolve_field_value(&mut storage, fv("I:Jane Doe")).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(
            storage.presenters.get(id2).unwrap().rank,
            PresenterRank::InvitedGuest(None)
        );
    }

    // P: then G: upgrades Panelist → Guest (lower priority number wins)
    #[test]
    fn resolve_rank_upgrade_panelist_to_guest() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::Panelist
        );

        PresenterEntityType::resolve_field_value(&mut storage, fv("G:Alice")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::Guest
        );
    }

    // I: then F: does not downgrade; bare name also does not downgrade
    #[test]
    fn resolve_rank_no_downgrade_mixed() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("I:Jane Doe")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::InvitedGuest(None)
        );

        // Lower-rank tag
        PresenterEntityType::resolve_field_value(&mut storage, fv("F:Jane Doe")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::InvitedGuest(None)
        );

        // Bare name also does not change rank (no implicit rank)
        PresenterEntityType::resolve_field_value(&mut storage, fv("Jane Doe")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::InvitedGuest(None)
        );
    }

    // Bare name does not upgrade FanPanelist to Panelist
    #[test]
    fn resolve_bare_name_does_not_upgrade_fan_panelist() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("F:Alice")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::FanPanelist
        );

        // Bare name has no implicit rank — must not promote FanPanelist
        PresenterEntityType::resolve_field_value(&mut storage, fv("Alice")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::FanPanelist,
            "bare name must not upgrade FanPanelist to Panelist"
        );
    }

    // --- Idempotent / UUID resolution ---

    // Same tag twice returns same presenter
    #[test]
    fn resolve_idempotent_tagged() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id1 = PresenterEntityType::resolve_field_value(&mut storage, fv("G:Troupe")).unwrap();
        let id2 = PresenterEntityType::resolve_field_value(&mut storage, fv("G:Troupe")).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(storage.presenters.len(), 1);
    }

    // Same bare name twice returns same presenter
    #[test]
    fn resolve_idempotent_bare() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id1 = PresenterEntityType::resolve_field_value(&mut storage, fv("Troupe")).unwrap();
        let id2 = PresenterEntityType::resolve_field_value(&mut storage, fv("Troupe")).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(storage.presenters.len(), 1);
    }

    // Prefixed UUID string ("presenter-<uuid>") resolves to the same presenter
    #[test]
    fn resolve_by_prefixed_uuid_string() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice")).unwrap();
        let uuid_str = format!("presenter-{}", id.non_nil_uuid());
        let id2 = PresenterEntityType::resolve_field_value(&mut storage, fv(&uuid_str)).unwrap();
        assert_eq!(id, id2);
    }

    // Bare UUID string (no prefix) resolves to the same presenter
    #[test]
    fn resolve_by_bare_uuid_string() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice")).unwrap();
        let uuid_str = id.non_nil_uuid().to_string();
        let id2 = PresenterEntityType::resolve_field_value(&mut storage, fv(&uuid_str)).unwrap();
        assert_eq!(id, id2);
    }

    // FieldValue::NonNilUuid resolves to the same presenter
    #[test]
    fn resolve_by_non_nil_uuid() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice")).unwrap();
        let id2 = PresenterEntityType::resolve_field_value(
            &mut storage,
            FieldValue::NonNilUuid(id.non_nil_uuid()),
        )
        .unwrap();
        assert_eq!(id, id2);
    }

    // FieldValue::EntityIdentifier resolves to the same presenter
    #[test]
    fn resolve_by_entity_identifier() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice")).unwrap();
        let euuid = crate::entity::EntityUUID::Presenter(id);
        let id2 = PresenterEntityType::resolve_field_value(
            &mut storage,
            FieldValue::EntityIdentifier(euuid),
        )
        .unwrap();
        assert_eq!(id, id2);
    }

    // --- Comma-separated values with mixed tags ---

    #[test]
    fn resolve_field_values_comma_mixed() {
        let mut storage = crate::schedule::EntityStorage::default();
        // Pre-create Bob via bare name so it exists for bare lookup
        PresenterEntityType::resolve_field_value(&mut storage, fv("S:Bob")).unwrap();
        let ids =
            PresenterEntityType::resolve_field_values(&mut storage, fv("P:Alice, Bob, G:Carol"))
                .unwrap();
        assert_eq!(ids.len(), 3);
        assert_eq!(storage.presenters.len(), 3);
    }

    // --- Group membership: single = ---

    // Kind:Name=Group links member to an explicit group
    #[test]
    fn resolve_single_equals_creates_group() {
        let mut storage = crate::schedule::EntityStorage::default();
        let alice_id =
            PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice=BandName")).unwrap();
        let alice = storage.presenters.get(alice_id).unwrap();
        assert_eq!(alice.name, "Alice");
        assert_eq!(alice.rank, PresenterRank::Panelist);
        assert!(!alice.is_explicit_group);
        assert!(!alice.always_grouped);
        let group_id = alice.group_ids.first().copied().unwrap();
        let group = storage.presenters.get(group_id).unwrap();
        assert!(group.is_explicit_group);
        assert!(!group.always_shown_in_group);
        assert_eq!(group.name, "BandName");
    }

    // --- Double == sets always_shown_in_group ---

    #[test]
    fn resolve_double_equals_sets_always_shown() {
        let mut storage = crate::schedule::EntityStorage::default();
        let alice_id =
            PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice==BandName"))
                .unwrap();
        let alice = storage.presenters.get(alice_id).unwrap();
        assert!(!alice.always_grouped);
        let group_id = alice.group_ids.first().copied().unwrap();
        let group = storage.presenters.get(group_id).unwrap();
        assert!(group.is_explicit_group);
        assert!(group.always_shown_in_group);
    }

    // Without tag prefix: I-tagged == also works
    #[test]
    fn resolve_double_equals_with_invited_tag() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("I:Bob==Crew")).unwrap();
        let bob = storage.presenters.get(id).unwrap();
        assert_eq!(bob.rank, PresenterRank::InvitedGuest(None));
        let group_id = bob.group_ids.first().copied().unwrap();
        assert!(
            storage
                .presenters
                .get(group_id)
                .unwrap()
                .always_shown_in_group
        );
    }

    // --- < prefix sets always_grouped ---

    #[test]
    fn resolve_less_than_prefix_sets_always_grouped() {
        let mut storage = crate::schedule::EntityStorage::default();
        let alice_id =
            PresenterEntityType::resolve_field_value(&mut storage, fv("P:<Alice=BandName"))
                .unwrap();
        let alice = storage.presenters.get(alice_id).unwrap();
        assert!(alice.always_grouped);
        assert_eq!(alice.name, "Alice");
        let group_id = alice.group_ids.first().copied().unwrap();
        assert!(storage.presenters.get(group_id).unwrap().is_explicit_group);
    }

    // < with G: tag
    #[test]
    fn resolve_less_than_with_guest_tag() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id =
            PresenterEntityType::resolve_field_value(&mut storage, fv("G:<Carol=Troupe")).unwrap();
        let carol = storage.presenters.get(id).unwrap();
        assert_eq!(carol.rank, PresenterRank::Guest);
        assert!(carol.always_grouped);
    }

    // --- Combined < and == ---

    #[test]
    fn resolve_less_than_double_equals_sets_both_flags() {
        let mut storage = crate::schedule::EntityStorage::default();
        let alice_id =
            PresenterEntityType::resolve_field_value(&mut storage, fv("P:<Alice==BandName"))
                .unwrap();
        let alice = storage.presenters.get(alice_id).unwrap();
        assert!(alice.always_grouped);
        let group_id = alice.group_ids.first().copied().unwrap();
        let group = storage.presenters.get(group_id).unwrap();
        assert!(group.is_explicit_group);
        assert!(group.always_shown_in_group);
    }

    // --- Group-only definition via == with empty member name ---

    // P:==GroupName creates explicit always-shown group, returns the group itself
    #[test]
    fn resolve_double_equals_group_only() {
        let mut storage = crate::schedule::EntityStorage::default();
        let gid =
            PresenterEntityType::resolve_field_value(&mut storage, fv("P:==BandName")).unwrap();
        let group = storage.presenters.get(gid).unwrap();
        assert_eq!(group.name, "BandName");
        assert!(group.is_explicit_group);
        assert!(group.always_shown_in_group);
        assert!(storage.presenter_group_members.by_left(&gid).is_empty());
    }

    // ==GroupName (no tag prefix) creates explicit always-shown group with Panelist rank
    #[test]
    fn resolve_untagged_double_equals_group_only() {
        let mut storage = crate::schedule::EntityStorage::default();
        let gid = PresenterEntityType::resolve_field_value(&mut storage, fv("==Troupe")).unwrap();
        let group = storage.presenters.get(gid).unwrap();
        assert_eq!(group.name, "Troupe");
        assert!(group.is_explicit_group);
        assert!(group.always_shown_in_group);
        assert_eq!(group.rank, PresenterRank::Panelist);
    }

    // <Name=Group (no tag prefix) sets always_grouped with default Panelist rank
    #[test]
    fn resolve_untagged_less_than_equals() {
        let mut storage = crate::schedule::EntityStorage::default();
        let alice_id =
            PresenterEntityType::resolve_field_value(&mut storage, fv("<Alice=BandName")).unwrap();
        let alice = storage.presenters.get(alice_id).unwrap();
        assert_eq!(alice.name, "Alice");
        assert!(alice.always_grouped);
        assert_eq!(alice.rank, PresenterRank::Panelist);
        let group_id = alice.group_ids.first().copied().unwrap();
        assert!(storage.presenters.get(group_id).unwrap().is_explicit_group);
    }

    // Name=Group (no tag, no <) creates membership with default rank, no flags
    #[test]
    fn resolve_untagged_single_equals() {
        let mut storage = crate::schedule::EntityStorage::default();
        let alice_id =
            PresenterEntityType::resolve_field_value(&mut storage, fv("Alice=BandName")).unwrap();
        let alice = storage.presenters.get(alice_id).unwrap();
        assert_eq!(alice.name, "Alice");
        assert!(!alice.always_grouped);
        assert_eq!(alice.rank, PresenterRank::Panelist);
        let group_id = alice.group_ids.first().copied().unwrap();
        let group = storage.presenters.get(group_id).unwrap();
        assert!(group.is_explicit_group);
        assert!(!group.always_shown_in_group);
    }

    // <Name==Group (no tag prefix) sets both always_grouped and always_shown_in_group
    #[test]
    fn resolve_untagged_less_than_double_equals() {
        let mut storage = crate::schedule::EntityStorage::default();
        let alice_id =
            PresenterEntityType::resolve_field_value(&mut storage, fv("<Alice==BandName")).unwrap();
        let alice = storage.presenters.get(alice_id).unwrap();
        assert!(alice.always_grouped);
        let group_id = alice.group_ids.first().copied().unwrap();
        let group = storage.presenters.get(group_id).unwrap();
        assert!(group.is_explicit_group);
        assert!(group.always_shown_in_group);
    }

    // Untagged Name=Group does not upgrade rank on existing FanPanelist
    #[test]
    fn resolve_untagged_equals_does_not_upgrade_rank() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("F:Alice")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::FanPanelist
        );

        // Untagged group syntax — should not upgrade rank
        PresenterEntityType::resolve_field_value(&mut storage, fv("Alice=BandName")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::FanPanelist,
            "untagged group syntax must not upgrade FanPanelist rank"
        );
        // But group membership should still be established
        assert!(!storage.presenters.get(id).unwrap().group_ids.is_empty());
    }

    // --- Flag persistence: once set, later resolves without flag don't clear it ---

    // always_grouped persists when re-resolved without < (with tag)
    #[test]
    fn flag_always_grouped_persists_via_tag() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("P:<Alice=BandName"))
            .unwrap();
        assert!(storage.presenters.get(id).unwrap().always_grouped);

        // Re-resolve with tag but without <
        let id2 =
            PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice=BandName")).unwrap();
        assert_eq!(id, id2);
        assert!(
            storage.presenters.get(id2).unwrap().always_grouped,
            "always_grouped must not be cleared by re-resolve without <"
        );
    }

    // always_grouped persists when re-resolved bare (no tag at all)
    #[test]
    fn flag_always_grouped_persists_via_bare() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("P:<Alice=BandName"))
            .unwrap();
        assert!(storage.presenters.get(id).unwrap().always_grouped);

        // Re-resolve bare
        let id2 = PresenterEntityType::resolve_field_value(&mut storage, fv("Alice")).unwrap();
        assert_eq!(id, id2);
        assert!(
            storage.presenters.get(id2).unwrap().always_grouped,
            "always_grouped must not be cleared by bare-name re-resolve"
        );
    }

    // always_shown_in_group on group persists when re-resolved with single =
    #[test]
    fn flag_always_shown_persists_via_single_equals() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice==BandName"))
            .unwrap();
        let group_id = storage
            .presenters
            .get(id)
            .unwrap()
            .group_ids
            .first()
            .copied()
            .unwrap();
        assert!(
            storage
                .presenters
                .get(group_id)
                .unwrap()
                .always_shown_in_group
        );

        // Re-resolve with single =
        PresenterEntityType::resolve_field_value(&mut storage, fv("P:Alice=BandName")).unwrap();
        assert!(
            storage
                .presenters
                .get(group_id)
                .unwrap()
                .always_shown_in_group,
            "always_shown_in_group must not be cleared by single ="
        );
    }

    // is_explicit_group persists when re-resolved bare
    #[test]
    fn flag_is_explicit_group_persists_via_bare() {
        let mut storage = crate::schedule::EntityStorage::default();
        let gid =
            PresenterEntityType::resolve_field_value(&mut storage, fv("P:==MyGroup")).unwrap();
        assert!(storage.presenters.get(gid).unwrap().is_explicit_group);

        // Re-resolve as bare name
        let gid2 = PresenterEntityType::resolve_field_value(&mut storage, fv("MyGroup")).unwrap();
        assert_eq!(gid, gid2);
        assert!(
            storage.presenters.get(gid2).unwrap().is_explicit_group,
            "is_explicit_group must not be cleared by bare-name re-resolve"
        );
    }

    // Rank persists through bare name re-resolve
    #[test]
    fn flag_rank_persists_via_bare() {
        let mut storage = crate::schedule::EntityStorage::default();
        let id = PresenterEntityType::resolve_field_value(&mut storage, fv("G:Alice")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::Guest
        );

        // Bare name re-resolve (default Panelist) must not downgrade Guest
        PresenterEntityType::resolve_field_value(&mut storage, fv("Alice")).unwrap();
        assert_eq!(
            storage.presenters.get(id).unwrap().rank,
            PresenterRank::Guest,
            "rank must not be downgraded by bare-name re-resolve"
        );
    }
}
