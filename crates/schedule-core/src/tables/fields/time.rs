/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! The shared start-instant fields (`time`, `start_time`) and the
//! [`HasStartTime`] capability trait.
//!
//! These read and write a single start instant as an `Option<NaiveDateTime>` —
//! the narrowest timing capability. Timeline stores exactly that natively;
//! Panel/Break project it through their backing [`TimeRange`]. The `end_time` /
//! `duration` fields, which need the full range, live in [`super::duration`].
//!
//! [`TimeRange`]: crate::value::time::TimeRange

use crate::entity::EntityType;
use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, WriteFn};
use crate::field_value;
use crate::value::time::parse_datetime;
use crate::value::{
    ConversionError, FieldCardinality, FieldError, FieldType, FieldTypeItem, FieldValue,
    FieldValueItem,
};
use chrono::NaiveDateTime;

/// Entity types that carry a start instant.
///
/// The accessor is a plain `Option<NaiveDateTime>`: an entity that stores a
/// single instant (Timeline) exposes it directly, while an entity backed by a
/// [`TimeRange`](crate::value::time::TimeRange) (Panel, Break) projects the
/// range's start. Both [`time_field`] and [`start_time_field`] build on this.
pub trait HasStartTime: EntityType {
    /// The start instant, if set.
    fn start_time(d: &Self::InternalData) -> Option<NaiveDateTime>;
    /// Set (or, with `None`, clear) the start instant.
    fn set_start_time(d: &mut Self::InternalData, start: Option<NaiveDateTime>);
}

/// Coerce a field value into an optional start instant. A list or text value
/// clears it; a `DateTime` or parseable `String` sets it.
fn value_to_opt_datetime(v: FieldValue) -> Result<Option<NaiveDateTime>, FieldError> {
    match v {
        FieldValue::List(_) | FieldValue::Single(FieldValueItem::Text(_)) => Ok(None),
        FieldValue::Single(FieldValueItem::DateTime(dt)) => Ok(Some(dt)),
        FieldValue::Single(FieldValueItem::String(s)) => match parse_datetime(&s) {
            Some(dt) => Ok(Some(dt)),
            None => Err(ConversionError::ParseError {
                message: format!("could not parse datetime {s:?}"),
            }
            .into()),
        },
        _ => Err(ConversionError::WrongVariant {
            expected: "DateTime or String",
            got: "other",
        }
        .into()),
    }
}

/// Shared builder for the start-instant fields; `time_field` / `start_time_field`
/// differ only in their metadata.
#[must_use]
const fn start_instant_field<E: HasStartTime>(
    name: &'static str,
    display: &'static str,
    description: &'static str,
    aliases: &'static [&'static str],
    example: &'static str,
    order: u32,
) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name,
            display,
            description,
            aliases,
            field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::DateTime),
            example,
            order,
        },
        crdt_type: crate::crdt::CrdtFieldType::Scalar,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                E::start_time(d).map(|dt| field_value!(dt))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                E::set_start_time(d, value_to_opt_datetime(v)?);
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}

/// `time` — a single instant. Used by Timeline, which stores only an
/// `Option<NaiveDateTime>`; it carries no end or duration.
#[must_use]
pub const fn time_field<E: HasStartTime>(
    order: u32,
    aliases: &'static [&'static str],
) -> FieldDescriptor<E> {
    start_instant_field(
        "time",
        "Time",
        "Time point.",
        aliases,
        "2026-01-01T09:00:00",
        order,
    )
}

/// `start_time` — the start instant of a duration-carrying entity.
#[must_use]
pub const fn start_time_field<E: HasStartTime>(order: u32) -> FieldDescriptor<E> {
    start_instant_field(
        "start_time",
        "Start Time",
        "Start time.",
        &["start", "time"],
        "2026-06-26T12:00:00",
        order,
    )
}
