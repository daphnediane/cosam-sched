/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! The shared `code` (Uniq ID) field.

use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, WriteFn};
use crate::field_value;
use crate::query::converter::{AsString, FieldTypeMapping};
use crate::tables::panel_like::PanelLike;
use crate::value::uniq_id::PanelUniqId;
use crate::value::{ConversionError, FieldCardinality, FieldType, FieldTypeItem};

/// `code` (Uniq ID) — stored as a parsed [`PanelUniqId`], exposed as a string.
#[must_use]
pub const fn code_field<E: PanelLike>(order: u32) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "code",
            display: "Code",
            description: "Uniq ID code (e.g. \"GP032\"), parsed from the Schedule sheet.",
            aliases: &["uid", "uniq_id", "id"],
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::String),
            example: "GP032",
            order,
        },
        crdt_type: <AsString as FieldTypeMapping>::CRDT_TYPE,
        required: true,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| Some(field_value!(E::code(d).full_id())))),
            write_fn: Some(WriteFn::Bare(|d, v| {
                let s = v.into_string()?;
                match PanelUniqId::parse(&s) {
                    Some(parsed) => {
                        *E::code_mut(d) = parsed;
                        Ok(())
                    }
                    None => Err(ConversionError::ParseError {
                        message: format!("could not parse code {s:?}"),
                    }
                    .into()),
                }
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}
