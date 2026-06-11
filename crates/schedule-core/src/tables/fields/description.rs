/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! The shared `description` field and the [`HasDescription`] capability trait.

use crate::entity::EntityType;
use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, WriteFn};
use crate::query::converter::{convert_optional, AsText, FieldTypeMapping};
use crate::value::{FieldCardinality, FieldType, FieldValue};

/// Entity types that carry an optional `description`.
///
/// A description is always long prose stored as a CRDT text field ([`AsText`]),
/// so [`description_field`] hardcodes that flavour — opting in is just the two
/// accessors, with no per-entity mapping to declare.
pub trait HasDescription: EntityType {
    /// The stored description.
    fn description(d: &Self::InternalData) -> &Option<String>;
    /// Mutable access to the stored description.
    fn description_mut(d: &mut Self::InternalData) -> &mut Option<String>;
}

/// `description` — optional long prose, stored as a CRDT text field ([`AsText`]).
#[must_use]
pub const fn description_field<E: HasDescription>(
    order: u32,
    aliases: &'static [&'static str],
) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "description",
            display: "Description",
            description: "Description.",
            aliases,
            field_type: FieldType(
                FieldCardinality::Optional,
                <AsText as FieldTypeMapping>::FIELD_TYPE_ITEM,
            ),
            example: "Mark the start of stuff on Thursday",
            order,
        },
        crdt_type: <AsText as FieldTypeMapping>::CRDT_TYPE,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                E::description(d).as_ref().map(|x| {
                    FieldValue::Single(<AsText as FieldTypeMapping>::to_field_value_item(x.clone()))
                })
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                *E::description_mut(d) = convert_optional::<AsText>(v)?;
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}
