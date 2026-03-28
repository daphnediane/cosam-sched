/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use serde::{Deserialize, Serialize};
use std::collections::btree_set::Iter;
use std::collections::{BTreeSet, HashMap, HashSet};

/// Edge representing a group/member relationship
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GroupEdge {
    /// Member name (can be empty for groups with unknown members, e.g. "G:==Group")
    pub member: String,
    /// Group name
    pub group: String,
    /// Member should always appear with group
    pub always_grouped: bool,
    /// Group should always be shown as group
    pub always_shown: bool,
}

impl GroupEdge {
    pub fn new(member: String, group: String, always_grouped: bool, always_shown: bool) -> Self {
        Self {
            member,
            group,
            always_grouped,
            always_shown,
        }
    }

    /// Create an edge for a group with unknown members (G:==Group syntax)
    pub fn group_only(group: String, always_shown: bool) -> Self {
        Self {
            member: String::new(),
            group,
            always_grouped: false,
            always_shown,
        }
    }

    /// Check if this edge represents a group with unknown members
    pub fn is_group_only(&self) -> bool {
        self.member.is_empty()
    }
}

/// Cached relationship data for fast queries
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RelationshipCache {
    /// Direct parent groups for each member
    direct_parent_groups: HashMap<String, Vec<String>>,
    /// Direct members for each group
    direct_members: HashMap<String, Vec<String>>,
    /// All members (transitive closure) for each group
    inclusive_members: HashMap<String, Vec<String>>,
    /// All groups (transitive closure) for each member
    inclusive_groups: HashMap<String, Vec<String>>,
    /// Cache version to detect invalidation
    cache_version: u64,
}

impl RelationshipCache {
    /// Clear all cached data
    fn clear(&mut self) {
        self.direct_parent_groups.clear();
        self.direct_members.clear();
        self.inclusive_members.clear();
        self.inclusive_groups.clear();
    }

