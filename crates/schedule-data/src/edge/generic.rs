/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Generic edge storage implementation for simple edges

use crate::edge::{Edge, EdgeError, EdgeId, EdgeStorage};
use crate::entity::EntityId;
use std::collections::HashMap;

/// Generic edge storage for simple edges
#[derive(Debug, Clone)]
pub struct GenericEdgeStorage<E: Edge> {
    edges: HashMap<EdgeId, E>,
    outgoing_index: HashMap<EntityId, Vec<EdgeId>>,
    incoming_index: HashMap<EntityId, Vec<EdgeId>>,
    next_id: u64,
}

impl<E: Edge> GenericEdgeStorage<E> {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            outgoing_index: HashMap::new(),
            incoming_index: HashMap::new(),
            next_id: 0,
        }
    }

    /// Remove all outgoing edges from an entity.
    pub fn remove_outgoing_edges(
        &mut self,
        from_id: crate::entity::InternalId,
    ) -> Result<(), EdgeError> {
        let edge_ids = self
            .outgoing_index
            .get(&from_id.entity_id)
            .cloned()
            .unwrap_or_default();
        for edge_id in edge_ids {
            self.remove_edge(edge_id)?;
        }
        Ok(())
    }

    /// Add an edge to storage
    pub fn add_edge(&mut self, edge: E) -> Result<EdgeId, EdgeError> {
        let edge_id = EdgeId(self.next_id);
        self.next_id += 1;

        let from_id = edge.from_id();
        let to_id = edge.to_id();

        // Check for duplicates (only for edges with a from_id)
        if let Some(from) = from_id {
            if let Some(to) = to_id {
                if self.edge_exists(&from, &to) {
                    return Err(EdgeError::DuplicateEdge {
                        from_id: from.to_string(),
                        to_id: to.to_string(),
                    });
                }
            }

            // Add to main storage
            self.edges.insert(edge_id, edge);

            // Update indexes using entity_id for efficiency
            self.outgoing_index
                .entry(from.entity_id)
                .or_default()
                .push(edge_id);
        } else {
            // Edge without from_id (group-only edge)
            self.edges.insert(edge_id, edge);
        }

        if let Some(to) = to_id {
            self.incoming_index
                .entry(to.entity_id)
                .or_default()
                .push(edge_id);
        }

        Ok(edge_id)
    }

    /// Remove an edge from storage
    pub fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), EdgeError> {
        if let Some(edge) = self.edges.remove(&edge_id) {
            let from_id = edge.from_id();
            let to_id = edge.to_id();

            // Remove from outgoing index
            if let Some(from) = from_id {
                if let Some(edges) = self.outgoing_index.get_mut(&from.entity_id) {
                    edges.retain(|&id| id != edge_id);
                    if edges.is_empty() {
                        self.outgoing_index.remove(&from.entity_id);
                    }
                }
            }

            // Remove from incoming index
            if let Some(to) = to_id {
                if let Some(edges) = self.incoming_index.get_mut(&to.entity_id) {
                    edges.retain(|&id| id != edge_id);
                    if edges.is_empty() {
                        self.incoming_index.remove(&to.entity_id);
                    }
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
    pub fn find_outgoing(&self, from_id: crate::entity::InternalId) -> Vec<&E> {
        self.outgoing_index
            .get(&from_id.entity_id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|&edge_id| self.edges.get(&edge_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find incoming edges to an entity
    pub fn find_incoming(&self, to_id: crate::entity::InternalId) -> Vec<&E> {
        self.incoming_index
            .get(&to_id.entity_id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|&edge_id| self.edges.get(&edge_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if an edge exists between two entities
    pub fn edge_exists(
        &self,
        from_id: &crate::entity::InternalId,
        to_id: &crate::entity::InternalId,
    ) -> bool {
        self.outgoing_index
            .get(&from_id.entity_id)
            .map(|edge_ids| {
                edge_ids.iter().any(|&edge_id| {
                    self.edges
                        .get(&edge_id)
                        .map(|edge| edge.to_id() == Some(*to_id))
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

impl<E: Edge> Default for GenericEdgeStorage<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Edge> EdgeStorage<E> for GenericEdgeStorage<E> {
    fn add_edge(&mut self, edge: E) -> Result<EdgeId, EdgeError> {
        self.add_edge(edge)
    }

    fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), EdgeError> {
        self.remove_edge(edge_id)
    }

    fn get_edge(&self, edge_id: EdgeId) -> Option<&E> {
        self.get_edge(edge_id)
    }

    fn find_outgoing(&self, from_id: crate::entity::InternalId) -> Vec<&E> {
        self.find_outgoing(from_id)
    }

    fn find_incoming(&self, to_id: crate::entity::InternalId) -> Vec<&E> {
        self.find_incoming(to_id)
    }

    fn edge_exists(
        &self,
        from_id: &crate::entity::InternalId,
        to_id: &crate::entity::InternalId,
    ) -> bool {
        self.edge_exists(from_id, to_id)
    }

    fn len(&self) -> usize {
        self.len()
    }
}
