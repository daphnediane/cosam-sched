/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! The shared `description` field.

use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, WriteFn};
use crate::query::converter::{convert_optional, FieldTypeMapping};
use crate::tables::panel_like::PanelLike;
use crate::value::{FieldCardinality, FieldType, FieldValue};

/// `description` — optional text. The marker `M` selects the value flavour:
/// [`AsString`](crate::query::converter::AsString) for Break/Timeline,
/// [`AsText`](crate::query::converter::AsText) for Panel (long prose, stored as
/// a CRDT text field).
#[must_use]
pub const fn description_field<E: PanelLike, M: FieldTypeMapping<Output = String>>(
    order: u32,
    aliases: &'static [&'static str],
) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "description",
            display: "Description",
            description: "Description.",
            aliases,
            field_type: FieldType(FieldCardinality::Optional, M::FIELD_TYPE_ITEM),
            example: "Mark the start of stuff on Thursday",
            order,
        },
        crdt_type: M::CRDT_TYPE,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                E::description(d)
                    .as_ref()
                    .map(|x| FieldValue::Single(M::to_field_value_item(x.clone())))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                *E::description_mut(d) = convert_optional::<M>(v)?;
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}
