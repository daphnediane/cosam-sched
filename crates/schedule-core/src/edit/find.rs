/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::BTreeSet;

use indexmap::IndexMap;

use crate::data::panel::ExtraFields;
use crate::data::presenter::{PresenterGroup, PresenterMember, PresenterRank, PresenterSortRank};
use crate::data::source_info::{ChangeState, SourceInfo};

use super::command::{EditCommand, PanelTypeSnapshot, PresenterSnapshot, RoomSnapshot};
use super::context::EditContext;

/// Options for finding or creating a room.
#[derive(Debug, Clone, Default)]
pub struct RoomOptions {
    pub long_name: Option<String>,
    pub hotel_room: Option<String>,
    pub sort_key: Option<u32>,
    pub is_break: Option<bool>,
    pub metadata: Option<ExtraFields>,
    /// Explicit UID to use when creating (import mode). If `None`, the next
    /// available UID is computed automatically.
    pub uid: Option<u32>,
    pub source: Option<SourceInfo>,
    pub change_state: Option<ChangeState>,
}

/// Options for finding or creating a presenter.
#[derive(Debug, Clone, Default)]
pub struct PresenterOptions {
    pub rank: Option<PresenterRank>,
    /// Groups to add to this presenter's membership list.
    pub add_groups: Vec<String>,
    /// Members to add (only meaningful if the presenter is a group).
    pub add_members: Vec<String>,
    /// If `true`, mark this presenter as a group even when `add_members` is
    /// empty (e.g. a group with no members yet).
    pub is_group: Option<bool>,
    pub always_grouped: Option<bool>,
    pub always_shown: Option<bool>,
    /// Ordering key recording where this presenter was first defined.
    pub sort_rank: Option<PresenterSortRank>,
    pub metadata: Option<ExtraFields>,
    pub source: Option<SourceInfo>,
    pub change_state: Option<ChangeState>,
}

/// Options for finding or creating a panel type.
#[derive(Debug, Clone, Default)]
pub struct PanelTypeOptions {
    pub kind: Option<String>,
    pub color: Option<String>,
    pub bw_color: Option<String>,
    /// Arbitrary color entries (e.g. from import). Merged into the colors map
    /// alongside `color` and `bw_color`.
    pub colors: Option<IndexMap<String, String>>,
    pub is_break: Option<bool>,
    pub is_cafe: Option<bool>,
    pub is_workshop: Option<bool>,
    pub is_hidden: Option<bool>,
    pub is_room_hours: Option<bool>,
    pub is_timeline: Option<bool>,
    pub is_private: Option<bool>,
    pub metadata: Option<ExtraFields>,
    pub source: Option<SourceInfo>,
    pub change_state: Option<ChangeState>,
}

