/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! PanelToPanelType edge implementation

use crate::edge::{Edge, EdgeError, EdgeId, EdgeStorage};
use crate::entity::EntityId;
use std::collections::HashMap;

/// PanelToPanelType edge implementation
#[derive(Debug, Clone)]
pub struct PanelToPanelTypeEdge {
    pub from_id: EntityId, // Panel
    pub to_id: EntityId,   // PanelType
    pub data: PanelToPanelTypeData,
}

#[derive(Debug, Clone)]
pub struct PanelToPanelTypeData {
    // No additional data needed for this simple relationship
}

impl PanelToPanelTypeEdge {
    pub fn new(panel_id: EntityId, panel_type_id: EntityId) -> Self {
        Self {
            from_id: panel_id,
            to_id: panel_type_id,
            data: PanelToPanelTypeData {},
        }
    }
}

impl Edge for PanelToPanelTypeEdge {
    type FromEntity = crate::entity::PanelEntityType;
    type ToEntity = crate::entity::PanelTypeEntityType;
    type Data = PanelToPanelTypeData;

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
        crate::edge::EdgeType::PanelToPanelType
    }
}

/// Specialized storage for PanelToPanelType with one-to-many cardinality enforcement
#[derive(Debug, Clone)]
pub struct PanelToPanelTypeStorage {
    edges_by_id: HashMap<EdgeId, PanelToPanelTypeEdge>,
    panel_to_edge_id: HashMap<EntityId, EdgeId>,
    panel_to_type_index: HashMap<EntityId, EntityId>,
    type_to_panels: HashMap<EntityId, Vec<EntityId>>,
    next_id: u64,
}

impl PanelToPanelTypeStorage {
    pub fn new() -> Self {
        Self {
            edges_by_id: HashMap::new(),
            panel_to_edge_id: HashMap::new(),
            panel_to_type_index: HashMap::new(),
            type_to_panels: HashMap::new(),
            next_id: 0,
        }
    }

    /// Get the panel type for a specific panel
    pub fn get_panel_type(&self, panel_id: EntityId) -> Option<EntityId> {
        self.panel_to_type_index.get(&panel_id).copied()
    }

    /// Get all panels of a specific type
    pub fn get_panels_by_type(&self, type_id: EntityId) -> Vec<EntityId> {
        self.type_to_panels
            .get(&type_id)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for PanelToPanelTypeStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeStorage<PanelToPanelTypeEdge> for PanelToPanelTypeStorage {
    fn add_edge(&mut self, edge: PanelToPanelTypeEdge) -> Result<EdgeId, EdgeError> {
        let panel_id = edge.from_id;
        let type_id = edge.to_id;

        if let Some(existing_edge_id) = self.panel_to_edge_id.get(&panel_id).copied() {
            self.remove_edge(existing_edge_id)?;
        }

        let id = EdgeId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);

        self.edges_by_id.insert(id, edge);
        self.panel_to_edge_id.insert(panel_id, id);
        self.panel_to_type_index.insert(panel_id, type_id);
        self.type_to_panels
            .entry(type_id)
            .or_default()
            .push(panel_id);
        Ok(id)
    }

    fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), EdgeError> {
        if let Some(edge) = self.edges_by_id.remove(&edge_id) {
            self.panel_to_type_index.remove(&edge.from_id);
            self.panel_to_edge_id.remove(&edge.from_id);

            if let Some(panels) = self.type_to_panels.get_mut(&edge.to_id) {
                panels.retain(|panel_id| *panel_id != edge.from_id);
                if panels.is_empty() {
                    self.type_to_panels.remove(&edge.to_id);
                }
            }
            Ok(())
        } else {
            Err(EdgeError::EdgeNotFound {
                edge_id: edge_id.to_string(),
            })
        }
    }

    fn get_edge(&self, edge_id: EdgeId) -> Option<&PanelToPanelTypeEdge> {
        self.edges_by_id.get(&edge_id)
    }

    fn find_outgoing(&self, from_id: EntityId) -> Vec<&PanelToPanelTypeEdge> {
        if let Some(edge_id) = self.panel_to_edge_id.get(&from_id) {
            if let Some(edge) = self.edges_by_id.get(edge_id) {
                return vec![edge];
            }
        }
        Vec::new()
    }

    fn find_incoming(&self, to_id: EntityId) -> Vec<&PanelToPanelTypeEdge> {
        self.type_to_panels
            .get(&to_id)
            .map(|panel_ids| {
                panel_ids
                    .iter()
                    .filter_map(|panel_id| {
                        self.panel_to_edge_id
                            .get(panel_id)
                            .and_then(|edge_id| self.edges_by_id.get(edge_id))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn edge_exists(&self, from_id: &EntityId, to_id: &EntityId) -> bool {
        self.panel_to_type_index
            .get(from_id)
            .map(|existing_to_id| existing_to_id == to_id)
            .unwrap_or(false)
    }

    fn len(&self) -> usize {
        self.edges_by_id.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_to_many_enforcement() {
        let mut storage = PanelToPanelTypeStorage::new();
        let panel_id: EntityId = 1;
        let type_id_1: EntityId = 10;
        let type_id_2: EntityId = 11;

        // Add first edge
        let edge1 = PanelToPanelTypeEdge::new(panel_id, type_id_1);
        storage.add_edge(edge1).unwrap();

        // Verify panel has type_id_1
        assert_eq!(storage.get_panel_type(panel_id), Some(type_id_1));

        // Add second edge for same panel (should replace)
        let edge2 = PanelToPanelTypeEdge::new(panel_id, type_id_2);
        storage.add_edge(edge2).unwrap();

        // Verify panel now has type_id_2
        assert_eq!(storage.get_panel_type(panel_id), Some(type_id_2));

        // Verify only one edge exists
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_get_panels_by_type() {
        let mut storage = PanelToPanelTypeStorage::new();
        let panel_id_1: EntityId = 1;
        let panel_id_2: EntityId = 2;
        let panel_id_3: EntityId = 3;
        let type_id_1: EntityId = 10;
        let type_id_2: EntityId = 11;

        storage
            .add_edge(PanelToPanelTypeEdge::new(panel_id_1, type_id_1))
            .unwrap();
        storage
            .add_edge(PanelToPanelTypeEdge::new(panel_id_2, type_id_1))
            .unwrap();
        storage
            .add_edge(PanelToPanelTypeEdge::new(panel_id_3, type_id_2))
            .unwrap();

        let panels_type_1 = storage.get_panels_by_type(type_id_1);
        assert_eq!(panels_type_1.len(), 2);
        assert!(panels_type_1.contains(&panel_id_1));
        assert!(panels_type_1.contains(&panel_id_2));

        let panels_type_2 = storage.get_panels_by_type(type_id_2);
        assert_eq!(panels_type_2.len(), 1);
        assert!(panels_type_2.contains(&panel_id_3));
    }

    #[test]
    fn test_remove_edge_updates_index() {
        let mut storage = PanelToPanelTypeStorage::new();
        let panel_id: EntityId = 1;
        let type_id: EntityId = 10;

        let edge = PanelToPanelTypeEdge::new(panel_id, type_id);
        let edge_id = storage.add_edge(edge).unwrap();

        assert_eq!(storage.get_panel_type(panel_id), Some(type_id));

        storage.remove_edge(edge_id).unwrap();

        assert_eq!(storage.get_panel_type(panel_id), None);
    }
}
