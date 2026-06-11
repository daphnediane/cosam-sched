/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! The shared `end_time` and `duration` fields and the [`HasDuration`]
//! capability trait.
//!
//! Both read–modify–write the full [`TimeRange`] exposed by [`HasDuration`]:
//! the duration is the gap between start and end, so editing either end of the
//! range needs the whole thing. Only the duration-carrying kinds (Panel, Break)
//! opt in; a Timeline is a single instant (see [`super::time`]) and carries no
//! end or duration.

use crate::entity::EntityType;
use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, WriteFn};
use crate::field_value;
use crate::value::time::{parse_datetime, parse_duration, TimeRange};
use crate::value::{
    ConversionError, FieldCardinality, FieldType, FieldTypeItem, FieldValue, FieldValueItem,
};
use chrono::Duration;

/// Entity types that carry a duration — a full [`TimeRange`] (start plus end or
/// duration), not just an instant.
///
/// The accessor is the whole `TimeRange` because `end_time` / `duration` are
/// read–modify–write against it. An entity stores it however is natural
/// (Panel/Break hold a real `TimeRange`); a single-instant entity (Timeline)
/// simply does not implement this trait.
pub trait HasDuration: EntityType {
    /// The current timing as a [`TimeRange`].
    fn time_range(d: &Self::InternalData) -> TimeRange;
    /// Replace the timing from a [`TimeRange`].
    fn set_time_range(d: &mut Self::InternalData, time_range: TimeRange);
}

/// `end_time` — projected from the [`TimeRange`] of a duration-carrying entity.
#[must_use]
pub const fn end_time_field<E: HasDuration>(order: u32) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "end_time",
            display: "End Time",
            description: "End time.",
            aliases: &["end"],
            field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::DateTime),
            example: "2026-06-26T13:00:00",
            order,
        },
        crdt_type: crate::crdt::CrdtFieldType::Scalar,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                E::time_range(d).end_time().map(|dt| field_value!(dt))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                let mut ts = E::time_range(d);
                match v {
                    FieldValue::List(_) | FieldValue::Single(FieldValueItem::Text(_)) => {
                        ts.remove_end_time()
                    }
                    FieldValue::Single(FieldValueItem::DateTime(dt)) => ts.add_end_time(dt),
                    FieldValue::Single(FieldValueItem::String(s)) => match parse_datetime(&s) {
                        Some(dt) => ts.add_end_time(dt),
                        None => {
                            return Err(ConversionError::ParseError {
                                message: format!("could not parse datetime {s:?}"),
                            }
                            .into())
                        }
                    },
                    _ => {
                        return Err(ConversionError::WrongVariant {
                            expected: "DateTime or String",
                            got: "other",
                        }
                        .into())
                    }
                }
                E::set_time_range(d, ts);
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}

/// `duration` — projected from the [`TimeRange`] of a duration-carrying entity.
#[must_use]
pub const fn duration_field<E: HasDuration>(order: u32) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "duration",
            display: "Duration",
            description: "Duration.",
            aliases: &[],
            field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Duration),
            example: "60",
            order,
        },
        crdt_type: crate::crdt::CrdtFieldType::Scalar,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                E::time_range(d).duration().map(|dur| field_value!(dur))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                let mut ts = E::time_range(d);
                match v {
                    FieldValue::List(_) | FieldValue::Single(FieldValueItem::Text(_)) => {
                        ts.remove_duration()
                    }
                    FieldValue::Single(FieldValueItem::Duration(dur)) => ts.add_duration(dur),
                    FieldValue::Single(FieldValueItem::Integer(m)) => {
                        ts.add_duration(Duration::minutes(m))
                    }
                    FieldValue::Single(FieldValueItem::String(s)) => match parse_duration(&s) {
                        Some(dur) => ts.add_duration(dur),
                        None => {
                            return Err(ConversionError::ParseError {
                                message: format!("could not parse duration {s:?}"),
                            }
                            .into())
                        }
                    },
                    _ => {
                        return Err(ConversionError::WrongVariant {
                            expected: "Duration, Integer, or String",
                            got: "other",
                        }
                        .into())
                    }
                }
                E::set_time_range(d, ts);
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}
