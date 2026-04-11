/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PresenterToGroup edge-entity implementation
//!
//! This edge type connects presenters to groups (self-referential).
//! As an edge-entity, it has its own UUID and stores membership flags.
//!
//! # Group Semantics
//!
//! - **Group marker edge**: A presenter marks itself as a group via self-loop
//!   - `member_uuid == group_uuid` (self-loop)
//!   - `is_group_marker = true`
//!   - `is_group_member = false`
//!
//! - **Group member edge**: A presenter joins a group
//!   - `member_uuid != group_uuid`
//!   - `is_group_marker = false`
//!   - `is_group_member = true`

use crate::EntityFields;
use uuid::NonNilUuid;

/// PresenterToGroup edge-entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
#[entity_kind(PresenterToGroup)]
pub struct PresenterToGroup {
    /// UUID of the member presenter (left side)
    #[field(display = "Member UUID", description = "UUID of the member presenter")]
    #[required]
    #[edge_from(Presenter, accessor = member_id)]
    pub member_uuid: NonNilUuid,

    /// UUID of the group presenter (right side)
    #[field(display = "Group UUID", description = "UUID of the group presenter")]
    #[required]
    #[edge_to(Presenter, accessor = group_id)]
    pub group_uuid: NonNilUuid,

    /// Whether this member should always be shown when the group is displayed
    #[field(
        display = "Always Shown in Group",
        description = "Whether always shown in group"
    )]
    pub always_shown_in_group: bool,

    /// Whether this presenter should always be grouped with this group
    #[field(display = "Always Grouped", description = "Whether always grouped")]
    pub always_grouped: bool,
}

impl PresenterToGroupData {
    /// Check if this is a self-loop (group marker edge)
    pub fn is_self_loop(&self) -> bool {
        self.member_uuid == self.group_uuid
    }
}

// ---------------------------------------------------------------------------
// Convenience queries on PresenterToGroupEntityType
// ---------------------------------------------------------------------------

impl PresenterToGroupEntityType {
    /// Direct groups a presenter belongs to (outgoing edges, excluding self-loops).
    pub fn groups_of(
        storage: &crate::schedule::EntityStorage,
        member: NonNilUuid,
    ) -> Vec<crate::entity::PresenterId> {
        use crate::entity::DirectedEdge;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .outgoing(member)
            .iter()
            .filter_map(|edge_uuid| map.get(edge_uuid))
            .filter(|edge| !edge.is_self_loop())
            .map(|edge| crate::entity::PresenterId::from(edge.right_uuid()))
            .collect()
    }

    /// Direct members of a group (incoming edges, excluding self-loops).
    pub fn members_of(
        storage: &crate::schedule::EntityStorage,
        group: NonNilUuid,
    ) -> Vec<crate::entity::PresenterId> {
        use crate::entity::DirectedEdge;
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .incoming(group)
            .iter()
            .filter_map(|edge_uuid| map.get(edge_uuid))
            .filter(|edge| !edge.is_self_loop())
            .map(|edge| crate::entity::PresenterId::from(edge.left_uuid()))
            .collect()
    }

    /// Whether a presenter is a group.
    ///
    /// Returns `true` if `is_explicit_group` is set on the presenter entity, or if a
    /// self-loop edge exists (legacy; self-loops will be removed in Phase 4).
    pub fn is_group(storage: &crate::schedule::EntityStorage, presenter: NonNilUuid) -> bool {
        if storage
            .presenters
            .get(&presenter)
            .is_some_and(|d| d.is_explicit_group)
        {
            return true;
        }
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let index = Self::edge_index(storage);
        let map = Self::typed_map(storage);
        index
            .outgoing(presenter)
            .iter()
            .filter_map(|edge_uuid| map.get(edge_uuid))
            .any(|edge| edge.is_self_loop())
    }

    /// Inclusive groups of a member (transitive closure upward).
    ///
    /// Returns all groups this member belongs to, directly or transitively.
    /// Uses BFS to traverse group-of-group relationships.
    pub fn inclusive_groups_of(
        storage: &crate::schedule::EntityStorage,
        member: NonNilUuid,
    ) -> Vec<crate::entity::PresenterId> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Start with direct groups
        for group_id in Self::groups_of(storage, member) {
            let group_uuid = group_id.non_nil_uuid();
            if visited.insert(group_uuid) {
                queue.push_back(group_uuid);
                result.push(group_id);
            }
        }

