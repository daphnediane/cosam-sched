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
use serde::{Deserialize, Serialize};

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

/// Serializable proxy for [`FullEdge`].
///
/// Stores the owner half-edge's `"entity_type:field_name"` key and a bool
/// indicating which side of the edge is `near`. Every valid `FullEdge` has
/// exactly one Owner half-edge, so this 2-field form uniquely identifies any
/// edge even when a Target half-edge is shared by multiple Owners.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct FullEdgeProxy {
    /// `"entity_type:field_name"` of the Owner half-edge.
    owner_field: String,
    /// `true` if `near` is the Owner side; `false` if `near` is the Target side.
    near_is_owner: bool,
}

impl From<FullEdge> for FullEdgeProxy {
    fn from(edge: FullEdge) -> Self {
        if edge.near.edge_kind().is_owner() {
            Self {
                owner_field: edge.near.field_key(),
                near_is_owner: true,
            }
        } else {
            Self {
                owner_field: edge.far.field_key(),
                near_is_owner: false,
            }
        }
    }
}

impl TryFrom<FullEdgeProxy> for FullEdge {
    type Error = String;

    fn try_from(proxy: FullEdgeProxy) -> Result<Self, Self::Error> {
        let canonical = crate::registry::get_full_edge_by_owner(&proxy.owner_field)
            .ok_or_else(|| format!("Unknown owner edge: {}", proxy.owner_field))?;
        if proxy.near_is_owner {
            Ok(canonical)
        } else {
            Ok(canonical.flip())
        }
    }
}

impl Serialize for FullEdge {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        FullEdgeProxy::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FullEdge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let proxy = FullEdgeProxy::deserialize(deserializer)?;
        FullEdge::try_from(proxy).map_err(serde::de::Error::custom)
    }
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

impl<E: EntityType> TryFrom<&'static crate::edge::HalfEdgeDescriptor<E>> for FullEdge {
    type Error = ConversionError;

    fn try_from(
        descriptor: &'static crate::edge::HalfEdgeDescriptor<E>,
    ) -> Result<Self, Self::Error> {
        // EdgeDescriptor implements HalfEdge, so delegate to that implementation
        Self::try_from(descriptor as &'static dyn HalfEdge)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::tables::panel;

    use super::*;

    #[test]
    fn test_full_edge_serialize_round_trip() {
        // FullEdge from the panel -> panel_type edge
        let edge = panel::EDGE_PANEL_TYPE;

        // Serialize to JSON
        let json = serde_json::to_string(&edge).expect("Failed to serialize FullEdge");

        // Deserialize back
        let deserialized: FullEdge =
            serde_json::from_str(&json).expect("Failed to deserialize FullEdge");

        // Verify the round-trip preserves the edge (by pointer equality)
        assert_eq!(edge, deserialized);
    }

    #[test]
    fn test_full_edge_proxy_from() {
        let edge = panel::EDGE_PANEL_TYPE;
        let proxy = FullEdgeProxy::from(edge);

        assert_eq!(proxy.owner_field, "panel:panel_type");
        assert!(proxy.near_is_owner);
    }

    #[test]
    fn test_full_edge_proxy_from_flipped() {
        let canonical = panel::EDGE_PANEL_TYPE;
        let flipped = canonical.flip();
        let proxy = FullEdgeProxy::from(flipped);

        assert_eq!(proxy.owner_field, "panel:panel_type");
        assert!(!proxy.near_is_owner);
    }

    #[test]
    fn test_full_edge_proxy_try_from() {
        let edge = panel::EDGE_PANEL_TYPE;
        let proxy = FullEdgeProxy::from(edge);

        let reconstructed = FullEdge::try_from(proxy).expect("Failed to reconstruct FullEdge");

        // Verify reconstruction matches original (by pointer equality)
        assert_eq!(edge, reconstructed);
    }

    #[test]
    fn test_full_edge_proxy_try_from_flipped() {
        let canonical = panel::EDGE_PANEL_TYPE;
        let flipped = canonical.flip();
        let proxy = FullEdgeProxy::from(flipped);

        let reconstructed =
            FullEdge::try_from(proxy).expect("Failed to reconstruct flipped FullEdge");

        assert_eq!(flipped, reconstructed);
    }
}
