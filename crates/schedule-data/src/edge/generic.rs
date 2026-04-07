/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Generic edge storage implementation for simple edges

use crate::edge::{Edge, EdgeError, EdgeId, EdgeStorage};
use std::collections::HashMap;
use uuid::Uuid;

/// Generic edge storage for simple edges
#[derive(Debug, Clone)]
pub struct GenericEdgeStorage<E: Edge> {
    edges: HashMap<EdgeId, E>,
    outgoing_index: HashMap<Uuid, Vec<EdgeId>>,
    incoming_index: HashMap<Uuid, Vec<EdgeId>>,
}

impl<E: Edge> GenericEdgeStorage<E> {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            outgoing_index: HashMap::new(),
            incoming_index: HashMap::new(),
        }
    }

    /// Remove all outgoing edges from an entity.
    pub fn remove_outgoing_edges(&mut self, from_uuid: Uuid) -> Result<(), EdgeError> {
        let edge_ids = self
            .outgoing_index
            .get(&from_uuid)
            .cloned()
            .unwrap_or_default();
        for edge_id in edge_ids {
            self.remove_edge(edge_id)?;
        }
        Ok(())
    }

    /// Add an edge to storage with a specific edge ID
    pub fn add_edge_with_id(&mut self, edge_id: EdgeId, edge: E) -> Result<EdgeId, EdgeError> {
        let from_uuid = edge.from_uuid();
        let to_uuid = edge.to_uuid();

        // Check for duplicates (only for edges with a from_uuid)
        if let Some(from) = from_uuid {
            if let Some(to) = to_uuid {
                if self.edge_exists(from, to) {
                    return Err(EdgeError::DuplicateEdge {
                        from_id: from.to_string(),
                        to_id: to.to_string(),
                    });
                }
            }

            // Add to main storage
            self.edges.insert(edge_id, edge);

            // Update indexes using uuid for efficiency
            self.outgoing_index.entry(from).or_default().push(edge_id);
        } else {
            // Edge without from_uuid (group-only edge)
            self.edges.insert(edge_id, edge);
        }

        if let Some(to) = to_uuid {
            self.incoming_index.entry(to).or_default().push(edge_id);
        }

        Ok(edge_id)
    }

    /// Remove an edge from storage
    pub fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), EdgeError> {
        if let Some(edge) = self.edges.remove(&edge_id) {
            let from_uuid = edge.from_uuid();
            let to_uuid = edge.to_uuid();

            // Remove from outgoing index
            if let Some(from) = from_uuid {
                if let Some(edges) = self.outgoing_index.get_mut(&from) {
                    edges.retain(|&id| id != edge_id);
                    if edges.is_empty() {
                        self.outgoing_index.remove(&from);
                    }
                }
            }

            // Remove from incoming index
            if let Some(to) = to_uuid {
                if let Some(edges) = self.incoming_index.get_mut(&to) {
                    edges.retain(|&id| id != edge_id);
                    if edges.is_empty() {
                        self.incoming_index.remove(&to);
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
    pub fn find_outgoing(&self, from_uuid: Uuid) -> Vec<&E> {
        self.outgoing_index
            .get(&from_uuid)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|&edge_id| self.edges.get(&edge_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find incoming edges to an entity
    pub fn find_incoming(&self, to_uuid: Uuid) -> Vec<&E> {
        self.incoming_index
            .get(&to_uuid)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|&edge_id| self.edges.get(&edge_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if an edge exists between two entities
    pub fn edge_exists(&self, from_uuid: Uuid, to_uuid: Uuid) -> bool {
        self.outgoing_index
            .get(&from_uuid)
            .map(|edge_ids| {
                edge_ids.iter().any(|&edge_id| {
                    self.edges
                        .get(&edge_id)
                        .map(|edge| edge.to_uuid() == Some(to_uuid))
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
        // EdgeId is allocated by the caller and passed via the edge
        // For now, we generate a simple EdgeId based on current count
        // This will be replaced when EdgeId becomes UUID in REFACTOR-038
        let edge_id = EdgeId(self.edges.len() as u64);
        self.add_edge_with_id(edge_id, edge)
    }

    fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), EdgeError> {
        self.remove_edge(edge_id)
    }

    fn get_edge(&self, edge_id: EdgeId) -> Option<&E> {
        self.get_edge(edge_id)
    }

    fn find_outgoing(&self, from_uuid: Uuid) -> Vec<&E> {
        self.find_outgoing(from_uuid)
    }

    fn find_incoming(&self, to_uuid: Uuid) -> Vec<&E> {
        self.find_incoming(to_uuid)
    }

    fn edge_exists(&self, from_uuid: Uuid, to_uuid: Uuid) -> bool {
        self.edge_exists(from_uuid, to_uuid)
    }

    fn len(&self) -> usize {
        self.len()
    }
}
