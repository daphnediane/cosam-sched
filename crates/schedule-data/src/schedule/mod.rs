/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Schedule container and storage system

pub mod storage;

use chrono::NaiveDateTime;
use std::fmt;
use uuid::Uuid;

use crate::entity::edge::{EdgeType, RelationshipDirection};
use crate::entity::{EntityId, EntityType};
use crate::field::validation::ValidationError;
use crate::query::{FieldMatch, QueryOptions};

// Re-export storage types
pub use storage::*;

/// Unique identifier for edges
pub type EdgeId = Uuid;

/// Schedule container holding all entities and relationships
#[derive(Debug, Clone)]
pub struct Schedule {
    pub entities: EntityStorage,
    pub edges: EdgeStorage,
    pub metadata: ScheduleMetadata,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            entities: EntityStorage::new(),
            edges: EdgeStorage::new(),
            metadata: ScheduleMetadata::new(),
        }
    }

    /// Get entity by type and ID
    pub fn get_entity<T: EntityType>(&self, id: T::Id) -> Option<&T::Data> {
        self.entities.get::<T>(id)
    }

    /// Find entities matching field conditions
    pub fn find_entities<T: EntityType>(
        &self,
        matches: &[FieldMatch],
        options: Option<QueryOptions>,
    ) -> Vec<T::Id> {
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
    pub fn add_entity<T: EntityType>(&mut self, entity: T::Data) -> Result<T::Id, ScheduleError> {
        let id = T::entity_id(&entity);
        self.entities.add::<T>(entity)?;
        Ok(id)
    }

    /// Update entity fields
    pub fn update_entity<T: EntityType>(
        &mut self,
        id: T::Id,
        updates: &[(String, FieldValue)],
    ) -> Result<(), ScheduleError> {
        self.entities.update::<T>(id, updates)
    }

    /// Find entities related to a given entity
    pub fn find_related<T: EntityType>(
        &self,
        entity_id: T::Id,
        edge_type: EdgeType,
        direction: RelationshipDirection,
    ) -> Vec<EntityId> {
        self.edges
            .find_related::<T>(entity_id, edge_type, direction)
    }

    /// Add relationship between entities
    pub fn add_edge<From: EntityType, To: EntityType>(
        &mut self,
        from_id: From::Id,
        to_id: To::Id,
        edge_type: EdgeType,
    ) -> Result<EdgeId, ScheduleError> {
        self.edges.add_edge::<From, To>(from_id, to_id, edge_type)
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
