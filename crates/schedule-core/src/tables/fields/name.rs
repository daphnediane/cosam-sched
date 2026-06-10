/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! The shared `name` field.

use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, WriteFn};
use crate::query::converter::{convert_required, AsString, FieldTypeMapping};
use crate::tables::panel_like::PanelLike;
use crate::value::{FieldCardinality, FieldType, FieldTypeItem, FieldValue};

/// `name` — required, single string.
#[must_use]
pub const fn name_field<E: PanelLike>(
    order: u32,
    aliases: &'static [&'static str],
) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "name",
            display: "Name",
            description: "Name / title.",
            aliases,
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::String),
            example: "Opening Ceremony",
            order,
        },
        crdt_type: <AsString as FieldTypeMapping>::CRDT_TYPE,
        required: true,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                Some(FieldValue::Single(
                    <AsString as FieldTypeMapping>::to_field_value_item(E::name(d).clone()),
                ))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                *E::name_mut(d) = convert_required::<AsString>(v)?;
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}
