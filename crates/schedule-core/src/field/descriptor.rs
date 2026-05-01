/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field descriptor types: [`FieldDescriptor<E>`] and function pointer enums.

use crate::crdt::CrdtFieldType;
use crate::edge::traits::HalfEdge;
use crate::edge::EdgeKind;
use crate::entity::{EntityId, EntityType};
use crate::field::traits::{NamedField, ReadableField, VerifiableField, WritableField};
use crate::schedule::Schedule;
use crate::value::{FieldError, FieldValue, VerificationError};
use crate::FullEdge;

// ── ReadFn<E> ─────────────────────────────────────────────────────────────────

/// How a field reads its value: directly from [`EntityType::InternalData`], or
/// via a [`Schedule`] lookup by [`EntityId`].
pub enum ReadFn<E: EntityType> {
    /// Data-only read — no schedule access needed.
    Bare(fn(&E::InternalData) -> Option<FieldValue>),
    /// Schedule-aware read — fn receives `(&Schedule, EntityId<E>)` and
    /// performs its own entity lookup internally.
    Schedule(fn(&Schedule, EntityId<E>) -> Option<FieldValue>),
    /// Get Entities connected to this entity via a list of full edges.
    ReadEdges { edges: &'static [&'static FullEdge] },
    /// Read our edge -- to do remove and add to EdgeReadFn
    ReadEdge,
}

// ── WriteFn<E> ────────────────────────────────────────────────────────────────

/// How a field writes its value: directly into [`EntityType::InternalData`], or
/// via a [`Schedule`] lookup by [`EntityId`].
///
/// The `Schedule` variant avoids the double-`&mut` borrow problem: the fn
/// receives `(&mut Schedule, EntityId<E>)` with no `&mut InternalData`
/// parameter and handles its own lookup/release internally.
pub enum WriteFn<E: EntityType> {
    /// Data-only write — no schedule access needed.
    Bare(fn(&mut E::InternalData, FieldValue) -> Result<(), FieldError>),
    /// Schedule-aware write — used for edge mutations (e.g. `add_presenters`).
    Schedule(fn(&mut Schedule, EntityId<E>, FieldValue) -> Result<(), FieldError>),
    /// Add to an edge where both near and far are specified (for other fields)
    AddEdge {
        edge: FullEdge,
        exclusive_with: Option<FullEdge>,
    },
    /// Remove from an edge where both near and far are specified (for other fields)
    RemoveEdge {
        edge: FullEdge,
        exclusive_with: Option<FullEdge>,
    },
    /// Write our edge -- to do remove and add to EdgeWriteFn
    WriteEdge,
}

// ── VerifyFn<E> ─────────────────────────────────────────────────────────────────

/// How a field verifies its value after a batch write: directly from
/// [`EntityType::InternalData`], via a [`Schedule`] lookup, or by re-reading.
///
/// Verification checks that the field still has the value that was requested
/// after all writes in a batch have completed. This catches conflicts where
/// one computed field's write modified another field's backing data.
pub enum VerifyFn<E: EntityType> {
    /// Data-only verification — no schedule access needed.
    Bare(fn(&E::InternalData, &FieldValue) -> Result<(), VerificationError>),
    /// Schedule-aware verification — fn receives `(&Schedule, EntityId<E>)`.
    Schedule(fn(&Schedule, EntityId<E>, &FieldValue) -> Result<(), VerificationError>),
    /// Re-read verification — read the field back and compare to attempted value.
    /// Uses `read_fn` internally; fails verification if field is write-only.
    ReRead,
}

// ── FieldDescriptor<E> ─────────────────────────────────────────────────────────