impl EditContext<'_> {
    /// Find a room by short name (case-insensitive), or create one if it does
    /// not exist. If found, specified option fields are applied as updates.
    /// Returns the room's UID.
    pub fn find_or_create_room(&mut self, short_name: &str, opts: &RoomOptions) -> u32 {
        let existing = self
            .schedule
            .rooms
            .iter()
            .find(|r| r.short_name.eq_ignore_ascii_case(short_name));

        if let Some(room) = existing {
            let uid = room.uid;
            // Build updated snapshot from current + opts
            let mut new_snap = RoomSnapshot::from_room(room);
            if let Some(ref long_name) = opts.long_name {
                new_snap.long_name = long_name.clone();
            }
            if let Some(ref hotel_room) = opts.hotel_room {
                new_snap.hotel_room = hotel_room.clone();
            }
            if let Some(sort_key) = opts.sort_key {
                new_snap.sort_key = sort_key;
            }
            if let Some(is_break) = opts.is_break {
                new_snap.is_break = is_break;
            }
            if let Some(ref metadata) = opts.metadata {
                new_snap.metadata = Some(metadata.clone());
            }

            // Only emit a command if something actually changed
            let old_snap = RoomSnapshot::from_room(
                self.schedule
                    .rooms
                    .iter()
                    .find(|r| r.uid == uid)
                    .expect("room just found"),
            );
            if old_snap != new_snap {
                let cmd = EditCommand::UpdateRoom {
                    uid,
                    old: old_snap,
                    new: new_snap,
                };
                self.execute(cmd);
            }
            uid
        } else {
            let uid = opts.uid.unwrap_or_else(|| self.next_room_uid());
            let snapshot = RoomSnapshot {
                short_name: short_name.to_string(),
                long_name: opts
                    .long_name
                    .clone()
                    .unwrap_or_else(|| short_name.to_string()),
                hotel_room: opts.hotel_room.clone().unwrap_or_default(),
                sort_key: opts.sort_key.unwrap_or(uid),
                is_break: opts.is_break.unwrap_or(false),
                metadata: opts.metadata.clone(),
            };
            let cmd = EditCommand::CreateRoom {
                uid,
                snapshot,
                source: opts.source.clone(),
                change_state: opts.change_state.unwrap_or(ChangeState::Added),
            };
            self.execute(cmd);
            uid
        }
    }

    /// Find a presenter by name (case-insensitive), or create one if it does
    /// not exist. If found, specified option fields are merged.
    /// Returns the presenter name as stored.
    pub fn find_or_create_presenter(&mut self, name: &str, opts: &PresenterOptions) -> String {
        let existing = self
            .schedule
            .presenters
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name));

        if let Some(presenter) = existing {
            let stored_name = presenter.name.clone();
            let old_snap = PresenterSnapshot::from_presenter(presenter);
            let mut new_snap = old_snap.clone();

            // Upgrade rank if the new rank has higher priority (lower number)
            if let Some(ref rank) = opts.rank {
                if rank.priority() < new_snap.rank.priority() {
                    new_snap.rank = rank.clone();
                }
            }

            // Merge groups
            if !opts.add_groups.is_empty() {
                let (groups, always_grouped) = match &mut new_snap.is_member {
                    PresenterMember::IsMember(groups, grouped) => (groups, grouped),
                    PresenterMember::NotMember => {
                        new_snap.is_member = PresenterMember::IsMember(BTreeSet::new(), false);
                        match &mut new_snap.is_member {
                            PresenterMember::IsMember(groups, grouped) => (groups, grouped),
                            _ => unreachable!(),
                        }
                    }
                };
                for group in &opts.add_groups {
                    groups.insert(group.clone());
                }
                if let Some(ag) = opts.always_grouped {
                    *always_grouped = ag;
                }
            } else if let Some(ag) = opts.always_grouped {
                if let PresenterMember::IsMember(_, grouped) = &mut new_snap.is_member {
                    *grouped = ag;
                }
            }

            // Merge members (if this presenter is/becomes a group)
            if !opts.add_members.is_empty() {
                let (members, always_shown) = match &mut new_snap.is_grouped {
                    PresenterGroup::IsGroup(members, shown) => (members, shown),
                    PresenterGroup::NotGroup => {
                        new_snap.is_grouped = PresenterGroup::IsGroup(BTreeSet::new(), false);
                        match &mut new_snap.is_grouped {
                            PresenterGroup::IsGroup(members, shown) => (members, shown),
                            _ => unreachable!(),
                        }
                    }
                };
                for member in &opts.add_members {
                    members.insert(member.clone());
                }
                if let Some(shown) = opts.always_shown {
                    *always_shown = shown;
                }
            } else if let Some(shown) = opts.always_shown {
                if let PresenterGroup::IsGroup(_, s) = &mut new_snap.is_grouped {
                    *s = shown;
                }
            }

            // Update sort_rank — earlier (lower) sort_rank wins.
            // PresenterSortRank derives Ord so direct comparison works.
            if let Some(ref new_sr) = opts.sort_rank {
                match &new_snap.sort_rank {
                    None => {
                        new_snap.sort_rank = Some(new_sr.clone());
                    }
                    Some(existing_sr) => {
                        if new_sr < existing_sr {
                            new_snap.sort_rank = Some(new_sr.clone());
                        }
                    }
                }
            }

            if let Some(ref metadata) = opts.metadata {
                new_snap.metadata = Some(metadata.clone());
            }

            if old_snap != new_snap {
                let cmd = EditCommand::UpdatePresenter {
                    name: stored_name.clone(),
                    old: old_snap,
                    new: new_snap,
                };
                self.execute(cmd);
            }
            stored_name
        } else {
            let rank = opts.rank.clone().unwrap_or_default();

            let is_member = if opts.add_groups.is_empty() {
                PresenterMember::NotMember
            } else {
                PresenterMember::IsMember(
                    opts.add_groups.iter().cloned().collect(),
                    opts.always_grouped.unwrap_or(false),
                )
            };

            let is_grouped = if !opts.add_members.is_empty() || opts.is_group == Some(true) {
                PresenterGroup::IsGroup(
                    opts.add_members.iter().cloned().collect(),
                    opts.always_shown.unwrap_or(false),
                )
            } else {
                PresenterGroup::NotGroup
            };

            let snapshot = PresenterSnapshot {
                rank,
                is_member,
                is_grouped,
                sort_rank: opts.sort_rank.clone(),
                metadata: opts.metadata.clone(),
            };
            let cmd = EditCommand::CreatePresenter {
                name: name.to_string(),
                snapshot,
                source: opts.source.clone(),
                change_state: opts.change_state.unwrap_or(ChangeState::Added),
            };
            self.execute(cmd);
            name.to_string()
        }
    }

    /// Find a panel type by prefix (case-insensitive), or create one if it
    /// does not exist. If found, specified option fields are applied as
    /// updates.
    /// Returns the prefix as stored.
    pub fn find_or_create_panel_type(&mut self, prefix: &str, opts: &PanelTypeOptions) -> String {
        // Panel types use exact prefix matching (they're keys in an IndexMap)
        let existing_key = self
            .schedule
            .panel_types
            .keys()
            .find(|k| k.eq_ignore_ascii_case(prefix))
            .cloned();

        if let Some(key) = existing_key {
            let pt = self.schedule.panel_types.get(&key).expect("key just found");
            let old_snap = PanelTypeSnapshot::from_panel_type(pt);
            let mut new_snap = old_snap.clone();

            if let Some(ref kind) = opts.kind {
                new_snap.kind = kind.clone();
            }
            if let Some(ref color) = opts.color {
                new_snap.colors.insert("color".to_string(), color.clone());
            }
            if let Some(ref bw) = opts.bw_color {
                new_snap.colors.insert("bw".to_string(), bw.clone());
            }
            if let Some(ref extra_colors) = opts.colors {
                for (k, v) in extra_colors {
                    new_snap.colors.insert(k.clone(), v.clone());
                }
            }
            if let Some(v) = opts.is_break {
                new_snap.is_break = v;
            }
            if let Some(v) = opts.is_cafe {
                new_snap.is_cafe = v;
            }
            if let Some(v) = opts.is_workshop {
                new_snap.is_workshop = v;
            }
            if let Some(v) = opts.is_hidden {
                new_snap.is_hidden = v;
            }
            if let Some(v) = opts.is_room_hours {
                new_snap.is_room_hours = v;
            }
            if let Some(v) = opts.is_timeline {
                new_snap.is_timeline = v;
            }
            if let Some(v) = opts.is_private {
                new_snap.is_private = v;
            }
            if let Some(ref metadata) = opts.metadata {
                new_snap.metadata = Some(metadata.clone());
            }

            if old_snap != new_snap {
                let cmd = EditCommand::UpdatePanelType {
                    prefix: key.clone(),
                    old: old_snap,
                    new: new_snap,
                };
                self.execute(cmd);
            }
            key
        } else {
            let mut colors = IndexMap::new();
            if let Some(ref extra_colors) = opts.colors {
                colors.extend(extra_colors.clone());
            }
            if let Some(ref color) = opts.color {
                colors.insert("color".to_string(), color.clone());
            }
            if let Some(ref bw) = opts.bw_color {
                colors.insert("bw".to_string(), bw.clone());
            }

            let snapshot = PanelTypeSnapshot {
                kind: opts.kind.clone().unwrap_or_default(),
                colors,
                is_break: opts.is_break.unwrap_or(false),
                is_cafe: opts.is_cafe.unwrap_or(false),
                is_workshop: opts.is_workshop.unwrap_or(false),
                is_hidden: opts.is_hidden.unwrap_or(false),
                is_room_hours: opts.is_room_hours.unwrap_or(false),
                is_timeline: opts.is_timeline.unwrap_or(false),
                is_private: opts.is_private.unwrap_or(false),
                metadata: opts.metadata.clone(),
            };
            let cmd = EditCommand::CreatePanelType {
                prefix: prefix.to_string(),
                snapshot,
                source: opts.source.clone(),
                change_state: opts.change_state.unwrap_or(ChangeState::Added),
            };
            self.execute(cmd);
            prefix.to_string()
        }
    }

    /// Parse a potentially tagged presenter string and find-or-create the
    /// presenter (and optional group) in the schedule.
    ///
    /// Tagged format: `<tag>:[<][name][=[=]group]` where tag is one of
    /// `G/J/I/S/P` (case-insensitive).
    ///
    /// - If the input matches an existing presenter name exactly
    ///   (case-insensitive), returns `Some(stored_name)` immediately.
    /// - If the input has a tag prefix, parses it to extract rank, name,
    ///   group, and flags (`always_grouped`, `always_shown`), then
    ///   creates/updates both the group and name entries via
    ///   `find_or_create_presenter`.
    /// - If `always_create` is true and the input has no tag, creates the
    ///   presenter with default rank (`FanPanelist`).
    /// - Returns `None` if the input is empty, or if a tag prefix is
    ///   present but resolves to `Other` (a column-type header, not a
    ///   real presenter).
    pub fn update_or_create_presenter(
        &mut self,
        input: &str,
        always_create: bool,
        column_index: Option<u32>,
        row_index: Option<u32>,
    ) -> Option<String> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }

        // Quick check: does it already exist as-is?
        if let Some(existing) = self
            .schedule
            .presenters
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(input))
        {
            return Some(existing.name.clone());
        }

        // Try to parse tag prefix  (e.g. "G:Name=Group")
        if let Some((rank, rest)) = Self::parse_tag_prefix(input) {
            return self.process_tagged_presenter(&rest, rank, column_index, row_index);
        }

        // No tag prefix — create if always_create, otherwise None
        if always_create {
            let sort_rank = match (column_index, row_index) {
                (Some(ci), Some(ri)) => Some(PresenterSortRank::new(ci, ri, 0)),
                _ => None,
            };
            let opts = PresenterOptions {
                rank: Some(PresenterRank::FanPanelist),
                sort_rank,
                ..Default::default()
            };
            Some(self.find_or_create_presenter(input, &opts))
        } else {
            None
        }
    }

    /// Parse a single-char tag prefix (`G:`, `J:`, etc.) from the start of
    /// a presenter string. Returns `(rank, rest)` if found.
    fn parse_tag_prefix(input: &str) -> Option<(PresenterRank, String)> {
        let mut chars = input.chars();
        let first = chars.next()?;
        let colon = chars.next()?;
        if colon != ':' {
            return None;
        }
        let rank = PresenterRank::from_prefix_char(first)?;
        let rest = input[2..].trim().to_string();
        Some((rank, rest))
    }

    /// Process the portion after the tag prefix, handling `<`, `=`, `==`
    /// syntax for group membership and flags.
    ///
    /// Returns the presenter name (or group name if no individual name),
    /// or `None` if the result is just "Other".
    fn process_tagged_presenter(
        &mut self,
        rest: &str,
        rank: PresenterRank,
        column_index: Option<u32>,
        row_index: Option<u32>,
    ) -> Option<String> {
        if rest.is_empty() {
            return None;
        }

        // "Other" is a column-type marker, not a real presenter
        if rest.eq_ignore_ascii_case("other") {
            return None;
        }

        // Split on first '=' to get name and optional group
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

        // Check for '<' prefix → always_grouped
        let (presenter_name, always_grouped) = if let Some(stripped) = name_raw.strip_prefix('<') {
            (stripped.trim().to_string(), true)
        } else {
            (name_raw, false)
        };

        // Check for '=' prefix on group (original '==' in input) → always_shown
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

        // Build sort ranks using member_index: 0 for the group, 1 for the
        // individual member.
        let group_sort = match (column_index, row_index) {
            (Some(ci), Some(ri)) => Some(PresenterSortRank::schedule_group(ci, ri)),
            _ => None,
        };
        let member_sort = match (column_index, row_index) {
            (Some(ci), Some(ri)) => Some(PresenterSortRank::schedule_member(ci, ri)),
            _ => None,
        };

        // Create/update group if present
        if let Some(ref gname) = group_name {
            let mut group_opts = PresenterOptions {
                rank: Some(rank.clone()),
                is_group: Some(true),
                always_shown: Some(always_shown),
                sort_rank: group_sort,
                ..Default::default()
            };
            if !presenter_name.is_empty() {
                group_opts.add_members = vec![presenter_name.clone()];
            }
            self.find_or_create_presenter(gname, &group_opts);
        }

        // If presenter name is empty or same as group, return the group name
        if presenter_name.is_empty() {
            return group_name;
        }
        if group_name
            .as_ref()
            .is_some_and(|g| g.eq_ignore_ascii_case(&presenter_name))
        {
            return group_name;
        }

        // Create/update the individual presenter
        let mut name_opts = PresenterOptions {
            rank: Some(rank),
            sort_rank: member_sort,
            ..Default::default()
        };
        if let Some(ref gname) = group_name {
            name_opts.add_groups = vec![gname.clone()];
            name_opts.always_grouped = Some(always_grouped);
        }
        Some(self.find_or_create_presenter(&presenter_name, &name_opts))
    }

    /// Compute the next available room UID.
    fn next_room_uid(&self) -> u32 {
        self.schedule.rooms.iter().map(|r| r.uid).max().unwrap_or(0) + 1
    }
}
