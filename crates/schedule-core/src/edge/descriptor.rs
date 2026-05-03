/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge descriptor type: [`EdgeDescriptor<E>`].

use crate::field::{CommonFieldData, NamedField};

// ── EdgeKind ─────────────────────────────────────────────────────────────────

/// Ownership and relationship info for an edge half-edge field.
///
/// Stored in [`EdgeDescriptor::edge_kind`](EdgeDescriptor) and
/// exposed through the `edge_kind` field on [`HalfEdgeDescriptor`].
/// Replaces the `target_field` payload that was previously embedded directly
/// in [`crate::value::CrdtFieldType::EdgeOwner`].
#[derive(Clone, Copy)]
pub enum EdgeKind {
    /// Non-owner (lookup/inverse) side of an edge relationship.
    ///
    /// `source_fields` lists all owner-side fields whose `target_field` points
    /// at this field.  A single target field may be reached by multiple owners
    /// (e.g. `HALF_EDGE_PANELS` on `Presenter` is targeted by both
    /// `HALF_EDGE_CREDITED_PRESENTERS` and `HALF_EDGE_UNCREDITED_PRESENTERS` on
    /// `Panel`).
    ///
    /// Use `&[]` when no sources are known at static-initializer time.
    /// Because all edge fields live in the same crate, cross-module static
    /// references compile without circular-dependency issues.
    Target {
        /// Owner-side fields whose `target_field` is this field.
        source_fields: &'static [&'static HalfEdgeDescriptor],
    },
    /// CRDT-canonical owner side of an edge relationship.
    ///
    /// `exclusive_with` names a sibling field on the *same* entity whose
    /// entries must be removed before adding to this field.  Previously this
    /// logic was embedded in macro-generated closures; storing it here makes
    /// the descriptor self-describing.
    Owner {
        /// Inverse/lookup field on the target entity.
        target_field: &'static HalfEdgeDescriptor,
        /// Sibling field on the *same* entity that is mutually exclusive with
        /// this one (e.g. credited vs uncredited presenter lists).
        exclusive_with: Option<&'static HalfEdgeDescriptor>,
    },
}

impl EdgeKind {
    /// Returns `true` if this is the owning side of an edge.
    #[must_use]
    pub fn is_owner(&self) -> bool {
        matches!(self, Self::Owner { .. })
    }

    /// Returns the target field if this is an owner, or `None` for targets.
    #[must_use]
    pub fn target_field(&self) -> Option<&'static HalfEdgeDescriptor> {
        match self {
            Self::Owner { target_field, .. } => Some(*target_field),
            Self::Target { .. } => None,
        }
    }

    /// Returns the source fields if this is a target, or `None` for owners.
    #[must_use]
    pub fn source_fields(&self) -> Option<&'static [&'static HalfEdgeDescriptor]> {
        match self {
            Self::Target { source_fields } => Some(source_fields),
            Self::Owner { .. } => None,
        }
    }
}

impl PartialEq for EdgeKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Target { .. }, Self::Target { .. }) => true,
            (
                Self::Owner {
                    target_field: a, ..
                },
                Self::Owner {
                    target_field: b, ..
                },
            ) => std::ptr::eq(*a, *b),
            _ => false,
        }
    }
}

impl Eq for EdgeKind {}

impl std::fmt::Debug for EdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Target { source_fields } => f
                .debug_struct("Target")
                .field("source_count", &source_fields.len())
                .finish(),
            Self::Owner {
                target_field,
                exclusive_with,
            } => f
                .debug_struct("Owner")
                .field("target_field", &target_field.name())
                .field("exclusive_with", &exclusive_with.map(|e| e.name()))
                .finish(),
        }
    }
}

// ── EdgeDescriptor<E> ─────────────────────────────────────────────────────────

/// Edge field descriptor — one `static` value per edge field on an entity type.
///
/// Replaces [`FieldDescriptor<E>`] for edge fields (owner and target sides).
/// The [`edge_kind`](Self::edge_kind) field distinguishes ownership and carries
/// the target/source field references and exclusivity information.
///
/// # Example
///
/// ```ignore
/// pub static HALF_EDGE_CREDITED_PRESENTERS: HalfEdgeDescriptor<PanelEntityType> = HalfEdgeDescriptor {
///     data: CommonFieldData {
///         name: "credited_presenters",
///         display: "Credited Presenters",
///         description: "Presenters credited on this panel.",
///         aliases: &[],
///         field_type: FieldType(FieldCardinality::List, FieldTypeItem::EntityIdentifier("presenter")),
///         example: "",
///         order: 40,
///     },
///     edge_kind: EdgeKind::Owner {
///         target_field: &presenter::HALF_EDGE_PANELS,
///         exclusive_with: Some(&HALF_EDGE_UNCREDITED_PRESENTERS),
///     },
/// };
/// ```
///
/// [`FieldDescriptor<E>`]: crate::field::FieldDescriptor
pub struct HalfEdgeDescriptor {
    pub(crate) data: CommonFieldData,
    pub edge_kind: EdgeKind,
    pub entity_name: &'static str,
}