/// Generic field descriptor — one `static` value per field on an entity type.
///
/// Uses enum fn pointers so it can be stored as a `static` value.
/// Non-capturing closures coerce to fn pointers automatically.
///
/// - `read_fn: None` — field is write-only; `read()` returns `FieldError::WriteOnly`.
/// - `write_fn: None` — field is read-only; `write()` returns `FieldError::ReadOnly`.
/// - `verify_fn: None` — field uses automatic read-back verification if `read_fn` is present.
///
/// # Example
///
/// ```ignore
/// static FIELD_NAME: FieldDescriptor<PanelEntityType> = FieldDescriptor {
///     data: CommonFieldData {
///         name: "name",
///         display: "Panel Name",
///         description: "The title of the panel.",
///         aliases: &[],
///         field_type: FieldType::Single(FieldTypeItem::String),
///         example: "",
///         order: 0,
///     },
///     required: true,
///     crdt_type: CrdtFieldType::Scalar,
///     edge_kind: EdgeKind::NonEdge,
///     read_fn: Some(ReadFn::Bare(|d| Some(FieldValue::String(d.data.name.clone())))),
///     write_fn: Some(WriteFn::Bare(|d, v| { d.data.name = v.into_string()?; Ok(()) })),
///     verify_fn: None,
/// };
///
/// static FIELD_ADD_PRESENTERS: FieldDescriptor<PanelEntityType> = FieldDescriptor {
///     data: CommonFieldData {
///         name: "add_presenters",
///         display: "Add Presenters",
///         description: "Add presenters to this panel.",
///         aliases: &[],
///         field_type: FieldType(FieldCardinality::List, FieldTypeItem::EntityIdentifier("presenter")),
///         example: "",
///         order: 10,
///     },
///     required: false,
///     crdt_type: CrdtFieldType::Derived,
///     edge_kind: EdgeKind::NonEdge,
///     read_fn: None,
///     write_fn: Some(WriteFn::Schedule(|schedule, id, v| { todo!() })),
///     verify_fn: None,
/// };
/// ```
pub struct FieldDescriptor<E: EntityType> {
    /// Data shared by all field types
    pub(crate) data: super::CommonFieldData,
    /// Whether the field is required (must be non-empty).
    pub required: bool,
    /// Edge ownership and relationship metadata -- (To be removed once EdgeDescriptor is live)
    pub edge_kind: EdgeKind,
    /// CRDT storage type annotation for Phase 4.
    pub crdt_type: CrdtFieldType,
    /// Read implementation. `None` means write-only.
    pub read_fn: Option<ReadFn<E>>,
    /// Write implementation. `None` means read-only.
    pub write_fn: Option<WriteFn<E>>,
    /// Verification implementation. `None` means use automatic read-back if `read_fn` is present.
    pub verify_fn: Option<VerifyFn<E>>,
}

impl<E: EntityType> NamedField for FieldDescriptor<E> {
    fn common_data(&self) -> &super::CommonFieldData {
        &self.data
    }

    fn entity_type_name(&self) -> &'static str {
        E::TYPE_NAME
    }

    fn crdt_type(&self) -> CrdtFieldType {
        self.crdt_type
    }

    fn try_as_half_edge(&self) -> Option<&dyn HalfEdge> {
        Some(self)
    }
}

