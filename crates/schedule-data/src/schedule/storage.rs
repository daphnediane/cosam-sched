/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity and edge storage implementation

use std::collections::HashMap;

use super::{EdgeId, ScheduleError};
use crate::entity::edge::{EdgeType, RelationshipDirection};
use crate::entity::{EntityId, EntityState, EntityType};
use crate::field::FieldValue;
use crate::query::{FieldMatch, QueryOptions};

/// Generic entity storage
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EntityStorage {
    // Store entities by type name and internal ID
    entities: HashMap<String, EntityTypeStorage>,
    // Indexing for efficient queries
    indexes: HashMap<String, HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Default)]
struct EntityTypeStorage {
    by_internal_id: HashMap<u64, StoredEntity>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct StoredEntity {
    internal_id: u64,
    data: String, // Serialized JSON data
    state: EntityState,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

impl EntityStorage {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            indexes: HashMap::new(),
        }
    }

    /// Get entity by type and ID
    pub fn get<T: EntityType>(&self, id: EntityId) -> Option<&T::Data> {
        let type_name = T::TYPE_NAME;
        self.entities
            .get(type_name)
            .and_then(|type_entities| type_entities.by_internal_id.get(&id))
            .and_then(|stored| self.deserialize::<T>(&stored.data))
    }

    /// Get entity by internal monotonic ID (alias for get)
    pub fn get_by_internal_id<T: EntityType>(&self, internal_id: u64) -> Option<&T::Data> {
        self.get::<T>(internal_id)
    }

