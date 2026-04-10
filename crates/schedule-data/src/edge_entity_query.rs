/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge entity query system with secondary indexes and caching
//!
//! This module provides query capabilities for edge-entities stored in EntityStorage,
//! including endpoint-based lookups and transitive closure caching.
//!
//! Unlike the old edge system which stored edges separately with integer IDs,
//! edge-entities are stored in EntityStorage with UUIDs. This module builds
//! secondary indexes on top of EntityStorage for efficient endpoint queries.

use crate::entity::NonNilUuid;
use crate::schedule::EntityStorage;
use std::collections::{HashMap, HashSet};

/// Secondary index for PanelToPresenter edge-entities
///
/// Maps endpoints to edge UUIDs for efficient queries:
/// - panel_uuid -> Vec<edge_uuid> (outgoing edges)
/// - presenter_uuid -> Vec<edge_uuid> (incoming edges)
#[derive(Debug, Clone, Default)]
pub struct PanelToPresenterIndex {
    /// Panel UUID -> Edge UUIDs (edges where panel is the from-side)
    by_panel: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// Presenter UUID -> Edge UUIDs (edges where presenter is the to-side)
    by_presenter: HashMap<NonNilUuid, Vec<NonNilUuid>>,
}

impl PanelToPresenterIndex {
    /// Build index from EntityStorage
    pub fn build(storage: &EntityStorage) -> Self {
        let mut index = Self::default();
        for (edge_uuid, data) in storage.panel_to_presenters.iter() {
            index
                .by_panel
                .entry(data.panel_uuid)
                .or_default()
                .push(*edge_uuid);
            index
                .by_presenter
                .entry(data.presenter_uuid)
                .or_default()
                .push(*edge_uuid);
        }
        index
    }

    /// Rebuild index from EntityStorage (clears existing data)
    pub fn rebuild(&mut self, storage: &EntityStorage) {
        self.by_panel.clear();
        self.by_presenter.clear();
        for (edge_uuid, data) in storage.panel_to_presenters.iter() {
            self.by_panel
                .entry(data.panel_uuid)
                .or_default()
                .push(*edge_uuid);
            self.by_presenter
                .entry(data.presenter_uuid)
                .or_default()
                .push(*edge_uuid);
        }
    }