impl NamedField for HalfEdgeDescriptor {
    fn common_data(&self) -> &CommonFieldData {
        &self.data
    }

    fn entity_type_name(&self) -> &'static str {
        self.entity_name
    }

    fn try_as_half_edge(&self) -> Option<&HalfEdgeDescriptor> {
        Some(self)
    }
}

impl HalfEdgeDescriptor {
    /// Get the full edge connecting this field to another field.
    pub const fn edge_to(
        &'static self,
        far: &'static HalfEdgeDescriptor,
    ) -> crate::edge::id::FullEdge {
        crate::edge::id::FullEdge { near: self, far }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_kind_is_owner() {
        let owner = EdgeKind::Owner {
            target_field: &crate::tables::panel::HALF_EDGE_CREDITED_PRESENTERS,
            exclusive_with: None,
        };
        assert!(owner.is_owner());

        let target = EdgeKind::Target { source_fields: &[] };
        assert!(!target.is_owner());
    }

    #[test]
    fn test_edge_kind_target_field() {
        let target_field = &crate::tables::panel::HALF_EDGE_CREDITED_PRESENTERS;
        let owner = EdgeKind::Owner {
            target_field,
            exclusive_with: None,
        };
        assert!(owner.target_field().is_some());

        let target = EdgeKind::Target { source_fields: &[] };
        assert!(target.target_field().is_none());
    }

    #[test]
    fn test_edge_kind_source_fields() {
        let source_fields: &[&HalfEdgeDescriptor] = &[];
        let target = EdgeKind::Target { source_fields };
        assert!(target.source_fields().is_some());

        let owner = EdgeKind::Owner {
            target_field: &crate::tables::panel::HALF_EDGE_CREDITED_PRESENTERS,
            exclusive_with: None,
        };
        assert!(owner.source_fields().is_none());
    }

    #[test]
    fn test_edge_kind_partial_eq_target() {
        let target1 = EdgeKind::Target { source_fields: &[] };
        let target2 = EdgeKind::Target { source_fields: &[] };
        assert_eq!(target1, target2);
    }

    #[test]
    fn test_edge_kind_partial_eq_owner_same() {
        let target_field = &crate::tables::panel::HALF_EDGE_CREDITED_PRESENTERS;
        let owner1 = EdgeKind::Owner {
            target_field,
            exclusive_with: None,
        };
        let owner2 = EdgeKind::Owner {
            target_field,
            exclusive_with: None,
        };
        assert_eq!(owner1, owner2);
    }

    #[test]
    fn test_edge_kind_partial_eq_owner_different() {
        let owner1 = EdgeKind::Owner {
            target_field: &crate::tables::panel::HALF_EDGE_CREDITED_PRESENTERS,
            exclusive_with: None,
        };
        let owner2 = EdgeKind::Owner {
            target_field: &crate::tables::panel::HALF_EDGE_UNCREDITED_PRESENTERS,
            exclusive_with: None,
        };
        assert_ne!(owner1, owner2);
    }

    #[test]
    fn test_edge_kind_partial_eq_mismatch() {
        let owner = EdgeKind::Owner {
            target_field: &crate::tables::panel::HALF_EDGE_CREDITED_PRESENTERS,
            exclusive_with: None,
        };
        let target = EdgeKind::Target { source_fields: &[] };
        assert_ne!(owner, target);
    }

    #[test]
    fn test_edge_kind_debug_target() {
        let target = EdgeKind::Target { source_fields: &[] };
        let s = format!("{:?}", target);
        assert!(s.contains("Target"));
    }

    #[test]
    fn test_edge_kind_debug_owner() {
        let owner = EdgeKind::Owner {
            target_field: &crate::tables::panel::HALF_EDGE_CREDITED_PRESENTERS,
            exclusive_with: None,
        };
        let s = format!("{:?}", owner);
        assert!(s.contains("Owner"));
    }
}
