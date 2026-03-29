/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity and edge storage implementation

use std::collections::HashMap;
use uuid::Uuid;

use super::{EdgeId, ScheduleError};
use crate::entity::edge::{EdgeType, RelationshipDirection};
use crate::entity::{EntityState, EntityType};
use crate::field::FieldValue;
use crate::query::{FieldMatch, QueryOptions};

/// Generic entity storage
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EntityStorage {
    // Store entities by type name and ID
    entities: HashMap<String, HashMap<String, StoredEntity>>,
    // Indexing for efficient queries
    indexes: HashMap<String, HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct StoredEntity {
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
    pub fn get<T: EntityType>(&self, id: T::Id) -> Option<&T::Data> {
        let type_name = T::TYPE_NAME;
        let id_str = id.to_string();

        self.entities
            .get(type_name)
            .and_then(|type_entities| type_entities.get(&id_str))
            .and_then(|stored| self.deserialize::<T>(&stored.data))
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
    ) -> Vec<T::Id> {
        let type_name = T::TYPE_NAME;
        let options = options.unwrap_or_default();

        if let Some(type_entities) = self.entities.get(type_name) {
            let mut results = Vec::new();

            for (id_str, stored) in type_entities {
                // Apply state filter
                if let Some(state_filter) = options.state_filter {
                    if stored.state != state_filter {
                        continue;
                    }
                }

                // Apply field matches
                if let Some(entity) = self.deserialize::<T>(&stored.data) {
                    let mut matches_all = true;

                    for field_match in matches {
                        let field = T::fields()
                            .iter()
                            .find(|f| f.name == field_match.field_name);

                        if let Some(field) = field {
                            // This is a simplified matching - in practice, you'd need
                            // the full Schedule context for relationship fields
                            if !self.simple_field_match(entity, field, &field_match.matcher) {
                                matches_all = false;
                                break;
                            }
                        } else {
                            matches_all = false;
                            break;
                        }
                    }

                    if matches_all {
                        if let Some(id) = self.parse_id::<T>(id_str) {
                            results.push(id);
                        }
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

    /// Add entity to storage
    pub fn add<T: EntityType>(&mut self, entity: T::Data) -> Result<(), ScheduleError> {
        let type_name = T::TYPE_NAME;
        let id = T::entity_id(&entity);
        let id_str = id.to_string();

        // Check for duplicates
        if let Some(type_entities) = self.entities.get(type_name) {
            if type_entities.contains_key(&id_str) {
                return Err(ScheduleError::DuplicateEntity {
                    entity_type: type_name.to_string(),
                    id: id_str,
                });
            }
        }

        let stored = StoredEntity {
            data: self.serialize::<T>(&entity),
            state: EntityState::Active,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };

        self.entities
            .entry(type_name.to_string())
            .or_default()
            .insert(id_str, stored);

        Ok(())
    }

    /// Update entity fields
    pub fn update<T: EntityType>(
        &mut self,
        id: T::Id,
        _updates: &[(String, FieldValue)],
    ) -> Result<(), ScheduleError> {
        let type_name = T::TYPE_NAME;
        let id_str = id.to_string();

        if let Some(type_entities) = self.entities.get_mut(type_name) {
            if let Some(stored) = type_entities.get_mut(&id_str) {
                // Update timestamp
                stored.updated_at = chrono::Utc::now().naive_utc();

                // In practice, you'd deserialize, apply updates, and re-serialize
                // For now, just mark as updated
                Ok(())
            } else {
                Err(ScheduleError::EntityNotFound {
                    entity_type: type_name.to_string(),
                    id: id_str,
                })
            }
        } else {
            Err(ScheduleError::EntityNotFound {
                entity_type: type_name.to_string(),
                id: id_str,
            })
        }
    }

    // Helper methods
    fn serialize<T: EntityType>(&self, entity: &T::Data) -> String {
        // Simplified serialization - in practice, use serde_json
        format!("{:?}", entity)
    }

    fn deserialize<T: EntityType>(&self, _data: &str) -> Option<&T::Data> {
        // Simplified deserialization - in practice, use serde_json
        // This is a placeholder that doesn't actually work
        None
    }

    fn simple_field_match<T: EntityType>(
        &self,
        _entity: &T::Data,
        _field: &FieldDescriptor<T>,
        _matcher: &crate::field::FieldMatcher,
    ) -> bool {
        // Simplified matching - in practice, extract field value and compare
        true
    }

    fn parse_id<T: EntityType>(&self, _id_str: &str) -> Option<T::Id> {
        // Simplified ID parsing - in practice, this depends on the ID type
        // For now, this won't work but serves as a placeholder
        None
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
    pub fn add_edge<From: EntityType, To: EntityType>(
        &mut self,
        from_id: From::Id,
        to_id: To::Id,
        edge_type: EdgeType,
    ) -> Result<EdgeId, ScheduleError> {
        let edge_id = Uuid::new_v4();
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
        entity_id: T::Id,
        edge_type: EdgeType,
        direction: RelationshipDirection,
    ) -> Vec<String> {
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
                RelationshipDirection::Outgoing => edge.to_id.clone(),
                RelationshipDirection::Incoming => edge.from_id.clone(),
            })
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