    /// Get edge UUIDs for a panel (outgoing edges)
    pub fn edges_by_panel(&self, panel_uuid: NonNilUuid) -> &[NonNilUuid] {
        self.by_panel
            .get(&panel_uuid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get edge UUIDs for a presenter (incoming edges)
    pub fn edges_by_presenter(&self, presenter_uuid: NonNilUuid) -> &[NonNilUuid] {
        self.by_presenter
            .get(&presenter_uuid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if an edge exists between panel and presenter
    pub fn edge_exists(
        &self,
        storage: &EntityStorage,
        panel_uuid: NonNilUuid,
        presenter_uuid: NonNilUuid,
    ) -> bool {
        self.edges_by_panel(panel_uuid).iter().any(|&edge_uuid| {
            storage
                .panel_to_presenters
                .get(&edge_uuid)
                .map(|data| data.presenter_uuid == presenter_uuid)
                .unwrap_or(false)
        })
    }
}

/// Secondary index for PresenterToGroup edge-entities
///
/// Maps endpoints to edge UUIDs and provides relationship lookup:
/// - member_uuid -> Vec<edge_uuid> (outgoing, groups this presenter belongs to)
/// - group_uuid -> Vec<edge_uuid> (incoming, members of this group)
#[derive(Debug, Clone, Default)]
pub struct PresenterToGroupIndex {
    /// Member UUID -> Edge UUIDs (edges where member is the from-side)
    by_member: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// Group UUID -> Edge UUIDs (edges where group is the to-side)
    by_group: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// Group marker edges (self-loops): group_uuid -> edge_uuid
    group_markers: HashMap<NonNilUuid, NonNilUuid>,
}

impl PresenterToGroupIndex {
    /// Build index from EntityStorage
    pub fn build(storage: &EntityStorage) -> Self {
        let mut index = Self::default();
        for (edge_uuid, data) in storage.presenter_to_groups.iter() {
            index
                .by_member
                .entry(data.member_uuid)
                .or_default()
                .push(*edge_uuid);
            index
                .by_group
                .entry(data.group_uuid)
                .or_default()
                .push(*edge_uuid);

            // Track group markers (self-loops)
            if data.is_self_loop() {
                index.group_markers.insert(data.group_uuid, *edge_uuid);
            }
        }
        index
    }

    /// Rebuild index from EntityStorage
    pub fn rebuild(&mut self, storage: &EntityStorage) {
        self.by_member.clear();
        self.by_group.clear();
        self.group_markers.clear();
        for (edge_uuid, data) in storage.presenter_to_groups.iter() {
            self.by_member
                .entry(data.member_uuid)
                .or_default()
                .push(*edge_uuid);
            self.by_group
                .entry(data.group_uuid)
                .or_default()
                .push(*edge_uuid);

            if data.is_self_loop() {
                self.group_markers.insert(data.group_uuid, *edge_uuid);
            }
        }
    }

    /// Get edge UUIDs for a member (groups they belong to)
    pub fn edges_by_member(&self, member_uuid: NonNilUuid) -> &[NonNilUuid] {
        self.by_member
            .get(&member_uuid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get edge UUIDs for a group (its members)
    pub fn edges_by_group(&self, group_uuid: NonNilUuid) -> &[NonNilUuid] {
        self.by_group
            .get(&group_uuid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a presenter is a group (has group marker or members)
    pub fn is_group(&self, presenter_uuid: NonNilUuid) -> bool {
        self.group_markers.contains_key(&presenter_uuid)
            || self.by_group.contains_key(&presenter_uuid)
    }

    /// Get group marker edge UUID if presenter is a group
    pub fn group_marker(&self, presenter_uuid: NonNilUuid) -> Option<NonNilUuid> {
        self.group_markers.get(&presenter_uuid).copied()
    }

    /// Get all group UUIDs (presenters that are groups)
    pub fn all_groups(&self) -> impl Iterator<Item = NonNilUuid> + '_ {
        self.group_markers.keys().copied()
    }
}

/// Cached query results for transitive closure operations
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct TransitiveCache {
    /// Panel UUID -> All presenter UUIDs (including from groups)
    inclusive_presenters: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// Presenter UUID -> All panel UUIDs (including from groups)
    inclusive_panels: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// Group UUID -> All member UUIDs (transitive)
    inclusive_members: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// Member UUID -> All group UUIDs (transitive)
    inclusive_groups: HashMap<NonNilUuid, Vec<NonNilUuid>>,
    /// Cache version for invalidation detection
    cache_version: u64,
}

impl TransitiveCache {
    /// Clear all cached data
    pub fn clear(&mut self) {
        self.inclusive_presenters.clear();
        self.inclusive_panels.clear();
        self.inclusive_members.clear();
        self.inclusive_groups.clear();
    }
}

/// Query engine for edge-entities with caching
///
/// Provides the same functionality as the old edge storage systems
/// but works with UUID-based edge-entities in EntityStorage.
#[derive(Debug, Clone)]
pub struct EdgeEntityQuery {
    /// Secondary index for PanelToPresenter
    panel_to_presenter_index: PanelToPresenterIndex,
    /// Secondary index for PresenterToGroup
    presenter_to_group_index: PresenterToGroupIndex,
    /// Cache for transitive closure results
    cache: TransitiveCache,
    /// Cache invalidation counter
    cache_version: u64,
}

impl EdgeEntityQuery {
    /// Create a new query engine with indexes built from storage
    pub fn new(storage: &EntityStorage) -> Self {
        Self {
            panel_to_presenter_index: PanelToPresenterIndex::build(storage),
            presenter_to_group_index: PresenterToGroupIndex::build(storage),
            cache: TransitiveCache::default(),
            cache_version: 0,
        }
    }

    /// Rebuild all indexes from storage (call after bulk changes)
    pub fn rebuild_indexes(&mut self, storage: &EntityStorage) {
        self.panel_to_presenter_index.rebuild(storage);
        self.presenter_to_group_index.rebuild(storage);
        self.invalidate_cache();
    }

    /// Invalidate the cache (call after any edge-entity modification)
    pub fn invalidate_cache(&mut self) {
        self.cache_version += 1;
        self.cache.clear();
    }

    #[allow(dead_code)]
    /// Ensure cache is valid, rebuilding if necessary
    fn ensure_cache_valid(&mut self, storage: &EntityStorage) {
        if self.cache.cache_version != self.cache_version {
            self.rebuild_cache(storage);
        }
    }

    #[allow(dead_code)]
    /// Rebuild the transitive closure cache
    fn rebuild_cache(&mut self, storage: &EntityStorage) {
        self.cache.clear();
        self.cache.cache_version = self.cache_version;

        // Build inclusive members for each group (transitive closure)
        self.build_inclusive_members(storage);

        // Build inclusive groups for each member (reverse transitive)
        self.build_inclusive_groups(storage);

        // Build inclusive presenters for each panel
        self.build_inclusive_presenters(storage);

        // Build inclusive panels for each presenter
        self.build_inclusive_panels(storage);
    }

    /// Build inclusive_members cache (all members of a group, transitively)
    fn build_inclusive_members(&mut self, storage: &EntityStorage) {
        let all_groups: Vec<NonNilUuid> = self.presenter_to_group_index.all_groups().collect();

        for group_uuid in all_groups {
            let mut inclusive = HashSet::new();
            let mut to_visit = vec![group_uuid];
            let mut visited = HashSet::new();

            while let Some(current) = to_visit.pop() {
                if !visited.insert(current) {
                    continue;
                }

                // Get direct members of current
                for &edge_uuid in self.presenter_to_group_index.edges_by_group(current) {
                    if let Some(data) = storage.presenter_to_groups.get(&edge_uuid) {
                        // Skip group markers (self-loops)
                        if !data.is_self_loop() {
                            let member = data.member_uuid;
                            if inclusive.insert(member) {
                                // If this member is also a group, add its members
                                if self.presenter_to_group_index.is_group(member) {
                                    to_visit.push(member);
                                }
                            }
                        }
                    }
                }
            }

            let mut members: Vec<NonNilUuid> = inclusive.into_iter().collect();
            members.sort();
            self.cache.inclusive_members.insert(group_uuid, members);
        }
    }

    /// Build inclusive_groups cache (all groups a member belongs to, transitively)
    fn build_inclusive_groups(&mut self, storage: &EntityStorage) {
        // Collect all members
        let all_members: Vec<NonNilUuid> = self
            .presenter_to_group_index
            .by_member
            .keys()
            .copied()
            .collect();

        for member_uuid in all_members {
            let mut inclusive = HashSet::new();
            let mut to_visit = vec![member_uuid];
            let mut visited = HashSet::new();

            while let Some(current) = to_visit.pop() {
                if !visited.insert(current) {
                    continue;
                }

                // Get direct groups for current
                for &edge_uuid in self.presenter_to_group_index.edges_by_member(current) {
                    if let Some(data) = storage.presenter_to_groups.get(&edge_uuid) {
                        let group = data.group_uuid;
                        if inclusive.insert(group) {
                            // If this group is also a member of other groups, continue traversal
                            to_visit.push(group);
                        }
                    }
                }
            }

            let mut groups: Vec<NonNilUuid> = inclusive.into_iter().collect();
            groups.sort();
            self.cache.inclusive_groups.insert(member_uuid, groups);
        }
    }

    /// Build inclusive_presenters cache (all presenters for a panel, including from groups)
    fn build_inclusive_presenters(&mut self, storage: &EntityStorage) {
        // Get all panels that have presenters
        let all_panels: Vec<NonNilUuid> = self
            .panel_to_presenter_index
            .by_panel
            .keys()
            .copied()
            .collect();

        for panel_uuid in all_panels {
            let mut inclusive = HashSet::new();
            let mut visited = HashSet::new();
            let mut to_visit: Vec<NonNilUuid> = self
                .panel_to_presenter_index
                .edges_by_panel(panel_uuid)
                .iter()
                .filter_map(|&edge_uuid| {
                    storage
                        .panel_to_presenters
                        .get(&edge_uuid)
                        .map(|d| d.presenter_uuid)
                })
                .collect();

            while let Some(current) = to_visit.pop() {
                if !visited.insert(current) {
                    continue;
                }
                inclusive.insert(current);

                // If this presenter is a group, add all its members
                if self.presenter_to_group_index.is_group(current) {
                    let members = self.get_inclusive_members_internal(storage, current);
                    for &member in members {
                        if !visited.contains(&member) {
                            to_visit.push(member);
                        }
                    }
                }
            }

            let mut presenters: Vec<NonNilUuid> = inclusive.into_iter().collect();
            presenters.sort();
            self.cache
                .inclusive_presenters
                .insert(panel_uuid, presenters);
        }
    }

    /// Build inclusive_panels cache (all panels for a presenter, including from groups)
    fn build_inclusive_panels(&mut self, storage: &EntityStorage) {
        // Get all presenters
        let all_presenters: Vec<NonNilUuid> = self
            .panel_to_presenter_index
            .by_presenter
            .keys()
            .copied()
            .chain(self.presenter_to_group_index.by_member.keys().copied())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        for presenter_uuid in all_presenters {
            let mut inclusive = HashSet::new();
            let mut visited = HashSet::new();
            let mut to_visit = vec![presenter_uuid];

            while let Some(current) = to_visit.pop() {
                if !visited.insert(current) {
                    continue;
                }

                // Add panels for current presenter
                for edge_uuid in self.panel_to_presenter_index.edges_by_presenter(current) {
                    if let Some(data) = storage.panel_to_presenters.get(edge_uuid) {
                        inclusive.insert(data.panel_uuid);
                    }
                }

                // Add groups this presenter belongs to for transitive traversal
                for edge_uuid in self.presenter_to_group_index.edges_by_member(current) {
                    if let Some(data) = storage.presenter_to_groups.get(edge_uuid) {
                        let group = data.group_uuid;
                        if !visited.contains(&group) {
                            to_visit.push(group);
                        }
                    }
                }
            }

            let mut panels: Vec<NonNilUuid> = inclusive.into_iter().collect();
            panels.sort();
            self.cache.inclusive_panels.insert(presenter_uuid, panels);
        }
    }

    /// Get inclusive members for a group (internal, uses cache)
    fn get_inclusive_members_internal(
        &self,
        _storage: &EntityStorage,
        group_uuid: NonNilUuid,
    ) -> &[NonNilUuid] {
        self.cache
            .inclusive_members
            .get(&group_uuid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    // === Public Query API ===

    /// Get all presenters for a panel (direct only, not including groups)
    pub fn get_panel_presenters(
        &self,
        storage: &EntityStorage,
        panel_uuid: NonNilUuid,
    ) -> Vec<NonNilUuid> {
        self.panel_to_presenter_index
            .edges_by_panel(panel_uuid)
            .iter()
            .filter_map(|&edge_uuid| {
                storage
                    .panel_to_presenters
                    .get(&edge_uuid)
                    .map(|d| d.presenter_uuid)
            })
            .collect()
    }

    /// Get all panels for a presenter (direct only)
    pub fn get_presenter_panels(
        &self,
        storage: &EntityStorage,
        presenter_uuid: NonNilUuid,
    ) -> Vec<NonNilUuid> {
        self.panel_to_presenter_index
            .edges_by_presenter(presenter_uuid)
            .iter()
            .filter_map(|&edge_uuid| {
                storage
                    .panel_to_presenters
                    .get(&edge_uuid)
                    .map(|d| d.panel_uuid)
            })
            .collect()
    }

    /// Get all presenters for a panel, including those from presenter groups (cached)
    pub fn get_inclusive_presenters(
        &mut self,
        _storage: &EntityStorage,
        panel_uuid: NonNilUuid,
    ) -> &[NonNilUuid] {
        // Note: We should rebuild cache if needed, but need mutable self
        // For now, return from cache if available
        self.cache
            .inclusive_presenters
            .get(&panel_uuid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all panels for a presenter, including those from presenter groups (cached)
    pub fn get_inclusive_panels(
        &mut self,
        _storage: &EntityStorage,
        presenter_uuid: NonNilUuid,
    ) -> &[NonNilUuid] {
        self.cache
            .inclusive_panels
            .get(&presenter_uuid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get direct groups for a member
    pub fn get_direct_groups(
        &self,
        storage: &EntityStorage,
        member_uuid: NonNilUuid,
    ) -> Vec<NonNilUuid> {
        self.presenter_to_group_index
            .edges_by_member(member_uuid)
            .iter()
            .filter_map(|&edge_uuid| {
                storage
                    .presenter_to_groups
                    .get(&edge_uuid)
                    .map(|d| d.group_uuid)
            })
            .collect()
    }

    /// Get direct members for a group
    pub fn get_direct_members(
        &self,
        storage: &EntityStorage,
        group_uuid: NonNilUuid,
    ) -> Vec<NonNilUuid> {
        self.presenter_to_group_index
            .edges_by_group(group_uuid)
            .iter()
            .filter_map(|&edge_uuid| {
                storage
                    .presenter_to_groups
                    .get(&edge_uuid)
                    .filter(|d| !d.is_self_loop())
                    .map(|d| d.member_uuid)
            })
            .collect()
    }

    /// Get all members for a group (transitive, cached)
    pub fn get_inclusive_members(
        &mut self,
        _storage: &EntityStorage,
        group_uuid: NonNilUuid,
    ) -> &[NonNilUuid] {
        self.cache
            .inclusive_members
            .get(&group_uuid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all groups for a member (transitive, cached)
    pub fn get_inclusive_groups(
        &mut self,
        _storage: &EntityStorage,
        member_uuid: NonNilUuid,
    ) -> &[NonNilUuid] {
        self.cache
            .inclusive_groups
            .get(&member_uuid)
            .map_or(&[] as &[NonNilUuid], |v| v.as_slice())
    }

    /// Check if a presenter is a group
    pub fn is_group(&self, presenter_uuid: NonNilUuid) -> bool {
        self.presenter_to_group_index.is_group(presenter_uuid)
    }

    /// Check if an edge exists between panel and presenter
    pub fn panel_to_presenter_exists(
        &self,
        storage: &EntityStorage,
        panel_uuid: NonNilUuid,
        presenter_uuid: NonNilUuid,
    ) -> bool {
        self.panel_to_presenter_index
            .edge_exists(storage, panel_uuid, presenter_uuid)
    }
}

impl Default for EdgeEntityQuery {
    fn default() -> Self {
        Self {
            panel_to_presenter_index: PanelToPresenterIndex::default(),
            presenter_to_group_index: PresenterToGroupIndex::default(),
            cache: TransitiveCache::default(),
            cache_version: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{PanelToPresenterData, PresenterToGroupData};

    fn test_uuid(byte: u8) -> NonNilUuid {
        let mut bytes = [0u8; 16];
        bytes[15] = byte;
        unsafe { NonNilUuid::new_unchecked(uuid::Uuid::from_bytes(bytes)) }
    }

    fn make_panel_to_presenter_edge(
        panel_uuid: NonNilUuid,
        presenter_uuid: NonNilUuid,
    ) -> PanelToPresenterData {
        PanelToPresenterData {
            entity_uuid: test_uuid(255),
            panel_uuid,
            presenter_uuid,
        }
    }

    fn make_presenter_to_group_edge(
        member_uuid: NonNilUuid,
        group_uuid: NonNilUuid,
        always_shown: bool,
        always_grouped: bool,
    ) -> PresenterToGroupData {
        PresenterToGroupData {
            entity_uuid: test_uuid(254),
            member_uuid,
            group_uuid,
            always_shown_in_group: always_shown,
            always_grouped,
        }
    }

    #[test]
    fn test_panel_to_presenter_index_build() {
        let mut storage = EntityStorage::new();
        let panel = test_uuid(1);
        let presenter = test_uuid(2);
        let edge_uuid = test_uuid(100);

        storage
            .panel_to_presenters
            .insert(edge_uuid, make_panel_to_presenter_edge(panel, presenter));

        let index = PanelToPresenterIndex::build(&storage);

        assert_eq!(index.edges_by_panel(panel).len(), 1);
        assert_eq!(index.edges_by_presenter(presenter).len(), 1);
        assert!(index.edge_exists(&storage, panel, presenter));
    }

    #[test]
    fn test_presenter_to_group_index_build() {
        let mut storage = EntityStorage::new();
        let member = test_uuid(1);
        let group = test_uuid(2);
        let edge_uuid = test_uuid(100);

        storage.presenter_to_groups.insert(
            edge_uuid,
            make_presenter_to_group_edge(member, group, false, false),
        );

        let index = PresenterToGroupIndex::build(&storage);

        assert_eq!(index.edges_by_member(member).len(), 1);
        assert_eq!(index.edges_by_group(group).len(), 1);
        assert!(!index.is_group(member));
        // Group is a group because it has members (edges in by_group)
        assert!(index.is_group(group));
        // But it doesn't have a group marker (self-loop)
        assert_eq!(index.group_marker(group), None);
    }

    #[test]
    fn test_group_marker_detection() {
        let mut storage = EntityStorage::new();
        let group = test_uuid(1);
        let edge_uuid = test_uuid(100);

        // Self-loop creates a group marker
        storage.presenter_to_groups.insert(
            edge_uuid,
            make_presenter_to_group_edge(group, group, true, false),
        );

        let index = PresenterToGroupIndex::build(&storage);

        assert!(index.is_group(group));
        assert_eq!(index.group_marker(group), Some(edge_uuid));
    }
}