    /// Get direct parent groups for a member
    pub fn get_direct_groups(&self, member: &str) -> &[String] {
        self.direct_parent_groups
            .get(member)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get direct members for a group
    pub fn get_direct_members(&self, group: &str) -> &[String] {
        self.direct_members
            .get(group)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all members (transitive) for a group
    pub fn get_inclusive_members(&self, group: &str) -> &[String] {
        self.inclusive_members
            .get(group)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all groups (transitive) for a member
    pub fn get_inclusive_groups(&self, member: &str) -> &[String] {
        self.inclusive_groups
            .get(member)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// Manages group/member relationships using edge-based storage
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RelationshipManager {
    /// All relationship edges, ordered for predictable iteration
    edges: BTreeSet<GroupEdge>,
    /// Indexes for O(1) lookups
    member_to_groups: HashMap<String, Vec<String>>,
    group_to_members: HashMap<String, Vec<String>>,
    /// Cache for relationship queries
    cache: RelationshipCache,
    /// Cache invalidation counter
    cache_invalidation: u64,
}

impl RelationshipManager {
    /// Create a new relationship manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a relationship edge
    pub fn add_edge(&mut self, edge: GroupEdge) {
        // Remove existing edge for this member-group pair if it exists
        if !edge.member.is_empty() {
            self.remove_edge(&edge.member, &edge.group);
        }

        // Add the edge
        self.edges.insert(edge.clone());

        // Update indexes
        if !edge.member.is_empty() {
            self.member_to_groups
                .entry(edge.member.clone())
                .or_default()
                .push(edge.group.clone());
        }

        self.group_to_members
            .entry(edge.group.clone())
            .or_default()
            .push(edge.member.clone());

        // Invalidate cache
        self.invalidate_cache();
    }

    /// Remove a relationship edge by member/group pair (ignoring flag values)
    pub fn remove_edge(&mut self, member: &str, group: &str) {
        // Find the actual edge (flags may differ from a default-constructed edge)
        let found = self
            .edges
            .iter()
            .find(|e| e.member == member && e.group == group)
            .cloned();

        if let Some(edge) = found {
            self.edges.remove(&edge);

            // Update indexes
            if !member.is_empty() {
                if let Some(groups) = self.member_to_groups.get_mut(member) {
                    groups.retain(|g| g != group);
                    if groups.is_empty() {
                        self.member_to_groups.remove(member);
                    }
                }
            }

            if let Some(members) = self.group_to_members.get_mut(group) {
                members.retain(|m| m != member);
                if members.is_empty() {
                    self.group_to_members.remove(group);
                }
            }

            // Invalidate cache
            self.invalidate_cache();
        }
    }

    /// Clear all members for a group
    pub fn clear_group(&mut self, group: &str) {
        // Collect members to remove
        let members_to_remove: Vec<String> = self
            .group_to_members
            .get(group)
            .map(|members| members.clone())
            .unwrap_or_default();

        // Remove each edge
        for member in &members_to_remove {
            self.remove_edge(member, group);
        }

        // Also remove group-only edges
        let group_only_edge = GroupEdge::group_only(group.to_string(), false);
        self.edges.remove(&group_only_edge);
    }

    /// Get direct parent groups for a member
    pub fn get_direct_groups(&mut self, member: &str) -> &[String] {
        self.ensure_cache_valid();
        self.cache.get_direct_groups(member)
    }

    /// Get direct members for a group
    pub fn get_direct_members(&mut self, group: &str) -> &[String] {
        self.ensure_cache_valid();
        self.cache.get_direct_members(group)
    }

    /// Get all members (transitive) for a group
    pub fn get_inclusive_members(&mut self, group: &str) -> &[String] {
        self.ensure_cache_valid();
        self.cache.get_inclusive_members(group)
    }

    /// Get all groups (transitive) for a member
    pub fn get_inclusive_groups(&mut self, member: &str) -> &[String] {
        self.ensure_cache_valid();
        self.cache.get_inclusive_groups(member)
    }

    /// Get direct parent groups for a member (non-caching, borrows `&self`).
    pub fn direct_groups_of(&self, member: &str) -> &[String] {
        self.member_to_groups
            .get(member)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get direct members for a group (non-caching, borrows `&self`).
    pub fn direct_members_of(&self, group: &str) -> &[String] {
        self.group_to_members
            .get(group)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a presenter is a group
    pub fn is_group(&self, name: &str) -> bool {
        self.group_to_members.contains_key(name)
    }

    /// Check if a member should always be grouped with a specific group
    pub fn is_always_grouped(&self, member: &str, group: &str) -> bool {
        self.edges
            .iter()
            .find(|e| e.member == member && e.group == group)
            .map(|e| e.always_grouped)
            .unwrap_or(false)
    }

    /// Check if a member should always be grouped (with any group)
    pub fn is_any_always_grouped(&self, member: &str) -> bool {
        self.edges
            .iter()
            .any(|e| e.member == member && e.always_grouped)
    }

    /// Check if a group should always be shown as a group
    pub fn is_always_shown(&self, group: &str) -> bool {
        self.edges
            .iter()
            .any(|e| e.group == group && e.always_shown)
    }

    /// Get all edges (for debugging/serialization)
    pub fn edges(&self) -> Iter<'_, GroupEdge> {
        self.edges.iter()
    }

    /// Find an edge by member/group pair, returning a reference if it exists.
    pub fn find_edge(&self, member: &str, group: &str) -> Option<&GroupEdge> {
        self.edges
            .iter()
            .find(|e| e.member == member && e.group == group)
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
            if !edge.member.is_empty() {
                self.cache
                    .direct_parent_groups
                    .entry(edge.member.clone())
                    .or_default()
                    .push(edge.group.clone());
            }

            self.cache
                .direct_members
                .entry(edge.group.clone())
                .or_default()
                .push(edge.member.clone());
        }

        // Build transitive relationships
        self.build_transitive_relationships();
    }

    /// Build transitive closure for relationships
    fn build_transitive_relationships(&mut self) {
        // For inclusive members: find all members that belong to a group, directly or indirectly
        let all_groups: Vec<String> = self.group_to_members.keys().cloned().collect();

        for group in all_groups {
            let mut inclusive_members = HashSet::new();
            let mut to_visit = vec![group.clone()];

            while let Some(current_group) = to_visit.pop() {
                if let Some(direct_members) = self.cache.direct_members.get(&current_group) {
                    for member in direct_members {
                        if !member.is_empty() && inclusive_members.insert(member.clone()) {
                            // If this member is also a group, add its members too
                            if self.group_to_members.contains_key(member) {
                                to_visit.push(member.clone());
                            }
                        }
                    }
                }
            }

            let mut members: Vec<String> = inclusive_members.into_iter().collect();
            members.sort();
            self.cache.inclusive_members.insert(group, members);
        }

        // For inclusive groups: find all groups a member belongs to, directly or indirectly
        let all_members: Vec<String> = self.member_to_groups.keys().cloned().collect();

        for member in all_members {
            let mut inclusive_groups = HashSet::new();
            let mut to_visit = vec![member.clone()];
            let mut visited = HashSet::new();

            while let Some(current) = to_visit.pop() {
                if !visited.insert(current.clone()) {
                    continue;
                }
                if let Some(direct_groups) = self.cache.direct_parent_groups.get(&current) {
                    for group in direct_groups {
                        inclusive_groups.insert(group.clone());
                        to_visit.push(group.clone());
                    }
                }
            }

            let mut groups: Vec<String> = inclusive_groups.into_iter().collect();
            groups.sort();
            self.cache.inclusive_groups.insert(member, groups);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_edge_creation() {
        let edge = GroupEdge::new("Alice".to_string(), "Team A".to_string(), true, false);
        assert_eq!(edge.member, "Alice");
        assert_eq!(edge.group, "Team A");
        assert!(edge.always_grouped);
        assert!(!edge.always_shown);
        assert!(!edge.is_group_only());

        let group_only = GroupEdge::group_only("Team B".to_string(), true);
        assert_eq!(group_only.member, "");
        assert_eq!(group_only.group, "Team B");
        assert!(group_only.is_group_only());
    }

    #[test]
    fn test_basic_relationships() {
        let mut manager = RelationshipManager::new();

        // Add a simple relationship
        manager.add_edge(GroupEdge::new(
            "Alice".to_string(),
            "Team A".to_string(),
            false,
            false,
        ));

        assert_eq!(
            manager.get_direct_groups(&"Alice".to_string()),
            <&[String]>::from(&["Team A".to_string()])
        );
        assert_eq!(
            manager.get_direct_members(&"Team A".to_string()),
            <&[String]>::from(&["Alice".to_string()])
        );
        assert!(manager.is_group("Team A"));
        assert!(!manager.is_group("Alice"));
    }

    #[test]
    fn test_group_only_edge() {
        let mut manager = RelationshipManager::new();

        // Add a group with unknown members
        manager.add_edge(GroupEdge::group_only("Team A".to_string(), true));

        assert!(manager.is_group(&"Team A".to_string()));
        assert_eq!(
            manager.get_direct_members(&"Team A".to_string()),
            <&[String]>::from(&["".to_string()])
        );
        assert!(manager.is_always_shown(&"Team A".to_string()));
    }

    #[test]
    fn test_remove_edge() {
        let mut manager = RelationshipManager::new();

        manager.add_edge(GroupEdge::new(
            "Alice".to_string(),
            "Team A".to_string(),
            false,
            false,
        ));
        assert_eq!(
            manager.get_direct_groups(&"Alice".to_string()),
            <&[String]>::from(&["Team A".to_string()])
        );

        manager.remove_edge("Alice", "Team A");
        assert_eq!(
            manager.get_direct_groups(&"Alice".to_string()),
            <&[String]>::from(&[])
        );
        assert_eq!(
            manager.get_direct_members(&"Team A".to_string()),
            <&[String]>::from(&[])
        );
        assert!(!manager.is_group(&"Team A".to_string()));
    }

    #[test]
    fn test_clear_group() {
        let mut manager = RelationshipManager::new();

        manager.add_edge(GroupEdge::new(
            "Alice".to_string(),
            "Team A".to_string(),
            false,
            false,
        ));
        manager.add_edge(GroupEdge::new(
            "Bob".to_string(),
            "Team A".to_string(),
            false,
            false,
        ));

        assert_eq!(
            manager.get_direct_members(&"Team A".to_string()),
            <&[String]>::from(&["Alice".to_string(), "Bob".to_string()])
        );

        manager.clear_group(&"Team A".to_string());
        assert_eq!(
            manager.get_direct_members(&"Team A".to_string()),
            <&[String]>::from(&[])
        );
        assert!(!manager.is_group(&"Team A".to_string()));
    }

    #[test]
    fn test_always_grouped_and_shown() {
        let mut manager = RelationshipManager::new();

        manager.add_edge(GroupEdge::new(
            "Alice".to_string(),
            "Team A".to_string(),
            true,
            false,
        ));
        manager.add_edge(GroupEdge::group_only("Team A".to_string(), true));

        assert!(manager.is_always_grouped(&"Alice".to_string(), &"Team A".to_string()));
        assert!(manager.is_always_shown(&"Team A".to_string()));
        assert!(!manager.is_always_grouped(&"Bob".to_string(), &"Team A".to_string()));
    }

    #[test]
    fn test_transitive_relationships() {
        let mut manager = RelationshipManager::new();

        // Create nested groups: Alice -> Team A -> Department
        manager.add_edge(GroupEdge::new(
            "Alice".to_string(),
            "Team A".to_string(),
            false,
            false,
        ));
        manager.add_edge(GroupEdge::new(
            "Team A".to_string(),
            "Department".to_string(),
            false,
            false,
        ));

        // Rebuild cache to test transitive relationships
        manager.rebuild_cache();

        // Alice should be in Team A's inclusive members
        assert!(
            manager
                .get_inclusive_members(&"Team A".to_string())
                .contains(&"Alice".to_string())
        );

        // Team A should be in Department's inclusive members
        assert!(
            manager
                .get_inclusive_members(&"Department".to_string())
                .contains(&"Team A".to_string())
        );

        // Alice should be in Department's inclusive members (transitive)
        assert!(
            manager
                .get_inclusive_members(&"Department".to_string())
                .contains(&"Alice".to_string())
        );

        // Alice should transitively belong to Department
        assert!(
            manager
                .get_inclusive_groups(&"Alice".to_string())
                .contains(&"Department".to_string())
        );
        // Alice should also directly belong to Team A
        assert!(
            manager
                .get_inclusive_groups(&"Alice".to_string())
                .contains(&"Team A".to_string())
        );
    }

    #[test]
    fn test_remove_edge_with_flags() {
        let mut manager = RelationshipManager::new();

        // Add edge with always_grouped=true
        manager.add_edge(GroupEdge::new(
            "Alice".to_string(),
            "Team A".to_string(),
            true,
            false,
        ));
        assert!(manager.is_always_grouped("Alice", "Team A"));

        // Remove should work even though flags differ from default
        manager.remove_edge("Alice", "Team A");
        assert!(!manager.is_group("Team A"));
        assert!(!manager.is_always_grouped("Alice", "Team A"));
        assert_eq!(manager.get_direct_groups("Alice"), &[] as &[String]);
    }

    #[test]
    fn test_is_any_always_grouped() {
        let mut manager = RelationshipManager::new();

        manager.add_edge(GroupEdge::new(
            "Alice".to_string(),
            "Team A".to_string(),
            true,
            false,
        ));
        manager.add_edge(GroupEdge::new(
            "Alice".to_string(),
            "Team B".to_string(),
            false,
            false,
        ));

        // Alice is always_grouped with Team A but not Team B
        assert!(manager.is_any_always_grouped("Alice"));
        assert!(!manager.is_any_always_grouped("Bob"));
    }

    #[test]
    fn test_cycle_tolerance() {
        let mut manager = RelationshipManager::new();

        // Create a cycle: A member of B, B member of A
        manager.add_edge(GroupEdge::new(
            "A".to_string(),
            "B".to_string(),
            false,
            false,
        ));
        manager.add_edge(GroupEdge::new(
            "B".to_string(),
            "A".to_string(),
            false,
            false,
        ));

        // Should not infinite loop — just terminate with whatever was reachable
        manager.rebuild_cache();

        // Both should see each other transitively
        assert!(
            manager
                .get_inclusive_members(&"A".to_string())
                .contains(&"B".to_string())
        );
        assert!(
            manager
                .get_inclusive_members(&"B".to_string())
                .contains(&"A".to_string())
        );
    }
}
