/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge storage implementations for schedule-data relationships

use crate::edge::{Edge, EdgeError, EdgeId, RelationshipStorage};
use crate::entity::EntityId;
use chrono::NaiveDateTime;
use std::collections::{BTreeSet, HashMap, HashSet};

/// Generic edge storage for simple edges
#[derive(Debug, Clone)]
pub struct EdgeStorage<E: Edge> {
    edges: HashMap<EdgeId, E>,
    outgoing_index: HashMap<EntityId, Vec<EdgeId>>,
    incoming_index: HashMap<EntityId, Vec<EdgeId>>,
    next_id: u64,
}

impl<E: Edge> EdgeStorage<E> {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            outgoing_index: HashMap::new(),
            incoming_index: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add an edge to storage
    pub fn add_edge(&mut self, edge: E) -> Result<EdgeId, EdgeError> {
        let edge_id = EdgeId(self.next_id);
        self.next_id += 1;

        let from_id = edge.from_id();
        let to_id = edge.to_id();

        // Check for duplicates
        if self.edge_exists(&from_id, &to_id) {
            return Err(EdgeError::DuplicateEdge {
                from_id: from_id.to_string(),
                to_id: to_id.to_string(),
            });
        }

        // Add to main storage
        self.edges.insert(edge_id, edge);

        // Update indexes
        self.outgoing_index
            .entry(from_id)
            .or_default()
            .push(edge_id);

        self.incoming_index.entry(to_id).or_default().push(edge_id);

        Ok(edge_id)
    }

