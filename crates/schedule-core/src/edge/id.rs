/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge identifier types.
//!
//! This module defines the [`FullEdge`] type, which represents a complete bidirectional
//! edge relationship between two fields. Each edge endpoint is identified by both the
//! entity UUID *and* which field the relationship belongs to, making edge direction
//! self-describing.
//!
//! ## Design
//!
//! A [`FullEdge`] combines two [`HalfEdge`] references to represent a complete
//! bidirectional edge. The `near` half-edge is the starting point, and the `far`
//! half-edge is the opposite side of the relationship.
//!
//! Equality and hashing are based on the pointer address of the field descriptors,
//! which is stable for `'static` references.
//!
//! [`FieldDescriptor<E>`]: crate::field::FieldDescriptor

use crate::edge::HalfEdge;
use crate::entity::{DynamicEntityId, EntityType};
use crate::value::ConversionError;

// ── FullEdge ─────────────────────────────────────────────────────────────────────

/// A complete edge with both near and far half-edges.
///
/// Represents a bidirectional edge relationship between two fields. The `near` half-edge
/// is the starting point, and the `far` half-edge is the opposite side of the relationship.
#[derive(Clone, Copy)]
pub struct FullEdge {
    /// The near half-edge (the starting side of the edge).
    pub near: &'static dyn HalfEdge,
    /// The far half-edge (the opposite side of the edge).
    pub far: &'static dyn HalfEdge,
}

impl FullEdge {
    /// Check if a dynamic entity ID is a valid near node for this edge.
    ///
    /// Returns `true` if the entity's type matches the near half-edge's entity type.
    #[must_use]
    pub fn is_valid_near(&self, entity: impl DynamicEntityId) -> bool {
        entity.entity_type_name() == self.near.entity_type_name()
    }

    /// Check if a dynamic entity ID is a valid far node for this edge.
    ///
    /// Returns `true` if the entity's type matches the far half-edge's entity type.
    #[must_use]
    pub fn is_valid_far(&self, entity: impl DynamicEntityId) -> bool {
        entity.entity_type_name() == self.far.entity_type_name()
    }

    /// Check if an entity type is a valid near node type for this edge.
    ///
    /// Returns `true` if the entity type's name matches the near half-edge's entity type.
    #[must_use]
    pub fn is_valid_near_type<E: EntityType>(&self) -> bool {
        E::TYPE_NAME == self.near.entity_type_name()
    }

    /// Check if an entity type is a valid far node type for this edge.
    ///
    /// Returns `true` if the entity type's name matches the far half-edge's entity type.
    #[must_use]
    pub fn is_valid_far_type<E: EntityType>(&self) -> bool {
        E::TYPE_NAME == self.far.entity_type_name()
    }

    /// Check if the edge is homogeneous (connects the same entity type on both sides).
    ///
    /// Returns `true` if both half-edges have the same entity type name.
    #[must_use]
    pub fn is_homogeneous(&self) -> bool {
        self.near.entity_type_name() == self.far.entity_type_name()
    }

    /// Flip the edge, swapping near and far.
    ///
    /// Returns a new FullEdge with near and far swapped.
    #[must_use]
    pub const fn flip(&self) -> Self {
        Self {
            near: self.far,
            far: self.near,
        }
    }
}

impl std::fmt::Debug for FullEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FullEdge")
            .field("near", &self.near.name())
            .field("far", &self.far.name())
            .finish()
    }
}

impl PartialEq for FullEdge {
    fn eq(&self, other: &Self) -> bool {
        // Compare by pointer address of both half-edges
        std::ptr::eq(
            self.near as *const dyn HalfEdge as *const (),
            other.near as *const dyn HalfEdge as *const (),
        ) && std::ptr::eq(
            self.far as *const dyn HalfEdge as *const (),
            other.far as *const dyn HalfEdge as *const (),
        )
    }
}

impl Eq for FullEdge {}

impl std::hash::Hash for FullEdge {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash by pointer address of both half-edges
        (self.near as *const dyn HalfEdge as *const () as usize).hash(state);
        (self.far as *const dyn HalfEdge as *const () as usize).hash(state);
    }
}

impl TryFrom<&'static dyn HalfEdge> for FullEdge {
    type Error = ConversionError;

    fn try_from(near: &'static dyn HalfEdge) -> Result<Self, Self::Error> {
        match near.edge_kind() {
            crate::edge::EdgeKind::Owner { target_field, .. } => {
                // Owner has a single target field
                Ok(Self {
                    near,
                    far: *target_field,
                })
            }
            crate::edge::EdgeKind::Target { source_fields } => {
                // Target must have exactly one source field
                match source_fields {
                    [single] => Ok(Self { near, far: *single }),
                    [] => Err(ConversionError::InvalidEdge {
                        reason: "Target edge has no source fields".to_string(),
                    }),
                    _ => Err(ConversionError::InvalidEdge {
                        reason: "Target edge has multiple source fields".to_string(),
                    }),
                }
            }
            crate::edge::EdgeKind::NonEdge => Err(ConversionError::InvalidEdge {
                reason: "Non-edge fields cannot form a FullEdge".to_string(),
            }),
        }
    }
}

impl<E: EntityType> TryFrom<&'static crate::edge::EdgeDescriptor<E>> for FullEdge {
    type Error = ConversionError;

    fn try_from(descriptor: &'static crate::edge::EdgeDescriptor<E>) -> Result<Self, Self::Error> {
        // EdgeDescriptor implements HalfEdge, so delegate to that implementation
        Self::try_from(descriptor as &'static dyn HalfEdge)
    }
}
