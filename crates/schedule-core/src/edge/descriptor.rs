/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edge descriptor type: [`EdgeDescriptor<E>`].

use crate::crdt::CrdtFieldType;
use crate::edge::traits::HalfEdge;
use crate::entity::{EntityId, EntityType, EntityUuid};
use crate::field::{
    CommonFieldData, FieldCallbacks, NamedField, ReadFn, ReadableField, VerifiableField, VerifyFn,
    WritableField, WriteFn,
};
use crate::schedule::Schedule;
use crate::value::{FieldError, FieldValue, VerificationError};

// ── EdgeKind ─────────────────────────────────────────────────────────────────

/// Ownership and relationship info for an edge half-edge field.
///
/// Stored in [`EdgeDescriptor::edge_kind`](EdgeDescriptor) and
/// exposed through [`HalfEdge::edge_kind`](crate::edge::HalfEdge::edge_kind).
/// Replaces the `target_field` payload that was previously embedded directly
/// in [`crate::value::CrdtFieldType::EdgeOwner`].
#[derive(Clone, Copy)]
pub enum EdgeKind {
    /// Non-owner (lookup/inverse) side of an edge relationship.
    ///
    /// `source_fields` lists all owner-side fields whose `target_field` points
    /// at this field.  A single target field may be reached by multiple owners
    /// (e.g. `FIELD_PANELS` on `Presenter` is targeted by both
    /// `FIELD_CREDITED_PRESENTERS` and `FIELD_UNCREDITED_PRESENTERS` on
    /// `Panel`).
    ///
    /// Use `&[]` when no sources are known at static-initializer time.
    /// Because all edge fields live in the same crate, cross-module static
    /// references compile without circular-dependency issues.
    Target {
        /// Owner-side fields whose `target_field` is this field.
        source_fields: &'static [&'static dyn crate::edge::HalfEdge],
    },
    /// CRDT-canonical owner side of an edge relationship.
    ///
    /// `exclusive_with` names a sibling field on the *same* entity whose
    /// entries must be removed before adding to this field.  Previously this
    /// logic was embedded in macro-generated closures; storing it here makes
    /// the descriptor self-describing.
    Owner {
        /// Inverse/lookup field on the target entity.
        target_field: &'static dyn crate::edge::HalfEdge,
        /// Sibling field on the *same* entity that is mutually exclusive with
        /// this one (e.g. credited vs uncredited presenter lists).
        exclusive_with: Option<&'static dyn crate::edge::HalfEdge>,
    },
    /// Temporary value for non-edges for use by FieldDescriptor
    /// before we separate the descriptor types
    NonEdge,
}

impl EdgeKind {
    /// Returns `true` if this is the owning side of an edge.
    #[must_use]
    pub fn is_owner(&self) -> bool {
        matches!(self, Self::Owner { .. })
    }

    /// Returns the target field if this is an owner, or `None` for targets.
    #[must_use]
    pub fn target_field(&self) -> Option<&'static dyn crate::edge::HalfEdge> {
        match self {
            Self::Owner { target_field, .. } => Some(*target_field),
            Self::Target { .. } => None,
            Self::NonEdge => None,
        }
    }

    /// Returns the source fields if this is a target, or `None` for owners.
    #[must_use]
    pub fn source_fields(&self) -> Option<&'static [&'static dyn crate::edge::HalfEdge]> {
        match self {
            Self::Target { source_fields } => Some(source_fields),
            Self::Owner { .. } => None,
            Self::NonEdge => None,
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
            ) => std::ptr::eq(
                *a as *const dyn crate::edge::HalfEdge as *const (),
                *b as *const dyn crate::edge::HalfEdge as *const (),
            ),
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
            Self::NonEdge => f.write_str("NonEdge"),
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
/// static FIELD_CREDITED_PRESENTERS: EdgeDescriptor<PanelEntityType> = EdgeDescriptor {
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
///         target_field: &crate::tables::presenter::FIELD_PANELS,
///         exclusive_with: Some(&FIELD_UNCREDITED_PRESENTERS),
///     },
///     cb: FieldCallbacks {
///         read_fn: Some(ReadFn::Schedule(|sched, id| { … })),
///         write_fn: Some(WriteFn::Schedule(|sched, id, val| { … })),
///         verify_fn: None,
///     },
/// };
/// ```
///
/// [`FieldDescriptor<E>`]: crate::field::FieldDescriptor
pub struct EdgeDescriptor<E: EntityType> {
    /// Data shared by all field types
    pub(crate) data: CommonFieldData,
    /// Edge ownership and relationship metadata.
    pub edge_kind: EdgeKind,
    /// Callback functions for read/write/verify operations
    pub(crate) cb: FieldCallbacks<E>,
}

impl<E: EntityType> NamedField for EdgeDescriptor<E> {
    fn common_data(&self) -> &CommonFieldData {
        &self.data
    }

    fn entity_type_name(&self) -> &'static str {
        E::TYPE_NAME
    }

    fn crdt_type(&self) -> CrdtFieldType {
        CrdtFieldType::Derived
    }

    fn try_as_half_edge(&self) -> Option<&dyn HalfEdge> {
        Some(self)
    }
}