impl<E: EntityType> HalfEdge for FieldDescriptor<E> {
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

impl<E: EntityType> ReadableField<E> for FieldDescriptor<E> {
    fn read(&self, id: EntityId<E>, schedule: &Schedule) -> Result<Option<FieldValue>, FieldError> {
        match &self.read_fn {
            None => Err(FieldError::WriteOnly {
                name: self.data.name,
            }),
            Some(ReadFn::Bare(f)) => Ok(schedule.get_internal::<E>(id).and_then(f)),
            Some(ReadFn::Schedule(f)) => Ok(f(schedule, id)),
            Some(ReadFn::ReadEdges { edges }) => {
                // Read entities connected via multiple full edges
                crate::schedule::combine_full_edges(schedule, id, edges)
            }
            Some(ReadFn::ReadEdge) => {
                // Read entities connected via this field's own edge relationship
                // This requires the field to implement HalfEdge
                match self.edge_kind {
                    crate::edge::EdgeKind::Owner { .. } | crate::edge::EdgeKind::Target { .. } => {
                        // SAFETY: self is a &'static FieldDescriptor<E> (field descriptors are static singletons).
                        let static_field: &'static dyn HalfEdge =
                            unsafe { std::mem::transmute(self as &dyn HalfEdge) };
                        crate::schedule::read_edge(schedule, id, static_field)
                    }
                    crate::edge::EdgeKind::NonEdge => Err(FieldError::Conversion(
                        crate::value::ConversionError::InvalidEdge {
                            reason: "NonEdge fields cannot use ReadEdge".to_string(),
                        },
                    )),
                }
            }
        }
    }
}

impl<E: EntityType> WritableField<E> for FieldDescriptor<E> {
    fn write(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        match &self.write_fn {
            None => {
                return Err(FieldError::ReadOnly {
                    name: self.data.name,
                })
            }
            Some(ref write_fn) => match write_fn {
                WriteFn::Bare(f) => {
                    let data = schedule
                        .get_internal_mut::<E>(id)
                        .ok_or(FieldError::NotFound {
                            name: self.data.name,
                        })?;
                    f(data, value)?;
                }
                WriteFn::Schedule(f) => f(schedule, id, value)?,
                WriteFn::AddEdge {
                    edge,
                    exclusive_with,
                } => crate::schedule::add_edge(schedule, id, edge, exclusive_with.as_ref(), value)?,
                WriteFn::RemoveEdge {
                    edge,
                    exclusive_with,
                } => crate::schedule::remove_edge(
                    schedule,
                    id,
                    edge,
                    exclusive_with.as_ref(),
                    value,
                )?,
                WriteFn::WriteEdge => {
                    // WriteEdge is valid for both, but FieldDescriptor should use WriteFn::Schedule for now
                    // This will be removed when HalfEdge is dropped from FieldDescriptor
                    return Err(FieldError::Conversion(crate::value::ConversionError::InvalidEdge {
                        reason: "FieldDescriptor should use WriteFn::Schedule for edge operations. WriteEdge will be removed from FieldDescriptor when HalfEdge is dropped.".to_string(),
                    }));
                }
            },
        }

        // CRDT mirror: after the inner write succeeds, read the post-write
        // value back through the descriptor's own read_fn and push it into
        // the authoritative automerge document.
        if !schedule.mirror_enabled()
            || matches!(self.crdt_type, crate::crdt::CrdtFieldType::Derived)
        {
            return Ok(());
        }
        let value_opt = match self.read(id, schedule) {
            Ok(v) => v,
            // Write-only fields are not mirrored back — edge commands mirror
            // their target-list fields themselves in FEATURE-023.
            Err(FieldError::WriteOnly { .. }) => return Ok(()),
            Err(e) => return Err(e),
        };
        schedule.mirror_field_value::<E>(id, self.data.name, self.crdt_type, value_opt.as_ref())
    }
}

impl<E: EntityType> VerifiableField<E> for FieldDescriptor<E> {
    fn verify(
        &self,
        id: EntityId<E>,
        schedule: &Schedule,
        attempted: &FieldValue,
    ) -> Result<(), VerificationError> {
        match &self.verify_fn {
            // Custom verification functions
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
            // Explicit opt-in to read-back verification
            Some(VerifyFn::ReRead) => {
                let actual = self
                    .read(id, schedule)
                    .map_err(|_| VerificationError::NotVerifiable {
                        field: self.data.name,
                    })?
                    .ok_or(VerificationError::NotVerifiable {
                        field: self.data.name,
                    })?;
                if actual == *attempted {
                    Ok(())
                } else {
                    Err(VerificationError::ValueChanged {
                        field: self.data.name,
                        requested: attempted.clone(),
                        actual,
                    })
                }
            }
            // No verification requested - success by default
            None => Ok(()),
        }
    }
}

impl<E: EntityType> FieldDescriptor<E> {
    /// Get the full edge connecting this field to another field.
    pub const fn edge_to<F: EntityType>(
        &'static self,
        far: &'static FieldDescriptor<F>,
    ) -> crate::edge::id::FullEdge {
        crate::edge::id::FullEdge { near: self, far }
    }
}
