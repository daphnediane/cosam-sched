/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToPresenter edge implementation

use crate::edge::generic::GenericEdgeStorage;
use crate::edge::{Edge, EdgeError, EdgeId, EdgeStorage};
use crate::entity::EntityId;

/// PanelToPresenter edge implementation
#[derive(Debug, Clone)]
pub struct PanelToPresenterEdge {
    pub from_id: EntityId, // Panel
    pub to_id: EntityId,   // Presenter
    pub data: PanelToPresenterData,
}

#[derive(Debug, Clone)]
pub struct PanelToPresenterData {
    // No additional data needed for this simple relationship
}

impl PanelToPresenterEdge {
    pub fn new(panel_id: EntityId, presenter_id: EntityId) -> Self {
        Self {
            from_id: panel_id,
            to_id: presenter_id,
            data: PanelToPresenterData {},
        }
    }
}

impl Edge for PanelToPresenterEdge {
    type FromEntity = crate::entity::PanelEntityType;
    type ToEntity = crate::entity::PresenterEntityType;
    type Data = PanelToPresenterData;

    fn from_id(&self) -> Option<EntityId> {
        Some(self.from_id)
    }

    fn to_id(&self) -> Option<EntityId> {
        Some(self.to_id)
    }

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.data
    }

    fn edge_type(&self) -> crate::edge::EdgeType {
        crate::edge::EdgeType::PanelToPresenter
    }
}

/// Specialized storage for PanelToPresenter with caching
#[derive(Debug, Clone)]
pub struct PanelToPresenterStorage {
    edges: GenericEdgeStorage<PanelToPresenterEdge>,
    inclusive_presenters: std::collections::HashMap<EntityId, Vec<EntityId>>,
    inclusive_panels: std::collections::HashMap<EntityId, Vec<EntityId>>,
    cache_invalidation: u64,
}

impl PanelToPresenterStorage {
    pub fn new() -> Self {
        Self {
            edges: GenericEdgeStorage::new(),
            inclusive_presenters: std::collections::HashMap::new(),
            inclusive_panels: std::collections::HashMap::new(),
            cache_invalidation: 0,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.cache_invalidation += 1;
        self.inclusive_presenters.clear();
        self.inclusive_panels.clear();
    }

    /// Get all presenters for a panel, including those from presenter groups (transitive closure)
    pub fn get_inclusive_presenters(
        &mut self,
        panel_id: EntityId,
        group_storage: &mut super::presenter_to_group::PresenterToGroupStorage,
    ) -> &[EntityId] {
        if self.inclusive_presenters.contains_key(&panel_id) {
            return self.inclusive_presenters.get(&panel_id).unwrap();
        }

        let direct_presenters: Vec<EntityId> = self
            .edges
            .find_outgoing(panel_id)
            .iter()
            .filter_map(|e| e.to_id())
            .collect();

        let mut inclusive = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut to_visit = direct_presenters;

        while let Some(current) = to_visit.pop() {
            if !visited.insert(current) {
                continue;
            }
            inclusive.push(current);
            // Collect groups first to avoid borrow conflicts
            let groups: Vec<EntityId> = group_storage.direct_groups_of(current).to_vec();
            for group_id in groups {
                for member_id in group_storage
                    .get_inclusive_members(group_id)
                    .iter()
                    .copied()
                {
                    to_visit.push(member_id);
                }
            }
        }

        self.inclusive_presenters.insert(panel_id, inclusive);
        self.inclusive_presenters.get(&panel_id).unwrap()
    }

    /// Get all panels for a presenter, including those from presenter groups (transitive closure)
    pub fn get_inclusive_panels(
        &mut self,
        presenter_id: EntityId,
        group_storage: &mut super::presenter_to_group::PresenterToGroupStorage,
    ) -> &[EntityId] {
        if self.inclusive_panels.contains_key(&presenter_id) {
            return self.inclusive_panels.get(&presenter_id).unwrap();
        }

        let direct_panels: Vec<EntityId> = self
            .edges
            .find_incoming(presenter_id)
            .iter()
            .map(|e| e.from_id)
            .collect();

        let mut inclusive = direct_panels;
        let mut visited = std::collections::HashSet::new();
        let mut to_visit = vec![presenter_id];

        while let Some(current) = to_visit.pop() {
            if !visited.insert(current) {
                continue;
            }
            // Add panels for the current presenter
            for panel_id in self.edges.find_incoming(current).iter().map(|e| e.from_id) {
                if !inclusive.contains(&panel_id) {
                    inclusive.push(panel_id);
                }
            }
            // Also get panels for all groups this presenter is a member of
            for group_id in group_storage.direct_groups_of(current).iter().copied() {
                for member_id in group_storage.direct_members_of(group_id).iter().copied() {
                    if !visited.contains(&member_id) {
                        to_visit.push(member_id);
                    }
                }
            }
        }

        self.inclusive_panels.insert(presenter_id, inclusive);
        self.inclusive_panels.get(&presenter_id).unwrap()
    }
}

impl Default for PanelToPresenterStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeStorage<PanelToPresenterEdge> for PanelToPresenterStorage {
    fn add_edge(&mut self, edge: PanelToPresenterEdge) -> Result<EdgeId, EdgeError> {
        let id = self.edges.add_edge(edge)?;
        self.invalidate_cache();
        Ok(id)
    }

    fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), EdgeError> {
        let result = self.edges.remove_edge(edge_id);
        self.invalidate_cache();
        result
    }

    fn get_edge(&self, edge_id: EdgeId) -> Option<&PanelToPresenterEdge> {
        self.edges.get_edge(edge_id)
    }

    fn find_outgoing(&self, from_id: EntityId) -> Vec<&PanelToPresenterEdge> {
        self.edges.find_outgoing(from_id)
    }

    fn find_incoming(&self, to_id: EntityId) -> Vec<&PanelToPresenterEdge> {
        self.edges.find_incoming(to_id)
    }

    fn edge_exists(&self, from_id: &EntityId, to_id: &EntityId) -> bool {
        self.edges.edge_exists(from_id, to_id)
    }

    fn len(&self) -> usize {
        self.edges.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_invalidation_on_add() {
        let mut storage = PanelToPresenterStorage::new();
        let panel_id: EntityId = 1;
        let presenter_id: EntityId = 10;

        storage
            .add_edge(PanelToPresenterEdge::new(panel_id, presenter_id))
            .unwrap();
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_cache_invalidation_on_remove() {
        let mut storage = PanelToPresenterStorage::new();
        let panel_id: EntityId = 1;
        let presenter_id: EntityId = 10;

        let edge_id = storage
            .add_edge(PanelToPresenterEdge::new(panel_id, presenter_id))
            .unwrap();
        storage.remove_edge(edge_id).unwrap();

        assert_eq!(storage.len(), 0);
    }

    #[test]
    fn test_get_inclusive_presenters_without_groups() {
        let mut storage = PanelToPresenterStorage::new();
        let panel_id: EntityId = 1;
        let presenter_id_1: EntityId = 10;
        let presenter_id_2: EntityId = 11;

        storage
            .add_edge(PanelToPresenterEdge::new(panel_id, presenter_id_1))
            .unwrap();
        storage
            .add_edge(PanelToPresenterEdge::new(panel_id, presenter_id_2))
            .unwrap();

        let mut group_storage = crate::edge::presenter_to_group::PresenterToGroupStorage::new();
        let presenters = storage.get_inclusive_presenters(panel_id, &mut group_storage);

        assert_eq!(presenters.len(), 2);
        assert!(presenters.contains(&presenter_id_1));
        assert!(presenters.contains(&presenter_id_2));
    }

    #[test]
    fn test_get_inclusive_presenters_with_groups() {
        let mut storage = PanelToPresenterStorage::new();
        let panel_id: EntityId = 1;
        let presenter_id: EntityId = 10;
        let group_id: EntityId = 20;
        let member_id: EntityId = 30;

        storage
            .add_edge(PanelToPresenterEdge::new(panel_id, presenter_id))
            .unwrap();

        let mut group_storage = crate::edge::presenter_to_group::PresenterToGroupStorage::new();
        group_storage
            .add_edge(crate::edge::presenter_to_group::PresenterToGroupEdge::new(
                presenter_id,
                group_id,
                false,
                false,
            ))
            .unwrap();
        group_storage
            .add_edge(crate::edge::presenter_to_group::PresenterToGroupEdge::new(
                member_id, group_id, false, false,
            ))
            .unwrap();

        let presenters = storage.get_inclusive_presenters(panel_id, &mut group_storage);

        assert!(presenters.contains(&presenter_id));
        assert!(presenters.contains(&member_id));
    }

    #[test]
    fn test_get_inclusive_panels_without_groups() {
        let mut storage = PanelToPresenterStorage::new();
        let panel_id_1: EntityId = 1;
        let panel_id_2: EntityId = 2;
        let presenter_id: EntityId = 10;

        storage
            .add_edge(PanelToPresenterEdge::new(panel_id_1, presenter_id))
            .unwrap();
        storage
            .add_edge(PanelToPresenterEdge::new(panel_id_2, presenter_id))
            .unwrap();

        let mut group_storage = crate::edge::presenter_to_group::PresenterToGroupStorage::new();
        let panels = storage.get_inclusive_panels(presenter_id, &mut group_storage);

        assert_eq!(panels.len(), 2);
        assert!(panels.contains(&panel_id_1));
        assert!(panels.contains(&panel_id_2));
    }

    #[test]
    fn test_cache_reuse() {
        let mut storage = PanelToPresenterStorage::new();
        let panel_id: EntityId = 1;
        let presenter_id: EntityId = 10;

        storage
            .add_edge(PanelToPresenterEdge::new(panel_id, presenter_id))
            .unwrap();

        let mut group_storage = crate::edge::presenter_to_group::PresenterToGroupStorage::new();

        // First call computes and caches
        let presenters1 = storage
            .get_inclusive_presenters(panel_id, &mut group_storage)
            .to_vec();
        // Second call should use cache
        let presenters2 = storage
            .get_inclusive_presenters(panel_id, &mut group_storage)
            .to_vec();

        assert_eq!(presenters1, presenters2);
    }
}