impl<E: EntityType> HalfEdge for EdgeDescriptor<E> {
    fn edge_kind(&self) -> &EdgeKind {
        &self.edge_kind
    }

    fn edge_id(&self) -> &'static dyn HalfEdge {
        // SAFETY: self is a &'static EdgeDescriptor<E> (edge descriptors are static singletons).
        unsafe { std::mem::transmute(self as &dyn HalfEdge) }
    }

    fn as_named_field(&self) -> &dyn NamedField {
        self
    }
}

impl<E: EntityType> ReadableField<E> for EdgeDescriptor<E> {
    fn read(&self, id: EntityId<E>, schedule: &Schedule) -> Result<Option<FieldValue>, FieldError> {
        match &self.cb.read_fn {
            None => Err(FieldError::WriteOnly {
                name: self.data.name,
            }),
            Some(ReadFn::Bare(f)) => Ok(schedule.get_internal::<E>(id).and_then(f)),
            Some(ReadFn::Schedule(f)) => Ok(f(schedule, id)),
            Some(ReadFn::ReadEdges { edges }) => {
                // Read entities connected via a list of full edges
                // Optimized single-edge case: no deduplication needed
                if edges.len() == 1 {
                    let edge = edges[0];
                    let neighbors = schedule.connected_field_nodes(id, *edge);
                    let items = neighbors
                        .into_iter()
                        .map(crate::value::FieldValueItem::EntityIdentifier)
                        .collect();
                    return Ok(Some(crate::value::FieldValue::List(items)));
                }

                // Multi-edge case: collect, deduplicate, and convert
                let mut all_ids: Vec<crate::entity::RuntimeEntityId> = Vec::new();
                for edge in *edges {
                    let neighbors = schedule.connected_field_nodes(id, **edge);
                    for neighbor in neighbors {
                        all_ids.push(neighbor);
                    }
                }
                // Deduplicate and convert to FieldValue
                all_ids.sort_by_key(|e| e.entity_uuid());
                all_ids.dedup_by_key(|e| e.entity_uuid());
                let items = all_ids
                    .into_iter()
                    .map(crate::value::FieldValueItem::EntityIdentifier)
                    .collect();
                Ok(Some(crate::value::FieldValue::List(items)))
            }
            Some(ReadFn::ReadEdge) => {
                // Read entities connected via this edge descriptor's own relationship
                // SAFETY: self is a &'static EdgeDescriptor<E> (edge descriptors are static singletons).
                let static_field: &'static dyn HalfEdge =
                    unsafe { std::mem::transmute(self as &dyn HalfEdge) };
                crate::schedule::edge::read_edge(schedule, id, static_field)
            }
        }
    }
}

impl<E: EntityType> WritableField<E> for EdgeDescriptor<E> {
    fn write(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        match &self.cb.write_fn {
            None => Err(FieldError::ReadOnly {
                name: self.data.name,
            }),
            Some(WriteFn::Bare(f)) => {
                let data = schedule
                    .get_internal_mut::<E>(id)
                    .ok_or(FieldError::NotFound {
                        name: self.data.name,
                    })?;
                f(data, value)
            }
            Some(WriteFn::Schedule(f)) => f(schedule, id, value),
            Some(WriteFn::AddEdge {
                edge,
                exclusive_with,
            }) => crate::schedule::add_edge(schedule, id, edge, exclusive_with.as_ref(), value),
            Some(WriteFn::RemoveEdge {
                edge,
                exclusive_with,
            }) => crate::schedule::remove_edge(schedule, id, edge, exclusive_with.as_ref(), value),
            Some(WriteFn::WriteEdge) => {
                // Set the edges from this entity to the target entities specified in value
                // SAFETY: self is a &'static EdgeDescriptor<E> (edge descriptors are static singletons).
                let static_field: &'static dyn HalfEdge =
                    unsafe { std::mem::transmute(self as &dyn HalfEdge) };
                crate::schedule::edge::write_edge(schedule, id, static_field, value)
            }
        }
    }
}

