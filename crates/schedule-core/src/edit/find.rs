/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use indexmap::IndexMap;

use crate::data::panel::ExtraFields;
use crate::data::presenter::{PresenterRank, PresenterSortRank};
use crate::data::relationship::GroupEdge;
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
    /// Ordering key recording where this presenter was first defined.
    pub sort_rank: Option<PresenterSortRank>,
    pub metadata: Option<ExtraFields>,
    pub source: Option<SourceInfo>,
    pub change_state: Option<ChangeState>,

    // Relationship management fields
    pub add_groups: Vec<String>,
    pub add_members: Vec<String>,
    pub is_group: Option<bool>,
    pub always_grouped: Option<bool>,
    pub always_shown: Option<bool>,
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

            let mut commands = Vec::new();

            if old_snap != new_snap {
                commands.push(EditCommand::UpdatePresenter {
                    name: stored_name.clone(),
                    old: old_snap,
                    new: new_snap,
                });
            }

            // Add relationship commands to the same batch
            let relationship_commands = self.collect_relationship_commands(&stored_name, opts);
            commands.extend(relationship_commands);

            // Execute all commands as a single batch
            if !commands.is_empty() {
                self.execute_batch(commands);
            }

            stored_name
        } else {
            let rank = opts.rank.clone().unwrap_or_default();

            let snapshot = PresenterSnapshot {
                rank,
                sort_rank: opts.sort_rank.clone(),
                metadata: opts.metadata.clone(),
            };

            let mut commands = Vec::new();

            // Create the main presenter
            commands.push(EditCommand::CreatePresenter {
                name: name.to_string(),
                snapshot,
                source: opts.source.clone(),
                change_state: opts.change_state.unwrap_or(ChangeState::Added),
            });

            // Add relationship commands to the same batch
            let relationship_commands = self.collect_relationship_commands(name, opts);
            commands.extend(relationship_commands);

            // Execute all commands as a single batch
            self.execute_batch(commands);

            name.to_string()
        }
    }

    /// Collect relationship-related commands without executing them
    fn collect_relationship_commands(
        &self,
        presenter_name: &str,
        opts: &PresenterOptions,
    ) -> Vec<EditCommand> {
        let mut commands = Vec::new();

        // Handle is_group and always_shown flags
        if let Some(is_group) = opts.is_group {
            if is_group {
                // Create a group-only edge if this is a group
                let edge = GroupEdge::group_only(
                    presenter_name.to_string(),
                    opts.always_shown.unwrap_or(false),
                );
                commands.push(EditCommand::AddRelationship { edge });
            }
        }

        // Handle add_groups (presenter -> group relationships)
        for group_name in &opts.add_groups {
            // Ensure the group presenter exists through command system
            if !self
                .schedule
                .presenters
                .iter()
                .any(|p| p.name.eq_ignore_ascii_case(group_name))
            {
                // Create the group presenter command
                let snapshot = PresenterSnapshot {
                    rank: PresenterRank::default(),
                    sort_rank: None,
                    metadata: None,
                };
                commands.push(EditCommand::CreatePresenter {
                    name: group_name.clone(),
                    snapshot,
                    source: None,
                    change_state: ChangeState::Added,
                });

                // Create the group-only edge
                let edge = GroupEdge::group_only(group_name.clone(), false);
                commands.push(EditCommand::AddRelationship { edge });
            }

            let edge = GroupEdge::new(
                presenter_name.to_string(),
                group_name.clone(),
                opts.always_grouped.unwrap_or(false), // always_grouped
                false,                                // always_shown
            );
            commands.push(EditCommand::AddRelationship { edge });
        }

        // Handle add_members (group -> presenter relationships)
        for member_name in &opts.add_members {
            // Ensure the member presenter exists through command system
            if !self
                .schedule
                .presenters
                .iter()
                .any(|p| p.name.eq_ignore_ascii_case(member_name))
            {
                // Create the member presenter command
                let snapshot = PresenterSnapshot {
                    rank: PresenterRank::default(),
                    sort_rank: None,
                    metadata: None,
                };
                commands.push(EditCommand::CreatePresenter {
                    name: member_name.clone(),
                    snapshot,
                    source: None,
                    change_state: ChangeState::Added,
                });
            }

            let edge = GroupEdge::new(
                member_name.clone(),
                presenter_name.to_string(),
                false, // always_grouped
                false, // always_shown
            );
            commands.push(EditCommand::AddRelationship { edge });
        }

        commands
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
            let group_opts = PresenterOptions {
                rank: Some(rank.clone()),
                sort_rank: group_sort,
                ..Default::default()
            };
            self.find_or_create_presenter(gname, &group_opts);

            // Add group-only edge if always_shown
            if always_shown {
                let cmd = EditCommand::AddRelationship {
                    edge: crate::data::relationship::GroupEdge::group_only(gname.clone(), true),
                };
                self.execute(cmd);
            }

            // Add member edges if presenter name is provided and different from group
            if !presenter_name.is_empty() && !presenter_name.eq_ignore_ascii_case(gname) {
                let cmd = EditCommand::AddRelationship {
                    edge: crate::data::relationship::GroupEdge::new(
                        presenter_name.clone(),
                        gname.clone(),
                        always_grouped,
                        false, // members don't set always_shown on the group edge
                    ),
                };
                self.execute(cmd);
            }
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
        let name_opts = PresenterOptions {
            rank: Some(rank),
            sort_rank: member_sort,
            ..Default::default()
        };
        let presenter_name = self.find_or_create_presenter(&presenter_name, &name_opts);

        // Add relationship to group if present
        if let Some(ref gname) = group_name {
            let cmd = EditCommand::AddRelationship {
                edge: crate::data::relationship::GroupEdge::new(
                    presenter_name.clone(),
                    gname.clone(),
                    always_grouped,
                    false, // individual members don't set always_shown on groups
                ),
            };
            self.execute(cmd);
        }

        Some(presenter_name)
    }

    /// Compute the next available room UID.
    fn next_room_uid(&self) -> u32 {
        self.schedule.rooms.iter().map(|r| r.uid).max().unwrap_or(0) + 1
    }
}
