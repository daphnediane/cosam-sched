/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PresenterToGroup edge implementation

use crate::edge::{Edge, EdgeError, EdgeId, EdgeStorage, EdgeType};
use crate::entity::{NonNilUuid, PresenterId};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Cached relationship data for fast queries
#[derive(Debug, Default, Clone)]
pub struct RelationshipCache {
    /// Direct parent groups for each member
    direct_parent_groups: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// Direct members for each group
    direct_members: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// All members (transitive) for each group
    inclusive_members: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// All groups (transitive) for each member
    inclusive_groups: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// Cache version to detect invalidation
    cache_version: u64,
}

impl RelationshipCache {
    /// Clear all cached data
    pub fn clear(&mut self) {
        self.direct_parent_groups.clear();
        self.direct_members.clear();
        self.inclusive_members.clear();
        self.inclusive_groups.clear();
    }

    /// Get direct parent groups for a member
    pub fn get_direct_groups(&self, member_id: &NonNilUuid) -> &[NonNilUuid] {
        self.direct_parent_groups
            .get(member_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get direct members for a group
    pub fn get_direct_members(&self, group_id: &NonNilUuid) -> &[NonNilUuid] {
        self.direct_members
            .get(group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all members (transitive) for a group
    pub fn get_inclusive_members(&self, group_id: &NonNilUuid) -> &[NonNilUuid] {
        self.inclusive_members
            .get(group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all groups (transitive) for a member
    pub fn get_inclusive_groups(&self, member_id: &NonNilUuid) -> &[NonNilUuid] {
        self.inclusive_groups
            .get(member_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// Edge representing a group/member relationship (based on GroupEdge from schedule-core)
/// A self-referencing edge (member_id == group_id) indicates a group marker with unknown members.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PresenterToGroupEdge {
    member_id: PresenterId,
    group_id: PresenterId,
    always_grouped: bool,
    always_shown_in_group: bool,
}

impl PresenterToGroupEdge {
    /// Create a new presenter-group edge
    /// If member_id == group_id, this creates a group marker edge.
    pub fn new(
        member_id: PresenterId,
        group_id: PresenterId,
        always_grouped: bool,
        always_shown_in_group: bool,
    ) -> Self {
        Self {
            member_id,
            group_id,
            always_grouped,
            always_shown_in_group,
        }
    }

    /// Create an edge for a group with unknown members (G:==Group syntax)
    /// This is a self-referencing edge where member_id == group_id.
    pub fn group_only(group_id: PresenterId, always_shown_in_group: bool) -> Self {
        Self {
            member_id: group_id,
            group_id,
            always_grouped: false,
            always_shown_in_group,
        }
    }

    /// Check if this edge represents a group marker (self-referencing edge)
    pub fn is_group_marker(&self) -> bool {
        self.member_id == self.group_id
    }

    /// Get the member ID
    pub fn member_id(&self) -> PresenterId {
        self.member_id
    }

    /// Get the group ID
    pub fn group_id(&self) -> PresenterId {
        self.group_id
    }

    /// Get the always_grouped flag (always false for group markers)
    pub fn always_grouped(&self) -> bool {
        if self.is_group_marker() {
            false
        } else {
            self.always_grouped
        }
    }

    /// Get the always_shown_in_group flag
    pub fn always_shown_in_group(&self) -> bool {
        self.always_shown_in_group
    }
}

impl Edge for PresenterToGroupEdge {
    type FromEntity = crate::entity::PresenterEntityType;
    type ToEntity = crate::entity::PresenterEntityType;
    type Data = Self;

    fn from_uuid(&self) -> NonNilUuid {
        NonNilUuid::from(self.member_id)
    }

    fn to_uuid(&self) -> NonNilUuid {
        NonNilUuid::from(self.group_id)
    }

    fn data(&self) -> &Self::Data {
        self
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        self
    }

    fn edge_type(&self) -> EdgeType {
        EdgeType::PresenterToGroup
    }

    /// Override is_self_cycle to indicate group markers
    fn is_self_cycle(&self) -> bool {
        self.is_group_marker()
    }
}

/// Specialized storage for PresenterToGroup with relationship manager features
#[derive(Debug, Clone)]
pub struct PresenterToGroupStorage {
    edges: BTreeSet<PresenterToGroupEdge>,
    member_to_groups: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    group_to_members: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    cache: RelationshipCache,
    cache_invalidation: u64,
    next_id: u64,
}

impl PresenterToGroupStorage {
    /// Create a new relationship manager
    pub fn new() -> Self {
        Self {
            edges: BTreeSet::new(),
            member_to_groups: HashMap::new(),
            group_to_members: HashMap::new(),
            cache: RelationshipCache::default(),
            cache_invalidation: 0,
            next_id: 0,
        }
    }

    /// Add a relationship edge with cycle tolerance
    pub fn add_edge(&mut self, edge: PresenterToGroupEdge) -> Result<EdgeId, EdgeError> {
        self.add_edge_internal(edge)
    }

    /// Internal implementation for adding a relationship edge
    fn add_edge_internal(&mut self, edge: PresenterToGroupEdge) -> Result<EdgeId, EdgeError> {
        let edge_id = EdgeId(self.next_id);
        self.next_id += 1;

        // Remove existing edge for this member-group pair if it exists
        let member_uuid = NonNilUuid::from(edge.member_id());
        let group_uuid = NonNilUuid::from(edge.group_id());
        self.remove_edge_internal(&member_uuid, &group_uuid);

        // Add the edge
        self.edges.insert(edge.clone());

        // Update indexes using NonNilUuid for efficiency
        // Only update member->groups index for non-group-marker edges
        if !edge.is_group_marker() {
            self.member_to_groups
                .entry(member_uuid)
                .or_default()
                .push(group_uuid);
        }

        // Only update group->members index for non-group-marker edges
        if !edge.is_group_marker() {
            self.group_to_members
                .entry(group_uuid)
                .or_default()
                .push(member_uuid);
        }

        // Invalidate cache
        self.invalidate_cache();

        Ok(edge_id)
    }

    /// Remove a relationship edge by member/group pair (ignoring flag values)
    pub fn remove_edge(
        &mut self,
        member_id: NonNilUuid,
        group_id: NonNilUuid,
    ) -> Result<(), EdgeError> {
        self.remove_edge_internal(&member_id, &group_id);
        self.invalidate_cache();
        Ok(())
    }

    /// Internal edge removal without cache invalidation
    fn remove_edge_internal(&mut self, member_id: &NonNilUuid, group_id: &NonNilUuid) {
        // Find the actual edge (flags may differ from a default-constructed edge)
        let found = self
            .edges
            .iter()
            .find(|e| {
                NonNilUuid::from(e.member_id()) == *member_id
                    && NonNilUuid::from(e.group_id()) == *group_id
            })
            .cloned();

        if let Some(edge) = found {
            self.edges.remove(&edge);

            // Update indexes
            if !edge.is_group_marker() {
                let member_uuid = NonNilUuid::from(edge.member_id());
                if let Some(groups) = self.member_to_groups.get_mut(&member_uuid) {
                    let group_uuid = NonNilUuid::from(edge.group_id());
                    groups.retain(|&g| g != group_uuid);
                    if groups.is_empty() {
                        self.member_to_groups.remove(&member_uuid);
                    }
                }
            }

            let group_uuid = NonNilUuid::from(edge.group_id());
            if let Some(members) = self.group_to_members.get_mut(&group_uuid) {
                let member_uuid = NonNilUuid::from(edge.member_id());
                members.retain(|&m| m != member_uuid);
                if members.is_empty() {
                    self.group_to_members.remove(&group_uuid);
                }
            }
        }
    }

    /// Clear all members for a group
    pub fn clear_group(&mut self, group_id: NonNilUuid) {
        // Collect members to remove
        let members_to_remove: Vec<NonNilUuid> = self
            .group_to_members
            .get(&group_id)
            .cloned()
            .unwrap_or_default();

        // Remove each edge
        for member_id in &members_to_remove {
            self.remove_edge_internal(member_id, &group_id);
        }

        // Also remove group-only edges
        let group_only_edge = PresenterToGroupEdge::group_only(PresenterId::from(group_id), false);
        self.edges.remove(&group_only_edge);

        self.invalidate_cache();
    }

    /// Get all members (transitive) for a group
    pub fn get_inclusive_members(&mut self, group_id: NonNilUuid) -> &[NonNilUuid] {
        self.ensure_cache_valid();
        self.cache.get_inclusive_members(&group_id)
    }

    /// Get all groups (transitive) for a member
    pub fn get_inclusive_groups(&mut self, member_id: NonNilUuid) -> &[NonNilUuid] {
        self.ensure_cache_valid();
        self.cache.get_inclusive_groups(&member_id)
    }

    /// Get direct parent groups for a member (non-caching, borrows `&self`).
    pub fn direct_groups_of(&self, member_id: NonNilUuid) -> &[NonNilUuid] {
        self.member_to_groups
            .get(&member_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get direct members for a group (non-caching, borrows `&self`).
    pub fn direct_members_of(&self, group_id: NonNilUuid) -> &[NonNilUuid] {
        self.group_to_members
            .get(&group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a presenter is a group
    pub fn is_group(&self, presenter_id: NonNilUuid) -> bool {
        self.group_to_members.contains_key(&presenter_id)
    }

    /// Check if a member should always be grouped with a specific group
    pub fn is_always_grouped(&self, member_id: NonNilUuid, group_id: NonNilUuid) -> bool {
        let member_id = PresenterId::from(member_id);
        let group_id = PresenterId::from(group_id);
        self.edges
            .iter()
            .find(|e| e.member_id() == member_id && e.group_id() == group_id)
            .map(|e| e.always_grouped())
            .unwrap_or(false)
    }

    /// Check if a member should always be grouped (with any group)
    pub fn is_any_always_grouped(&self, member_id: NonNilUuid) -> bool {
        let member_id = PresenterId::from(member_id);
        self.edges
            .iter()
            .any(|e| e.member_id() == member_id && e.always_grouped())
    }

    /// Check if a group should always be shown as a group
    pub fn is_always_shown_in_group(&self, group_id: NonNilUuid) -> bool {
        let group_id = PresenterId::from(group_id);
        self.edges
            .iter()
            .any(|e| e.group_id() == group_id && e.always_shown_in_group())
    }

    /// Get all edges (for debugging/serialization)
    pub fn edges(&self) -> impl Iterator<Item = &PresenterToGroupEdge> {
        self.edges.iter()
    }

    /// Find an edge by member/group pair, returning a reference if it exists.
    pub fn find_edge(
        &self,
        member_id: NonNilUuid,
        group_id: NonNilUuid,
    ) -> Option<&PresenterToGroupEdge> {
        let member_id = PresenterId::from(member_id);
        let group_id = PresenterId::from(group_id);
        self.edges
            .iter()
            .find(|e| e.member_id() == member_id && e.group_id() == group_id)
    }

    /// Invalidate the cache
    pub fn invalidate_cache(&mut self) {
        self.cache_invalidation += 1;
    }

    /// Ensure cache is valid, rebuilding if necessary
    fn ensure_cache_valid(&mut self) {
        if self.cache.cache_version != self.cache_invalidation {
            self.rebuild_cache();
        }
    }

    /// Rebuild the cache from current edges
    pub fn rebuild_cache(&mut self) {
        self.cache.clear();
        self.cache.cache_version = self.cache_invalidation;

        // Build direct relationships
        for edge in &self.edges {
            let member_uuid = NonNilUuid::from(edge.member_id());
            let group_uuid = NonNilUuid::from(edge.group_id());

            // Only add non-group-marker edges to relationship indexes
            if !edge.is_group_marker() {
                self.cache
                    .direct_parent_groups
                    .entry(member_uuid)
                    .or_default()
                    .push(group_uuid);

                self.cache
                    .direct_members
                    .entry(group_uuid)
                    .or_default()
                    .push(member_uuid);
            }
        }

        // Build transitive relationships
        self.build_transitive_relationships();
    }

    /// Build transitive closure for relationships
    fn build_transitive_relationships(&mut self) {
        // For inclusive members: find all members that belong to a group, directly or indirectly
        let all_groups: Vec<NonNilUuid> = self.group_to_members.keys().cloned().collect();

        for group in all_groups {
            let mut inclusive_members = HashSet::new();
            let mut to_visit = vec![group];
            let mut visited = HashSet::new();

            while let Some(current) = to_visit.pop() {
                if !visited.insert(current) {
                    continue;
                }
                if let Some(direct_members) = self.cache.direct_members.get(&current) {
                    for member in direct_members {
                        if inclusive_members.insert(*member) {
                            // If this member is also a group (has its own members), add its members too
                            if self.group_to_members.contains_key(member) {
                                to_visit.push(*member);
                            }
                        }
                    }
                }
            }

            let mut members: Vec<NonNilUuid> = inclusive_members.into_iter().collect();
            members.sort();
            self.cache.inclusive_members.insert(group, members);
        }

        // For inclusive groups: find all groups a member belongs to, directly or indirectly
        let all_members: Vec<NonNilUuid> = self.member_to_groups.keys().cloned().collect();

        for member in all_members {
            let mut inclusive_groups = HashSet::new();
            let mut to_visit = vec![member];
            let mut visited = HashSet::new();

            while let Some(current) = to_visit.pop() {
                if !visited.insert(current) {
                    continue;
                }
                if let Some(direct_groups) = self.cache.direct_parent_groups.get(&current) {
                    for group in direct_groups {
                        inclusive_groups.insert(*group);
                        to_visit.push(*group);
                    }
                }
            }

            let mut groups: Vec<NonNilUuid> = inclusive_groups.into_iter().collect();
            groups.sort();
            self.cache.inclusive_groups.insert(member, groups);
        }
    }
}

impl Default for PresenterToGroupStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeStorage<PresenterToGroupEdge> for PresenterToGroupStorage {
    fn add_edge(&mut self, edge: PresenterToGroupEdge) -> Result<EdgeId, EdgeError> {
        self.add_edge_internal(edge)
    }

    fn remove_edge(&mut self, _edge_id: EdgeId) -> Result<(), EdgeError> {
        // For PresenterToGroupStorage, we need to find the edge by ID and remove it
        // This is a bit tricky since we use BTreeSet and don't store edge IDs in the set
        // For now, we'll implement this by finding the edge and using remove_edge_internal
        // In practice, the caller should use the member/group specific remove_edge method
        Err(EdgeError::InvalidOperation {
            reason: "Use remove_edge(member_id, group_id) instead".to_string(),
        })
    }

    fn get_edge(&self, _edge_id: EdgeId) -> Option<&PresenterToGroupEdge> {
        // Since edges are stored in a BTreeSet without explicit ID mapping,
        // we can't easily retrieve by edge ID
        // This is a limitation of the current design
        None
    }

    fn find_outgoing(&self, from_uuid: NonNilUuid) -> Vec<&PresenterToGroupEdge> {
        // Outgoing from a member means finding all groups this member belongs to
        self.member_to_groups
            .get(&from_uuid)
            .map(|group_ids| {
                group_ids
                    .iter()
                    .filter_map(|&group_id| {
                        let member_id = PresenterId::from(from_uuid);
                        let group_id = PresenterId::from(group_id);
                        self.edges
                            .iter()
                            .find(|e| e.member_id() == member_id && e.group_id() == group_id)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn find_incoming(&self, to_uuid: NonNilUuid) -> Vec<&PresenterToGroupEdge> {
        // Incoming to a group means finding all members of this group
        self.group_to_members
            .get(&to_uuid)
            .map(|member_ids| {
                member_ids
                    .iter()
                    .filter_map(|&member_id| {
                        let member_id = PresenterId::from(member_id);
                        let group_id = PresenterId::from(to_uuid);
                        self.edges
                            .iter()
                            .find(|e| e.member_id() == member_id && e.group_id() == group_id)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn edge_exists(&self, from_uuid: NonNilUuid, to_uuid: NonNilUuid) -> bool {
        let member_id = PresenterId::from(from_uuid);
        let group_id = PresenterId::from(to_uuid);
        self.edges
            .iter()
            .any(|e| e.member_id() == member_id && e.group_id() == group_id)
    }

    fn len(&self) -> usize {
        self.edges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::EdgeStorage;

    fn make_id(id: u8) -> PresenterId {
        let uuid = unsafe {
            NonNilUuid::new_unchecked(uuid::Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, id,
            ]))
        };
        PresenterId::from(uuid)
    }

    #[test]
    fn test_circular_group_relationships_dont_cause_infinite_loop() {
        let mut storage = PresenterToGroupStorage::new();
        let group_a = make_id(10);
        let group_b = make_id(11);
        let member = make_id(20);

        // Create a circular relationship: member -> group_a -> group_b -> group_a
        storage
            .add_edge(PresenterToGroupEdge::new(member, group_a, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(group_a, group_b, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(group_b, group_a, false, false))
            .unwrap();

        // Rebuild cache to trigger transitive closure
        storage.rebuild_cache();

        // Should complete without infinite loop
        let member_uuid = NonNilUuid::from(member);
        let group_a_uuid = NonNilUuid::from(group_a);
        let group_b_uuid = NonNilUuid::from(group_b);
        let inclusive_groups = storage.get_inclusive_groups(member_uuid);
        // The member should be in group_a and group_b
        assert!(inclusive_groups.contains(&group_a_uuid));
        assert!(inclusive_groups.contains(&group_b_uuid));
    }

    #[test]
    fn test_direct_cycle_self_group() {
        let mut storage = PresenterToGroupStorage::new();
        let group_id = make_id(10);
        let member_id = make_id(20);

        // A group can be a member of itself (edge case)
        storage
            .add_edge(PresenterToGroupEdge::new(group_id, group_id, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(member_id, group_id, false, false))
            .unwrap();

        storage.rebuild_cache();

        // Should handle self-reference without infinite loop
        let group_uuid = NonNilUuid::from(group_id);
        let member_uuid = NonNilUuid::from(member_id);
        let inclusive_members = storage.get_inclusive_members(group_uuid);
        assert!(inclusive_members.contains(&member_uuid));
    }

    #[test]
    fn test_complex_cyclic_graph() {
        let mut storage = PresenterToGroupStorage::new();
        let group_1 = make_id(10);
        let group_2 = make_id(11);
        let group_3 = make_id(12);
        let member_1 = make_id(20);
        let member_2 = make_id(21);

        // Create a complex cycle: group_1 -> group_2 -> group_3 -> group_1
        storage
            .add_edge(PresenterToGroupEdge::new(member_1, group_1, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(member_2, group_2, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(group_1, group_2, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(group_2, group_3, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(group_3, group_1, false, false))
            .unwrap();

        storage.rebuild_cache();

        // Should handle complex cycle without infinite loop
        let member_1_uuid = NonNilUuid::from(member_1);
        let group_1_uuid = NonNilUuid::from(group_1);
        let group_2_uuid = NonNilUuid::from(group_2);
        let group_3_uuid = NonNilUuid::from(group_3);
        let inclusive_groups_1 = storage.get_inclusive_groups(member_1_uuid);
        assert!(inclusive_groups_1.contains(&group_1_uuid));
        assert!(inclusive_groups_1.contains(&group_2_uuid));
        assert!(inclusive_groups_1.contains(&group_3_uuid));
    }

    #[test]
    fn test_self_reference_converted_to_group_only() {
        let mut storage = PresenterToGroupStorage::new();
        let group_id = make_id(10);

        // Self-reference should be converted to group-only
        storage
            .add_edge(PresenterToGroupEdge::new(group_id, group_id, false, false))
            .unwrap();

        // Verify it became a group-only edge
        let edges: Vec<_> = storage.edges().collect();
        assert_eq!(edges.len(), 1);
        assert!(edges[0].is_group_marker());
        assert_eq!(edges[0].group_id(), group_id);
    }

    #[test]
    fn test_cycle_tolerance_simple_cycle() {
        let mut storage = PresenterToGroupStorage::new();
        let group_a = make_id(10);
        let group_b = make_id(11);
        let member = make_id(20);

        // Create a simple cycle: member -> group_a -> group_b -> member
        storage
            .add_edge(PresenterToGroupEdge::new(member, group_a, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(group_a, group_b, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(group_b, member, false, false))
            .unwrap();

        // Rebuild cache to test transitive closure with cycle
        storage.rebuild_cache();

        // Should complete without infinite loop
        let member_uuid = NonNilUuid::from(member);
        let group_a_uuid = NonNilUuid::from(group_a);
        let group_b_uuid = NonNilUuid::from(group_b);
        let inclusive_groups = storage.get_inclusive_groups(member_uuid);
        // The member should be in group_a and group_b (transitively)
        assert!(inclusive_groups.contains(&group_a_uuid));
        assert!(inclusive_groups.contains(&group_b_uuid));
    }

    #[test]
    fn test_cycle_tolerance_with_group_only() {
        let mut storage = PresenterToGroupStorage::new();
        let group_id = make_id(10);
        let member_id = make_id(20);

        // A group-only edge plus a member edge should work
        storage
            .add_edge(PresenterToGroupEdge::group_only(group_id, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(member_id, group_id, false, false))
            .unwrap();

        storage.rebuild_cache();

        // Should handle without infinite loop
        let group_uuid = NonNilUuid::from(group_id);
        let member_uuid = NonNilUuid::from(member_id);
        let inclusive_members = storage.get_inclusive_members(group_uuid);
        assert!(inclusive_members.contains(&member_uuid));
    }

    #[test]
    fn test_cycle_tolerance_complex_cycle() {
        let mut storage = PresenterToGroupStorage::new();
        let group_1 = make_id(10);
        let group_2 = make_id(11);
        let group_3 = make_id(12);
        let member_1 = make_id(20);
        let member_2 = make_id(21);

        // Create a complex cycle: group_1 -> group_2 -> group_3 -> group_1
        storage
            .add_edge(PresenterToGroupEdge::new(member_1, group_1, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(member_2, group_2, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(group_1, group_2, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(group_2, group_3, false, false))
            .unwrap();
        storage
            .add_edge(PresenterToGroupEdge::new(group_3, group_1, false, false))
            .unwrap();

        storage.rebuild_cache();

        // Should handle complex cycle without infinite loop
        let member_1_uuid = NonNilUuid::from(member_1);
        let group_1_uuid = NonNilUuid::from(group_1);
        let group_2_uuid = NonNilUuid::from(group_2);
        let group_3_uuid = NonNilUuid::from(group_3);
        let inclusive_groups_1 = storage.get_inclusive_groups(member_1_uuid);
        assert!(inclusive_groups_1.contains(&group_1_uuid));
        assert!(inclusive_groups_1.contains(&group_2_uuid));
        assert!(inclusive_groups_1.contains(&group_3_uuid));
    }

    #[test]
    fn test_trait_add_edge_does_not_recurse() {
        let mut storage = PresenterToGroupStorage::new();
        let group_id = make_id(10);
        let member_id = make_id(20);

        <PresenterToGroupStorage as EdgeStorage<PresenterToGroupEdge>>::add_edge(
            &mut storage,
            PresenterToGroupEdge::new(member_id, group_id, false, false),
        )
        .unwrap();

        let member_uuid = NonNilUuid::from(member_id);
        let group_uuid = NonNilUuid::from(group_id);
        assert!(storage.edge_exists(member_uuid, group_uuid));
    }
}
