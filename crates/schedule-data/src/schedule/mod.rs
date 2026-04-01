/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule container and storage system

pub mod storage;

use chrono::NaiveDateTime;
use std::collections::HashMap;
use std::fmt;

use crate::edge::EdgeType;

/// Relationship direction for edge queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipDirection {
    Outgoing, // Entity -> Related
    Incoming, // Related -> Entity
}
use crate::entity::{EntityId, EntityType};
use crate::field::validation::ValidationError;
use crate::field::FieldValue;
use crate::query::{FieldMatch, QueryOptions};

// Re-export storage types
pub use storage::*;

/// Unique identifier for edges
pub type EdgeId = u64;

#[derive(Debug, Clone)]
pub struct IdAllocators {
    next_by_type: HashMap<String, u64>,
    next_edge_id: u64,
}

impl IdAllocators {
    pub fn new() -> Self {
        Self {
            next_by_type: HashMap::new(),
            next_edge_id: 1,
        }
    }

    pub fn allocate_entity_id(&mut self, type_name: &str) -> u64 {
        let next = self.next_by_type.entry(type_name.to_string()).or_insert(1);
        let allocated = *next;
        *next = allocated.saturating_add(1);
        allocated
    }

    pub fn allocate_edge_id(&mut self) -> u64 {
        let allocated = self.next_edge_id;
        self.next_edge_id = allocated.saturating_add(1);
        allocated
    }
}

impl Default for IdAllocators {
    fn default() -> Self {
        Self::new()
    }
}

/// Schedule container holding all entities and relationships
#[derive(Debug, Clone)]
pub struct Schedule {
    pub entities: EntityStorage,
    pub edges: EdgeStorage,
    pub id_allocators: IdAllocators,
    pub metadata: ScheduleMetadata,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            entities: EntityStorage::new(),
            edges: EdgeStorage::new(),
            id_allocators: IdAllocators::new(),
            metadata: ScheduleMetadata::new(),
        }
    }

    /// Get entity by type and ID
    pub fn get_entity<T: EntityType>(&self, id: EntityId) -> Option<&T::Data> {
        self.entities.get::<T>(id)
    }

    /// Get entity by internal monotonic ID
    pub fn get_entity_by_internal_id<T: EntityType>(&self, internal_id: u64) -> Option<&T::Data> {
        self.entities.get_by_internal_id::<T>(internal_id)
    }

    /// Find entities matching field conditions
    pub fn find_entities<T: EntityType>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<EntityId> {
        self.entities.find::<T>(matches, options)
    }

    /// Get entities matching field conditions
    pub fn get_entities<T: EntityType>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<&T::Data> {
        self.entities.get_many::<T>(matches, options)
    }

    /// Add entity to schedule
    pub fn add_entity<T: EntityType>(
        &mut self,
        entity: T::Data,
    ) -> Result<EntityId, ScheduleError> {
        let internal_id = self.id_allocators.allocate_entity_id(T::TYPE_NAME);
        self.entities.add_with_id::<T>(internal_id, entity)?;
        Ok(internal_id)
    }

    /// Add entity and return both internal and external IDs
    pub fn add_entity_with_internal_id<T: EntityType>(
        &mut self,
        entity: T::Data,
    ) -> Result<u64, ScheduleError> {
        let internal_id = self.id_allocators.allocate_entity_id(T::TYPE_NAME);
        self.entities.add_with_id::<T>(internal_id, entity)?;
        Ok(internal_id)
    }

    /// Update entity fields
    pub fn update_entity<T: EntityType>(
        &mut self,
        id: EntityId,
        updates: &[(String, FieldValue)],
    ) -> Result<(), ScheduleError> {
        self.entities.update::<T>(id, updates)
    }

    /// Find entities related to a given entity
    pub fn find_related<T: EntityType>(
        &self,
        entity_id: EntityId,
        edge_type: EdgeType,
        direction: RelationshipDirection,
    ) -> Vec<EntityId> {
        self.edges
            .find_related::<T>(entity_id, edge_type, direction)
    }

    /// Add relationship between entities
    pub fn add_edge<From: EntityType, To: EntityType>(
        &mut self,
        from_id: EntityId,
        to_id: EntityId,
        edge_type: EdgeType,
    ) -> Result<EdgeId, ScheduleError> {
        let edge_id = self.id_allocators.allocate_edge_id();
        self.edges
            .add_edge_with_id::<From, To>(edge_id, from_id, to_id, edge_type)
    }

    /// Remove relationship
    pub fn remove_edge(&mut self, edge_id: EdgeId) -> Result<(), ScheduleError> {
        self.edges.remove_edge(edge_id)
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

/// Schedule metadata
#[derive(Debug, Clone)]
pub struct ScheduleMetadata {
    pub version: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub generator: String,
}

impl ScheduleMetadata {
    pub fn new() -> Self {
        let now = chrono::Utc::now().naive_utc();
        Self {
            version: "1.0".to_string(),
            created_at: now,
            updated_at: now,
            generator: "schedule-data".to_string(),
        }
    }
}

impl Default for ScheduleMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Schedule-specific errors
#[derive(Debug, Clone)]
pub enum ScheduleError {
    EntityNotFound { entity_type: String, id: String },
    EdgeNotFound { edge_id: String },
    ValidationError { errors: Vec<ValidationError> },
    StorageError { message: String },
    DuplicateEntity { entity_type: String, id: String },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScheduleError::EntityNotFound { entity_type, id } => {
                write!(f, "Entity {} with ID {} not found", entity_type, id)
            }
            ScheduleError::EdgeNotFound { edge_id } => {
                write!(f, "Edge with ID {} not found", edge_id)
            }
            ScheduleError::ValidationError { errors } => {
                write!(f, "Validation failed: {:?}", errors)
            }
            ScheduleError::StorageError { message } => {
                write!(f, "Storage error: {}", message)
            }
            ScheduleError::DuplicateEntity { entity_type, id } => {
                write!(f, "Duplicate entity {} with ID {}", entity_type, id)
            }
        }
    }
}

impl std::error::Error for ScheduleError {}
