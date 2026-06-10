/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! The shared timing fields (`time`, `start_time`, `end_time`, `duration`).
//!
//! All four read and write through the virtual [`TimeRange`] exposed by
//! [`PanelLikeTimed`], so the backing storage (a real `TimeRange`, or a single
//! `Option<NaiveDateTime>` for Timeline) is irrelevant to the field logic.

use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, WriteFn};
use crate::field_value;
use crate::tables::panel_like::PanelLikeTimed;
use crate::value::time::{parse_datetime, parse_duration};
use crate::value::{
    ConversionError, FieldCardinality, FieldType, FieldTypeItem, FieldValue, FieldValueItem,
};
use chrono::Duration;

/// `time` — a single instant, projected from the `start_time` of the virtual
/// [`TimeRange`]. Used by Timeline, which stores only an `Option<NaiveDateTime>`
/// and synthesises a start-only range; it carries no end or duration.
#[must_use]
pub const fn time_field<E: PanelLikeTimed>(
    order: u32,
    aliases: &'static [&'static str],
) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "time",
            display: "Time",
            description: "Time point.",
            aliases,
            field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::DateTime),
            example: "2026-01-01T09:00:00",
            order,
        },
        crdt_type: crate::crdt::CrdtFieldType::Scalar,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                E::time_slot(d).start_time().map(|dt| field_value!(dt))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                let mut ts = E::time_slot(d);
                match v {
                    FieldValue::List(_) | FieldValue::Single(FieldValueItem::Text(_)) => {
                        ts.remove_start_time()
                    }
                    FieldValue::Single(FieldValueItem::DateTime(dt)) => ts.add_start_time(dt),
                    FieldValue::Single(FieldValueItem::String(s)) => match parse_datetime(&s) {
                        Some(dt) => ts.add_start_time(dt),
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
                E::set_time_slot(d, ts);
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}

/// `start_time` — projected from the virtual [`TimeRange`].
#[must_use]
pub const fn start_time_field<E: PanelLikeTimed>(order: u32) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "start_time",
            display: "Start Time",
            description: "Start time.",
            aliases: &["start", "time"],
            field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::DateTime),
            example: "2026-06-26T12:00:00",
            order,
        },
        crdt_type: crate::crdt::CrdtFieldType::Scalar,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                E::time_slot(d).start_time().map(|dt| field_value!(dt))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                let mut ts = E::time_slot(d);
                match v {
                    FieldValue::List(_) | FieldValue::Single(FieldValueItem::Text(_)) => {
                        ts.remove_start_time()
                    }
                    FieldValue::Single(FieldValueItem::DateTime(dt)) => ts.add_start_time(dt),
                    FieldValue::Single(FieldValueItem::String(s)) => match parse_datetime(&s) {
                        Some(dt) => ts.add_start_time(dt),
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
                E::set_time_slot(d, ts);
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}

/// `end_time` — projected from the virtual [`TimeRange`].
#[must_use]
pub const fn end_time_field<E: PanelLikeTimed>(order: u32) -> FieldDescriptor<E> {
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
                E::time_slot(d).end_time().map(|dt| field_value!(dt))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                let mut ts = E::time_slot(d);
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
                E::set_time_slot(d, ts);
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}

/// `duration` — projected from the virtual [`TimeRange`].
#[must_use]
pub const fn duration_field<E: PanelLikeTimed>(order: u32) -> FieldDescriptor<E> {
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
                E::time_slot(d).duration().map(|dur| field_value!(dur))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                let mut ts = E::time_slot(d);
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
                E::set_time_slot(d, ts);
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}