    /// Get entities by index query, returning all that tie at the best match strength.
    pub fn get_by_index<T: EntityType>(&self, query: &str) -> Vec<&T::Data> {
        let type_name = T::TYPE_NAME;
        let field_set = T::field_set();

        if let Some(type_entities) = self.entities.get(type_name) {
            let mut matches: Vec<(u64, crate::field::traits::FieldMatchResult)> = Vec::new();
            let mut best_strength = crate::field::traits::MatchStrength::NotMatch;

            // @TODO: Should consider priority if matching strength is the same
            for (internal_id, stored) in &type_entities.by_internal_id {
                if let Some(entity) = self.deserialize::<T>(&stored.data) {
                    if let Some(match_result) = field_set.match_index(query, *internal_id, entity) {
                        if match_result.strength > best_strength {
                            best_strength = match_result.strength;
                            matches.clear();
                            matches.push((*internal_id, match_result));
                        } else if match_result.strength == best_strength {
                            matches.push((*internal_id, match_result));
                        }
                    }
                }
            }

            matches
                .into_iter()
                .filter_map(|(id, _)| {
                    type_entities
                        .by_internal_id
                        .get(&id)
                        .and_then(|stored| self.deserialize::<T>(&stored.data))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get multiple entities matching field conditions
    pub fn get_many<T: EntityType>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<&T::Data> {
        let ids = self.find::<T>(matches, options);
        ids.into_iter().filter_map(|id| self.get::<T>(id)).collect()
    }

    /// Find entity IDs matching field conditions
    pub fn find<T: EntityType>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<EntityId> {
        let type_name = T::TYPE_NAME;
        let options = options.unwrap_or_default();

        if let Some(type_entities) = self.entities.get(type_name) {
            let mut results = Vec::new();

            for stored in type_entities.by_internal_id.values() {
                // Apply state filter
                if let Some(state_filter) = options.state_filter {
                    if stored.state != state_filter {
                        continue;
                    }
                }

                // Apply field matches
                if let Some(_entity) = self.deserialize::<T>(&stored.data) {
                    let mut matches_all = true;

                    for field_match in matches {
                        let field = T::field_set().get_field(&field_match.field_name);

                        if let Some(_field) = field {
                            // TODO: Implement proper field matching using SimpleReadableField
                            if !self.simple_field_match::<T>(&field_match.matcher) {
                                matches_all = false;
                                break;
                            }
                        } else {
                            matches_all = false;
                            break;
                        }
                    }

                    if matches_all {
                        results.push(stored.internal_id);
                    }
                }
            }

            // Apply ordering
            if let Some(_order_by) = options.order_by {
                results.sort_by(|a, b| {
                    // Simplified ordering - in practice, you'd extract field values
                    let a_str = a.to_string();
                    let b_str = b.to_string();
                    if options.ascending {
                        a_str.cmp(&b_str)
                    } else {
                        b_str.cmp(&a_str)
                    }
                });
            }

            // Apply limit and offset
            let start = options.offset.unwrap_or(0);
            let end = if let Some(limit) = options.limit {
                (start + limit).min(results.len())
            } else {
                results.len()
            };

            results.into_iter().skip(start).take(end - start).collect()
        } else {
            Vec::new()
        }
    }

    /// Add entity to storage with pre-allocated internal ID
    pub fn add_with_id<T: EntityType>(
        &mut self,
        internal_id: u64,
        entity: T::Data,
    ) -> Result<(), ScheduleError> {
        let type_name = T::TYPE_NAME;

        let type_entities = self.entities.entry(type_name.to_string()).or_default();

        // Check for duplicates
        if type_entities.by_internal_id.contains_key(&internal_id) {
            return Err(ScheduleError::DuplicateEntity {
                entity_type: type_name.to_string(),
                id: internal_id.to_string(),
            });
        }

        let stored = StoredEntity {
            internal_id,
            data: format!("{:?}", entity),
            state: EntityState::Active,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };

        type_entities.by_internal_id.insert(internal_id, stored);

        Ok(())
    }

    /// Update entity fields
    pub fn update<T: EntityType>(
        &mut self,
        id: EntityId,
        _updates: &[(String, FieldValue)],
    ) -> Result<(), ScheduleError> {
        let type_name = T::TYPE_NAME;

        if let Some(type_entities) = self.entities.get_mut(type_name) {
            if let Some(stored) = type_entities.by_internal_id.get_mut(&id) {
                // Update timestamp
                stored.updated_at = chrono::Utc::now().naive_utc();

                // In practice, you'd deserialize, apply updates, and re-serialize
                // For now, just mark as updated
                Ok(())
            } else {
                Err(ScheduleError::EntityNotFound {
                    entity_type: type_name.to_string(),
                    id: id.to_string(),
                })
            }
        } else {
            Err(ScheduleError::EntityNotFound {
                entity_type: type_name.to_string(),
                id: id.to_string(),
            })
        }
    }

    // Helper methods
    fn deserialize<T: EntityType>(&self, _data: &str) -> Option<&T::Data> {
        // Simplified deserialization - in practice, use serde_json
        // This is a placeholder that doesn't actually work
        None
    }

    fn simple_field_match<T: EntityType>(&self, _matcher: &crate::field::FieldMatcher) -> bool {
        // Simplified matching - in practice, extract field value and compare
        true
    }
}

impl Default for EntityStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Edge storage implementation
#[derive(Debug, Clone)]
pub struct EdgeStorage {
    edges: HashMap<EdgeId, StoredEdge>,
    // Indexes for efficient relationship queries
    outgoing_index: HashMap<String, HashMap<EdgeType, Vec<EdgeId>>>,
    incoming_index: HashMap<String, HashMap<EdgeType, Vec<EdgeId>>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StoredEdge {
    from_id: String,
    to_id: String,
    edge_type: EdgeType,
    data: EdgeData,
}

#[derive(Debug, Clone)]
pub struct EdgeData {
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub metadata: HashMap<String, FieldValue>,
}

impl EdgeStorage {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            outgoing_index: HashMap::new(),
            incoming_index: HashMap::new(),
        }
    }

    /// Add edge between entities
    pub fn add_edge_with_id<From: EntityType, To: EntityType>(
        &mut self,
        edge_id: EdgeId,
        from_id: EntityId,
        to_id: EntityId,
        edge_type: EdgeType,
    ) -> Result<EdgeId, ScheduleError> {
        let from_str = from_id.to_string();
        let to_str = to_id.to_string();

        // Check for duplicates
        if self.edge_exists(&from_str, &to_str, edge_type) {
            return Err(ScheduleError::StorageError {
                message: "Duplicate edge".to_string(),
            });
        }

        let edge = StoredEdge {
            from_id: from_str.clone(),
            to_id: to_str.clone(),
            edge_type,
            data: EdgeData {
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
                metadata: HashMap::new(),
            },
        };

        // Add to main storage
        self.edges.insert(edge_id, edge);

        // Update indexes
        self.outgoing_index
            .entry(from_str)
            .or_default()
            .entry(edge_type)
            .or_default()
            .push(edge_id);

        self.incoming_index
            .entry(to_str)
            .or_default()
            .entry(edge_type)
            .or_default()
            .push(edge_id);

        Ok(edge_id)
    }

    /// Remove edge
    pub fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), ScheduleError> {
        if let Some(edge) = self.edges.remove(&edge_id) {
            // Remove from indexes
            if let Some(type_map) = self.outgoing_index.get_mut(&edge.from_id) {
                if let Some(edge_list) = type_map.get_mut(&edge.edge_type) {
                    edge_list.retain(|&id| id != edge_id);
                }
            }

            if let Some(type_map) = self.incoming_index.get_mut(&edge.to_id) {
                if let Some(edge_list) = type_map.get_mut(&edge.edge_type) {
                    edge_list.retain(|&id| id != edge_id);
                }
            }

            Ok(())
        } else {
            Err(ScheduleError::EdgeNotFound {
                edge_id: edge_id.to_string(),
            })
        }
    }

    /// Find entities related to a given entity
    pub fn find_related<T: EntityType>(
        &self,
        entity_id: EntityId,
        edge_type: EdgeType,
        direction: RelationshipDirection,
    ) -> Vec<EntityId> {
        let id_str = entity_id.to_string();

        let edge_ids = match direction {
            RelationshipDirection::Outgoing => self
                .outgoing_index
                .get(&id_str)
                .and_then(|type_map| type_map.get(&edge_type))
                .map(|list| list.as_slice())
                .unwrap_or(&[]),
            RelationshipDirection::Incoming => self
                .incoming_index
                .get(&id_str)
                .and_then(|type_map| type_map.get(&edge_type))
                .map(|list| list.as_slice())
                .unwrap_or(&[]),
        };

        edge_ids
            .iter()
            .filter_map(|&edge_id| self.edges.get(&edge_id))
            .map(|edge| match direction {
                RelationshipDirection::Outgoing => edge.to_id.parse::<u64>().unwrap_or(0),
                RelationshipDirection::Incoming => edge.from_id.parse::<u64>().unwrap_or(0),
            })
            .filter(|id| *id > 0)
            .collect()
    }

    /// Check if edge exists
    pub fn edge_exists(&self, from_id: &str, to_id: &str, edge_type: EdgeType) -> bool {
        if let Some(edge_ids) = self
            .outgoing_index
            .get(from_id)
            .and_then(|m| m.get(&edge_type))
        {
            edge_ids.iter().any(|&edge_id| {
                if let Some(edge) = self.edges.get(&edge_id) {
                    edge.to_id == to_id
                } else {
                    false
                }
            })
        } else {
            false
        }
    }

    /// Get outgoing edges from entity
    pub fn outgoing_from(
        &self,
        entity_id: &str,
        edge_type: EdgeType,
    ) -> impl Iterator<Item = &StoredEdge> {
        self.outgoing_index
            .get(entity_id)
            .and_then(|type_map| type_map.get(&edge_type))
            .into_iter()
            .flat_map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|&edge_id| self.edges.get(&edge_id))
            })
    }

    /// Get incoming edges to entity
    pub fn incoming_to(
        &self,
        entity_id: &str,
        edge_type: EdgeType,
    ) -> impl Iterator<Item = &StoredEdge> {
        self.incoming_index
            .get(entity_id)
            .and_then(|type_map| type_map.get(&edge_type))
            .into_iter()
            .flat_map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|&edge_id| self.edges.get(&edge_id))
            })
    }
}

impl Default for EdgeStorage {
    fn default() -> Self {
        Self::new()
    }
}
