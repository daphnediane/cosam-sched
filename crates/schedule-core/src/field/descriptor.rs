/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Field descriptor types: [`FieldDescriptor<E>`].

use crate::entity::{EntityId, EntityType};
use crate::field::callback::{FieldCallbacks, ReadFn, WriteFn};
use crate::field::traits::NamedField;
use crate::schedule::Schedule;
use crate::value::{FieldError, FieldValue};

// ── FieldDescriptor<E> ─────────────────────────────────────────────────────────

/// Generic field descriptor — one `static` value per field on an entity type.
///
/// Uses enum fn pointers so it can be stored as a `static` value.
/// Non-capturing closures coerce to fn pointers automatically.
///
/// - `cb.read_fn: None` — field is write-only; `read()` returns `FieldError::WriteOnly`.
/// - `cb.write_fn: None` — field is read-only; `write()` returns `FieldError::ReadOnly`.
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
///     crdt_type: CrdtFieldType::Scalar,
///     required: true,
///     cb: accessor_callbacks!(PanelEntityType, required, name, AsString),
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
///     crdt_type: CrdtFieldType::Derived,
///     required: false,
///     cb: FieldCallbacks {
///         read_fn: None,
///         write_fn: Some(WriteFn::Schedule(|schedule, id, v| { todo!() })),
///     },
/// };
/// ```
pub struct FieldDescriptor<E: EntityType> {
    /// Data shared by all field types
    pub(crate) data: super::CommonFieldData,
    /// CRDT storage type annotation.
    pub crdt_type: crate::crdt::CrdtFieldType,
    /// Whether the field is required (must be non-empty).
    pub required: bool,
    /// Callback functions for read/write operations
    pub(crate) cb: FieldCallbacks<E>,
}

impl<E: EntityType> NamedField for FieldDescriptor<E> {
    fn common_data(&self) -> &super::CommonFieldData {
        &self.data
    }

    fn entity_type_name(&self) -> &'static str {
        E::TYPE_NAME
    }

    fn try_as_half_edge(&self) -> Option<&crate::edge::HalfEdgeDescriptor> {
        None
    }
}

impl<E: EntityType> FieldDescriptor<E> {
    pub fn read(
        &self,
        id: EntityId<E>,
        schedule: &Schedule,
    ) -> Result<Option<FieldValue>, FieldError> {
        match &self.cb.read_fn {
            None => Err(FieldError::WriteOnly {
                name: self.data.name,
            }),
            Some(ReadFn::Bare(f)) => Ok(schedule.get_internal::<E>(id).and_then(f)),
            Some(ReadFn::Schedule(f)) => Ok(f(schedule, id)),
        }
    }

    pub fn write(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        match &self.cb.write_fn {
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

    pub fn add(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        match &self.cb.add_fn {
            None => Err(FieldError::ReadOnly {
                name: self.data.name,
            }),
            Some(ref add_fn) => match add_fn {
                crate::field::callback::AddFn::Bare(f) => {
                    let data = schedule
                        .get_internal_mut::<E>(id)
                        .ok_or(FieldError::NotFound {
                            name: self.data.name,
                        })?;
                    f(data, value)
                }
                crate::field::callback::AddFn::Schedule(f) => f(schedule, id, value),
            },
        }
    }

    pub fn remove(
        &self,
        id: EntityId<E>,
        schedule: &mut Schedule,
        value: FieldValue,
    ) -> Result<(), FieldError> {
        match &self.cb.remove_fn {
            None => Err(FieldError::ReadOnly {
                name: self.data.name,
            }),
            Some(ref remove_fn) => match remove_fn {
                crate::field::callback::RemoveFn::Bare(f) => {
                    let data = schedule
                        .get_internal_mut::<E>(id)
                        .ok_or(FieldError::NotFound {
                            name: self.data.name,
                        })?;
                    f(data, value)
                }
                crate::field::callback::RemoveFn::Schedule(f) => f(schedule, id, value),
            },
        }
    }
}