    /// Remove an edge from storage
    pub fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), EdgeError> {
        if let Some(edge) = self.edges.remove(&edge_id) {
            let from_id = edge.from_id();
            let to_id = edge.to_id();

            // Remove from outgoing index
            if let Some(edges) = self.outgoing_index.get_mut(&from_id) {
                edges.retain(|&id| id != edge_id);
                if edges.is_empty() {
                    self.outgoing_index.remove(&from_id);
                }
            }

            // Remove from incoming index
            if let Some(edges) = self.incoming_index.get_mut(&to_id) {
                edges.retain(|&id| id != edge_id);
                if edges.is_empty() {
                    self.incoming_index.remove(&to_id);
                }
            }

            Ok(())
        } else {
            Err(EdgeError::EdgeNotFound {
                edge_id: edge_id.to_string(),
            })
        }
    }

    /// Get an edge by ID
    pub fn get_edge(&self, edge_id: EdgeId) -> Option<&E> {
        self.edges.get(&edge_id)
    }

    /// Get an edge by ID (mutable)
    pub fn get_edge_mut(&mut self, edge_id: EdgeId) -> Option<&mut E> {
        self.edges.get_mut(&edge_id)
    }

    /// Find outgoing edges from an entity
    pub fn find_outgoing(&self, from_id: EntityId) -> Vec<&E> {
        self.outgoing_index
            .get(&from_id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|&edge_id| self.edges.get(&edge_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find incoming edges to an entity
    pub fn find_incoming(&self, to_id: EntityId) -> Vec<&E> {
        self.incoming_index
            .get(&to_id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|&edge_id| self.edges.get(&edge_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if an edge exists between two entities
    pub fn edge_exists(&self, from_id: &EntityId, to_id: &EntityId) -> bool {
        self.outgoing_index
            .get(from_id)
            .map(|edge_ids| {
                edge_ids.iter().any(|&edge_id| {
                    self.edges
                        .get(&edge_id)
                        .map(|edge| edge.to_id() == *to_id)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }

    /// Get all edges
    pub fn all_edges(&self) -> impl Iterator<Item = &E> {
        self.edges.values()
    }

    /// Get the number of edges
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    /// Check if storage is empty
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

impl<E: Edge> Default for EdgeStorage<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached relationship data for fast queries
#[derive(Debug, Default, Clone)]
pub struct RelationshipCache {
    /// Direct parent groups for each member
    direct_parent_groups: HashMap<EntityId, Vec<EntityId>>,
    /// Direct members for each group
    direct_members: HashMap<EntityId, Vec<EntityId>>,
    /// All members (transitive) for each group
    inclusive_members: HashMap<EntityId, Vec<EntityId>>,
    /// All groups (transitive) for each member
    inclusive_groups: HashMap<EntityId, Vec<EntityId>>,
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
    pub fn get_direct_groups(&self, member_id: &EntityId) -> &[EntityId] {
        self.direct_parent_groups
            .get(member_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get direct members for a group
    pub fn get_direct_members(&self, group_id: &EntityId) -> &[EntityId] {
        self.direct_members
            .get(group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all members (transitive) for a group
    pub fn get_inclusive_members(&self, group_id: &EntityId) -> &[EntityId] {
        self.inclusive_members
            .get(group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all groups (transitive) for a member
    pub fn get_inclusive_groups(&self, member_id: &EntityId) -> &[EntityId] {
        self.inclusive_groups
            .get(member_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// Specialized storage for PresenterMemberToGroup with relationship manager features
#[derive(Debug, Clone)]
pub struct PresenterMemberToGroupStorage {
    edges: BTreeSet<PresenterMemberToGroupEdge>,
    member_to_groups: HashMap<EntityId, Vec<EntityId>>,
    group_to_members: HashMap<EntityId, Vec<EntityId>>,
    cache: RelationshipCache,
    cache_invalidation: u64,
    next_id: u64,
}

/// Edge representing a group/member relationship (based on GroupEdge from schedule-core)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PresenterMemberToGroupEdge {
    /// Member ID (can be 0 for groups with unknown members, e.g. "G:==Group")
    pub member_id: EntityId,
    /// Group ID
    pub group_id: EntityId,
    /// Member should always appear with group
    pub always_grouped: bool,
    /// Group should always be shown as group
    pub always_shown_in_group: bool,
    /// Creation timestamp
    pub created_at: NaiveDateTime,
    /// Last update timestamp
    pub updated_at: NaiveDateTime,
}

impl PresenterMemberToGroupEdge {
    /// Create a new presenter-group edge
    pub fn new(
        member_id: EntityId,
        group_id: EntityId,
        always_grouped: bool,
        always_shown_in_group: bool,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            member_id,
            group_id,
            always_grouped,
            always_shown_in_group,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create an edge for a group with unknown members (G:==Group syntax)
    pub fn group_only(group_id: EntityId, always_shown_in_group: bool) -> Self {
        Self::new(0, group_id, false, always_shown_in_group)
    }

    /// Check if this edge represents a group with unknown members
    pub fn is_group_only(&self) -> bool {
        self.member_id == 0
    }

    /// Update the edge's timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}

impl PresenterMemberToGroupStorage {
    /// Create a new relationship manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a relationship edge
    pub fn add_edge(&mut self, edge: PresenterMemberToGroupEdge) -> Result<EdgeId, EdgeError> {
        let edge_id = EdgeId(self.next_id);
        self.next_id += 1;

        // Remove existing edge for this member-group pair if it exists
        if !edge.is_group_only() {
            self.remove_edge_internal(&edge.member_id, &edge.group_id);
        }

        // Add the edge
        self.edges.insert(edge.clone());

        // Update indexes
        if !edge.is_group_only() {
            self.member_to_groups
                .entry(edge.member_id)
                .or_default()
                .push(edge.group_id);
        }

        self.group_to_members
            .entry(edge.group_id)
            .or_default()
            .push(edge.member_id);

        // Invalidate cache
        self.invalidate_cache();

        Ok(edge_id)
    }

    /// Remove a relationship edge by member/group pair (ignoring flag values)
    pub fn remove_edge(
        &mut self,
        member_id: EntityId,
        group_id: EntityId,
    ) -> Result<(), EdgeError> {
        self.remove_edge_internal(&member_id, &group_id);
        self.invalidate_cache();
        Ok(())
    }

    /// Internal edge removal without cache invalidation
    fn remove_edge_internal(&mut self, member_id: &EntityId, group_id: &EntityId) {
        // Find the actual edge (flags may differ from a default-constructed edge)
        let found = self
            .edges
            .iter()
            .find(|e| e.member_id == *member_id && e.group_id == *group_id)
            .cloned();

        if let Some(edge) = found {
            self.edges.remove(&edge);

            // Update indexes
            if !edge.is_group_only() {
                if let Some(groups) = self.member_to_groups.get_mut(&edge.member_id) {
                    groups.retain(|&g| g != edge.group_id);
                    if groups.is_empty() {
                        self.member_to_groups.remove(&edge.member_id);
                    }
                }
            }

            if let Some(members) = self.group_to_members.get_mut(&edge.group_id) {
                members.retain(|&m| m != edge.member_id);
                if members.is_empty() {
                    self.group_to_members.remove(&edge.group_id);
                }
            }
        }
    }

    /// Clear all members for a group
    pub fn clear_group(&mut self, group_id: EntityId) {
        // Collect members to remove
        let members_to_remove: Vec<EntityId> = self
            .group_to_members
            .get(&group_id)
            .map(|members| members.clone())
            .unwrap_or_default();

        // Remove each edge
        for member_id in &members_to_remove {
            self.remove_edge_internal(member_id, &group_id);
        }

        // Also remove group-only edges
        let group_only_edge = PresenterMemberToGroupEdge::group_only(group_id, false);
        self.edges.remove(&group_only_edge);

        self.invalidate_cache();
    }

    /// Get all members (transitive) for a group
    pub fn get_inclusive_members(&mut self, group_id: EntityId) -> &[EntityId] {
        self.ensure_cache_valid();
        self.cache.get_inclusive_members(&group_id)
    }

    /// Get all groups (transitive) for a member
    pub fn get_inclusive_groups(&mut self, member_id: EntityId) -> &[EntityId] {
        self.ensure_cache_valid();
        self.cache.get_inclusive_groups(&member_id)
    }

    /// Get direct parent groups for a member (non-caching, borrows `&self`).
    pub fn direct_groups_of(&self, member_id: EntityId) -> &[EntityId] {
        self.member_to_groups
            .get(&member_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get direct members for a group (non-caching, borrows `&self`).
    pub fn direct_members_of(&self, group_id: EntityId) -> &[EntityId] {
        self.group_to_members
            .get(&group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a presenter is a group
    pub fn is_group(&self, presenter_id: EntityId) -> bool {
        self.group_to_members.contains_key(&presenter_id)
    }

    /// Check if a member should always be grouped with a specific group
    pub fn is_always_grouped(&self, member_id: EntityId, group_id: EntityId) -> bool {
        self.edges
            .iter()
            .find(|e| e.member_id == member_id && e.group_id == group_id)
            .map(|e| e.always_grouped)
            .unwrap_or(false)
    }

    /// Check if a member should always be grouped (with any group)
    pub fn is_any_always_grouped(&self, member_id: EntityId) -> bool {
        self.edges
            .iter()
            .any(|e| e.member_id == member_id && e.always_grouped)
    }

    /// Check if a group should always be shown as a group
    pub fn is_always_shown_in_group(&self, group_id: EntityId) -> bool {
        self.edges
            .iter()
            .any(|e| e.group_id == group_id && e.always_shown_in_group)
    }

    /// Get all edges (for debugging/serialization)
    pub fn edges(&self) -> impl Iterator<Item = &PresenterMemberToGroupEdge> {
        self.edges.iter()
    }

    /// Find an edge by member/group pair, returning a reference if it exists.
    pub fn find_edge(
        &self,
        member_id: EntityId,
        group_id: EntityId,
    ) -> Option<&PresenterMemberToGroupEdge> {
        self.edges
            .iter()
            .find(|e| e.member_id == member_id && e.group_id == group_id)
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
            if !edge.is_group_only() {
                self.cache
                    .direct_parent_groups
                    .entry(edge.member_id)
                    .or_default()
                    .push(edge.group_id);
            }

            self.cache
                .direct_members
                .entry(edge.group_id)
                .or_default()
                .push(edge.member_id);
        }

        // Build transitive relationships
        self.build_transitive_relationships();
    }

    /// Build transitive closure for relationships
    fn build_transitive_relationships(&mut self) {
        // For inclusive members: find all members that belong to a group, directly or indirectly
        let all_groups: Vec<EntityId> = self.group_to_members.keys().cloned().collect();

        for group in all_groups {
            let mut inclusive_members = HashSet::new();
            let mut to_visit = vec![group];

            while let Some(current_group) = to_visit.pop() {
                if let Some(direct_members) = self.cache.direct_members.get(&current_group) {
                    for member in direct_members {
                        if *member != 0 && inclusive_members.insert(*member) {
                            // If this member is also a group, add its members too
                            if self.group_to_members.contains_key(member) {
                                to_visit.push(*member);
                            }
                        }
                    }
                }
            }

            let mut members: Vec<EntityId> = inclusive_members.into_iter().collect();
            members.sort();
            self.cache.inclusive_members.insert(group, members);
        }

        // For inclusive groups: find all groups a member belongs to, directly or indirectly
        let all_members: Vec<EntityId> = self.member_to_groups.keys().cloned().collect();

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

            let mut groups: Vec<EntityId> = inclusive_groups.into_iter().collect();
            groups.sort();
            self.cache.inclusive_groups.insert(member, groups);
        }
    }
}

impl Default for PresenterMemberToGroupStorage {
    fn default() -> Self {
        Self::new()
    }
}

// Implement RelationshipStorage trait for PresenterMemberToGroupStorage
impl RelationshipStorage for PresenterMemberToGroupStorage {
    fn get_inclusive_members(&self, group_id: EntityId) -> &[EntityId] {
        // Note: This returns a reference to cached data
        // In practice, the caller should ensure cache is valid first
        self.cache
            .inclusive_members
            .get(&group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    fn get_inclusive_groups(&self, member_id: EntityId) -> &[EntityId] {
        // Note: This returns a reference to cached data
        // In practice, the caller should ensure cache is valid first
        self.cache
            .inclusive_groups
            .get(&member_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    fn is_group(&self, presenter_id: EntityId) -> bool {
        self.group_to_members.contains_key(&presenter_id)
    }

    fn is_always_grouped(&self, member_id: EntityId, group_id: EntityId) -> bool {
        self.edges
            .iter()
            .find(|e| e.member_id == member_id && e.group_id == group_id)
            .map(|e| e.always_grouped)
            .unwrap_or(false)
    }

    fn is_always_shown_in_group(&self, group_id: EntityId) -> bool {
        self.edges
            .iter()
            .any(|e| e.group_id == group_id && e.always_shown_in_group)
    }
}