impl<E: EntityType> VerifiableField<E> for EdgeDescriptor<E> {
    fn verify(
        &self,
        id: EntityId<E>,
        schedule: &Schedule,
        attempted: &FieldValue,
    ) -> Result<(), VerificationError> {
        match &self.cb.verify_fn {
            None => Ok(()),
            Some(VerifyFn::ReRead) => {
                // Re-read the field and compare with the attempted value
                if let Ok(Some(actual)) = self.read(id, schedule) {
                    if actual == *attempted {
                        Ok(())
                    } else {
                        Err(VerificationError::ValueChanged {
                            field: self.data.name,
                            requested: attempted.clone(),
                            actual,
                        })
                    }
                } else {
                    Err(VerificationError::NotVerifiable {
                        field: self.data.name,
                    })
                }
            }
            Some(VerifyFn::Bare(f)) => {
                let data =
                    schedule
                        .get_internal::<E>(id)
                        .ok_or(VerificationError::NotVerifiable {
                            field: self.data.name,
                        })?;
                f(data, attempted)
            }
            Some(VerifyFn::Schedule(f)) => f(schedule, id, attempted),
        }
    }
}

impl<E: EntityType> EdgeDescriptor<E> {
    /// Get the full edge connecting this field to another field.
    pub const fn edge_to<F: EntityType>(
        &'static self,
        far: &'static EdgeDescriptor<F>,
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
            target_field: &crate::tables::panel::FIELD_CREDITED_PRESENTERS,
            exclusive_with: None,
        };
        assert!(owner.is_owner());

        let target = EdgeKind::Target { source_fields: &[] };
        assert!(!target.is_owner());

        let non_edge = EdgeKind::NonEdge;
        assert!(!non_edge.is_owner());
    }

    #[test]
    fn test_edge_kind_target_field() {
        let target_field = &crate::tables::panel::FIELD_CREDITED_PRESENTERS;
        let owner = EdgeKind::Owner {
            target_field,
            exclusive_with: None,
        };
        assert!(owner.target_field().is_some());

        let target = EdgeKind::Target { source_fields: &[] };
        assert!(target.target_field().is_none());

        let non_edge = EdgeKind::NonEdge;
        assert!(non_edge.target_field().is_none());
    }

    #[test]
    fn test_edge_kind_source_fields() {
        let source_fields: &[&dyn HalfEdge] = &[];
        let target = EdgeKind::Target { source_fields };
        assert!(target.source_fields().is_some());

        let owner = EdgeKind::Owner {
            target_field: &crate::tables::panel::FIELD_CREDITED_PRESENTERS,
            exclusive_with: None,
        };
        assert!(owner.source_fields().is_none());

        let non_edge = EdgeKind::NonEdge;
        assert!(non_edge.source_fields().is_none());
    }

    #[test]
    fn test_edge_kind_partial_eq_target() {
        let target1 = EdgeKind::Target { source_fields: &[] };
        let target2 = EdgeKind::Target { source_fields: &[] };
        assert_eq!(target1, target2);
    }

    #[test]
    fn test_edge_kind_partial_eq_owner_same() {
        let target_field = &crate::tables::panel::FIELD_CREDITED_PRESENTERS;
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
            target_field: &crate::tables::panel::FIELD_CREDITED_PRESENTERS,
            exclusive_with: None,
        };
        let owner2 = EdgeKind::Owner {
            target_field: &crate::tables::panel::FIELD_UNCREDITED_PRESENTERS,
            exclusive_with: None,
        };
        assert_ne!(owner1, owner2);
    }

    #[test]
    fn test_edge_kind_partial_eq_mismatch() {
        let owner = EdgeKind::Owner {
            target_field: &crate::tables::panel::FIELD_CREDITED_PRESENTERS,
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
            target_field: &crate::tables::panel::FIELD_CREDITED_PRESENTERS,
            exclusive_with: None,
        };
        let s = format!("{:?}", owner);
        assert!(s.contains("Owner"));
    }

    #[test]
    fn test_edge_kind_debug_non_edge() {
        let non_edge = EdgeKind::NonEdge;
        let s = format!("{:?}", non_edge);
        assert_eq!(s, "NonEdge");
    }
}