        // BFS upward through group-of-group relationships
        while let Some(current) = queue.pop_front() {
            for group_id in Self::groups_of(storage, current) {
                let group_uuid = group_id.non_nil_uuid();
                if visited.insert(group_uuid) {
                    queue.push_back(group_uuid);
                    result.push(group_id);
                }
            }
        }

        result
    }

    /// Inclusive members of a group (transitive closure downward).
    ///
    /// Returns all members of this group, directly or transitively.
    /// If a member is itself a group, its members are also included.
    pub fn inclusive_members_of(
        storage: &crate::schedule::EntityStorage,
        group: NonNilUuid,
    ) -> Vec<crate::entity::PresenterId> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Start with direct members
        for member_id in Self::members_of(storage, group) {
            let member_uuid = member_id.non_nil_uuid();
            if visited.insert(member_uuid) {
                queue.push_back(member_uuid);
                result.push(member_id);
            }
        }

        // BFS downward through nested groups
        while let Some(current) = queue.pop_front() {
            // If this member is itself a group, include its members
            if Self::is_group(storage, current) {
                for member_id in Self::members_of(storage, current) {
                    let member_uuid = member_id.non_nil_uuid();
                    if visited.insert(member_uuid) {
                        queue.push_back(member_uuid);
                        result.push(member_id);
                    }
                }
            }
        }

        result
    }

    // -----------------------------------------------------------------------
    // Membership mutation methods
    // -----------------------------------------------------------------------

    /// Mark a presenter as a group by adding a self-loop membership edge.
    ///
    /// No-op if already marked as a group.
    pub fn mark_group(
        storage: &mut crate::schedule::EntityStorage,
        presenter_uuid: NonNilUuid,
    ) -> Result<(), crate::schedule::InsertError> {
        if Self::is_group(storage, presenter_uuid) {
            return Ok(());
        }
        let edge_uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
        storage
            .add_edge::<Self>(PresenterToGroupData {
                entity_uuid: edge_uuid,
                member_uuid: presenter_uuid,
                group_uuid: presenter_uuid,
                always_shown_in_group: false,
                always_grouped: false,
            })
            .map(|_| ())
    }

    /// Remove the group marker from a presenter (removes the self-loop edge).
    ///
    /// Returns `true` if the marker existed and was removed.
    pub fn unmark_group(
        storage: &mut crate::schedule::EntityStorage,
        presenter_uuid: NonNilUuid,
    ) -> bool {
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let self_loop_uuid: Option<NonNilUuid> = {
            let outgoing = Self::edge_index(storage).outgoing(presenter_uuid).to_vec();
            let map = Self::typed_map(storage);
            outgoing.into_iter().find(|&edge_uuid| {
                map.get(&edge_uuid).is_some_and(|e| {
                    e.member_uuid == presenter_uuid && e.group_uuid == presenter_uuid
                })
            })
        };
        if let Some(edge_uuid) = self_loop_uuid {
            storage.remove_edge::<Self>(PresenterToGroupId::from(edge_uuid));
            true
        } else {
            false
        }
    }

    /// Find the edge UUID for an existing non-self-loop membership edge.
    pub fn find_membership_edge(
        storage: &crate::schedule::EntityStorage,
        member: NonNilUuid,
        group: NonNilUuid,
    ) -> Option<NonNilUuid> {
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let map = Self::typed_map(storage);
        Self::edge_index(storage)
            .outgoing(member)
            .iter()
            .copied()
            .find(|&edge_uuid| {
                map.get(&edge_uuid)
                    .is_some_and(|e| e.group_uuid == group && !e.is_self_loop())
            })
    }

    /// Add `member` to `group` with default flags (`always_shown_in_group = false`,
    /// `always_grouped = false`).
    ///
    /// No-op if the membership edge already exists (flags are not changed).
    pub fn add_member(
        storage: &mut crate::schedule::EntityStorage,
        member: NonNilUuid,
        group: NonNilUuid,
    ) -> Result<(), crate::schedule::InsertError> {
        if Self::find_membership_edge(storage, member, group).is_some() {
            return Ok(());
        }
        let edge_uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
        storage
            .add_edge::<Self>(PresenterToGroupData {
                entity_uuid: edge_uuid,
                member_uuid: member,
                group_uuid: group,
                always_shown_in_group: false,
                always_grouped: false,
            })
            .map(|_| ())
    }

    /// Add `member` to `group` and set `always_grouped = true`.
    ///
    /// If the edge already exists, it is replaced preserving `always_shown_in_group`.
    pub fn add_grouped_member(
        storage: &mut crate::schedule::EntityStorage,
        member: NonNilUuid,
        group: NonNilUuid,
    ) -> Result<(), crate::schedule::InsertError> {
        use crate::schedule::TypedStorage;
        let shown = if let Some(edge_uuid) = Self::find_membership_edge(storage, member, group) {
            let shown_val = Self::typed_map(storage)
                .get(&edge_uuid)
                .is_some_and(|e| e.always_shown_in_group);
            storage.remove_edge::<Self>(PresenterToGroupId::from(edge_uuid));
            shown_val
        } else {
            false
        };
        let edge_uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
        storage
            .add_edge::<Self>(PresenterToGroupData {
                entity_uuid: edge_uuid,
                member_uuid: member,
                group_uuid: group,
                always_shown_in_group: shown,
                always_grouped: true,
            })
            .map(|_| ())
    }

    /// Add `member` to `group` and set `always_shown_in_group = true`.
    ///
    /// If the edge already exists, it is replaced preserving `always_grouped`.
    pub fn add_shown_member(
        storage: &mut crate::schedule::EntityStorage,
        member: NonNilUuid,
        group: NonNilUuid,
    ) -> Result<(), crate::schedule::InsertError> {
        use crate::schedule::TypedStorage;
        let grouped = if let Some(edge_uuid) = Self::find_membership_edge(storage, member, group) {
            let grouped_val = Self::typed_map(storage)
                .get(&edge_uuid)
                .is_some_and(|e| e.always_grouped);
            storage.remove_edge::<Self>(PresenterToGroupId::from(edge_uuid));
            grouped_val
        } else {
            false
        };
        let edge_uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
        storage
            .add_edge::<Self>(PresenterToGroupData {
                entity_uuid: edge_uuid,
                member_uuid: member,
                group_uuid: group,
                always_shown_in_group: true,
                always_grouped: grouped,
            })
            .map(|_| ())
    }

    /// Remove `member` from `group`.
    ///
    /// Returns `true` if a membership edge existed and was removed.
    pub fn remove_member(
        storage: &mut crate::schedule::EntityStorage,
        member: NonNilUuid,
        group: NonNilUuid,
    ) -> bool {
        if let Some(edge_uuid) = Self::find_membership_edge(storage, member, group) {
            storage.remove_edge::<Self>(PresenterToGroupId::from(edge_uuid));
            true
        } else {
            false
        }
    }

    /// Replace all groups of `member_uuid` with the given group UUIDs.
    ///
    /// Preserves the self-loop group marker if present.
    pub fn set_groups(
        storage: &mut crate::schedule::EntityStorage,
        member_uuid: NonNilUuid,
        group_uuids: &[NonNilUuid],
    ) -> Result<(), crate::schedule::InsertError> {
        use crate::schedule::{EdgePolicy, TypedEdgeStorage, TypedStorage};
        let old_edge_uuids: Vec<NonNilUuid> = {
            let map = Self::typed_map(storage);
            Self::edge_index(storage)
                .outgoing(member_uuid)
                .iter()
                .copied()
                .filter(|&edge_uuid| map.get(&edge_uuid).is_some_and(|e| !e.is_self_loop()))
                .collect()
        };
        for edge_uuid in old_edge_uuids {
            storage.remove_edge::<Self>(PresenterToGroupId::from(edge_uuid));
        }
        for &group_uuid in group_uuids {
            let edge_uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
            storage.add_edge_with_policy::<Self>(
                PresenterToGroupData {
                    entity_uuid: edge_uuid,
                    member_uuid,
                    group_uuid,
                    always_shown_in_group: false,
                    always_grouped: false,
                },
                EdgePolicy::Ignore,
            )?;
        }
        Ok(())
    }

    /// Replace all members of `group_uuid` with the given member UUIDs.
    ///
    /// Preserves the self-loop group marker if present.
    pub fn set_members(
        storage: &mut crate::schedule::EntityStorage,
        group_uuid: NonNilUuid,
        member_uuids: &[NonNilUuid],
    ) -> Result<(), crate::schedule::InsertError> {
        use crate::schedule::{EdgePolicy, TypedEdgeStorage, TypedStorage};
        let old_edge_uuids: Vec<NonNilUuid> = {
            let map = Self::typed_map(storage);
            Self::edge_index(storage)
                .incoming(group_uuid)
                .iter()
                .copied()
                .filter(|&edge_uuid| map.get(&edge_uuid).is_some_and(|e| !e.is_self_loop()))
                .collect()
        };
        for edge_uuid in old_edge_uuids {
            storage.remove_edge::<Self>(PresenterToGroupId::from(edge_uuid));
        }
        for &member_uuid in member_uuids {
            let edge_uuid = unsafe { NonNilUuid::new_unchecked(uuid::Uuid::now_v7()) };
            storage.add_edge_with_policy::<Self>(
                PresenterToGroupData {
                    entity_uuid: edge_uuid,
                    member_uuid,
                    group_uuid,
                    always_shown_in_group: false,
                    always_grouped: false,
                },
                EdgePolicy::Ignore,
            )?;
        }
        Ok(())
    }

    /// Set the `always_shown_in_group` flag on the self-loop group marker edge.
    ///
    /// No-op if the presenter is not marked as a group.
    pub fn set_group_marker_shown(
        storage: &mut crate::schedule::EntityStorage,
        group_uuid: NonNilUuid,
        always_shown: bool,
    ) {
        use crate::schedule::{TypedEdgeStorage, TypedStorage};
        let self_loop_uuid: Option<NonNilUuid> = {
            let outgoing = Self::edge_index(storage).outgoing(group_uuid).to_vec();
            let map = Self::typed_map(storage);
            outgoing.into_iter().find(|&edge_uuid| {
                map.get(&edge_uuid)
                    .is_some_and(|e| e.member_uuid == group_uuid && e.group_uuid == group_uuid)
            })
        };
        if let Some(edge_uuid) = self_loop_uuid {
            if let Some(data) = storage.presenter_to_groups.get_mut(&edge_uuid) {
                data.always_shown_in_group = always_shown;
            }
        }
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
    fn presenter_to_group_id_from_uuid() {
        let nn = test_nn();
        let id = PresenterToGroupId::from(nn);
        assert_eq!(NonNilUuid::from(id), nn);
    }

    #[test]
    fn presenter_to_group_id_try_from_nil_uuid_returns_none() {
        assert!(PresenterToGroupId::try_from_raw_uuid(Uuid::nil()).is_none());
    }

    #[test]
    fn presenter_to_group_id_display() {
        let id = PresenterToGroupId::from(test_nn());
        assert_eq!(
            id.to_string(),
            "presenter-to-group-00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn presenter_to_group_data_ids() {
        let member_uuid = unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        };
        let group_uuid = unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
            ]))
        };

        let data = PresenterToGroupData {
            entity_uuid: test_nn(),
            member_uuid,
            group_uuid,
            always_shown_in_group: true,
            always_grouped: false,
        };

        assert_eq!(data.member_id().non_nil_uuid(), member_uuid);
        assert_eq!(data.group_id().non_nil_uuid(), group_uuid);
        assert!(!data.is_self_loop());
    }

    #[test]
    fn presenter_to_group_self_loop_detection() {
        let uuid = unsafe {
            NonNilUuid::new_unchecked(Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]))
        };

        let data = PresenterToGroupData {
            entity_uuid: test_nn(),
            member_uuid: uuid,
            group_uuid: uuid,
            always_shown_in_group: true,
            always_grouped: true,
        };

        assert!(data.is_self_loop());
    }
}
