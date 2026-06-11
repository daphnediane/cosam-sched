/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! The shared `name` field and the [`HasName`] capability trait.

use crate::entity::EntityType;
use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, WriteFn};
use crate::query::converter::{convert_required, AsString, FieldTypeMapping};
use crate::value::{FieldCardinality, FieldType, FieldTypeItem, FieldValue};

/// Entity types that carry a required `name` string.
///
/// This is the narrowest capability the [`name_field`] builder needs: an entity
/// opts in by redirecting these two accessors into wherever its `InternalData`
/// already stores a display name — no shared storage struct, no new field. It is
/// deliberately *not* tied to [`PanelLike`](crate::tables::panel_like::PanelLike):
/// any entity (presenter, room, …) can implement it and reuse the one `name`
/// definition.
pub trait HasName: EntityType {
    /// The stored name.
    fn name(d: &Self::InternalData) -> &String;
    /// Mutable access to the stored name.
    fn name_mut(d: &mut Self::InternalData) -> &mut String;
}

/// `name` — required, single string — with caller-supplied `display` /
/// `description` / `example`.
///
/// Every name-bearing entity uses the canonical key `"name"`; types whose name
/// reads differently in the UI (a presenter's "Presenter or group display
/// name.", a room's "Room Name") use this form to override that metadata — and
/// pass their old field name (`room_name`, `hotel_room_name`, `panel_kind`, …)
/// as an alias so existing key-based lookups keep resolving — while still
/// reusing the one shared read/write logic. [`HasName`] redirects the accessor
/// into whatever struct field actually stores the name.
#[must_use]
pub const fn name_field_described<E: HasName>(
    order: u32,
    aliases: &'static [&'static str],
    display: &'static str,
    description: &'static str,
    example: &'static str,
) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "name",
            display,
            description,
            aliases,
            field_type: FieldType(FieldCardinality::Single, FieldTypeItem::String),
            example,
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

/// `name` — required, single string — with the generic display/description/example.
#[must_use]
pub const fn name_field<E: HasName>(
    order: u32,
    aliases: &'static [&'static str],
) -> FieldDescriptor<E> {
    name_field_described(order, aliases, "Name", "Name / title.", "Opening Ceremony")
}
