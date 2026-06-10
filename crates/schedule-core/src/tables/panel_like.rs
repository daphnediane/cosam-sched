/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Shared "panel-like" abstraction for [`Panel`](crate::tables::panel),
//! [`Break`](crate::tables::breaks), and [`Timeline`](crate::tables::timeline).
//!
//! These three entity types share most of their shape: a parsed Uniq ID
//! (`code`), a `name` / `description` / `note`, a panel-type edge, and timing.
//! Rather than collapse them into one type (their distinct
//! [`EntityId`](crate::entity::EntityId)s and `InternalData` `TypeId`s are
//! load-bearing for storage and `Schedule::identify`), this module lets the
//! *differences* stay virtual:
//!
//! - [`EventKind`] is the internal "mode" (Panel / Break / Timeline).
//! - [`PanelLike`] (and [`PanelLikeTimed`] for the duration-carrying kinds)
//!   expose the shared fields generically by redirecting into whatever
//!   common-data each entity already stores — no shared storage struct is
//!   imposed — so a single field descriptor can be **defined once and used by
//!   every panel-like entity type**.
//! - The `*_field` `const fn`s below each build one [`FieldDescriptor`] generic
//!   over the entity type; entity modules instantiate them as per-type statics
//!   (with their own `order`/`aliases`) and register them through the usual
//!   `inventory::submit!`.

use crate::entity::EntityType;
use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, WriteFn};
use crate::field_value;
use crate::query::converter::{convert_optional, convert_required, AsString, FieldTypeMapping};
use crate::value::time::{parse_datetime, parse_duration, TimeRange};
use crate::value::uniq_id::PanelUniqId;
use crate::value::{
    ConversionError, FieldCardinality, FieldType, FieldTypeItem, FieldValue, FieldValueItem,
};
use chrono::Duration;

// ── EventKind ──────────────────────────────────────────────────────────────────

/// The "mode" of a panel-like entity — which of the three kinds it is.
///
/// Each [`PanelLike`] type reports its kind as [`PanelLike::KIND`]. Generic code
/// that handles all three uniformly can branch on this when behaviour must
/// differ (e.g. Timeline is a single instant, Break/Panel carry a duration).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    Panel,
    Break,
    Timeline,
}

// ── PanelLike traits ───────────────────────────────────────────────────────────

/// Common interface over the three panel-like entity types.
///
/// Implemented by `PanelEntityType`, `BreakEntityType`, and `TimelineEntityType`.
/// The accessors redirect into whatever common-data each type already stores
/// (each owns its own `*CommonData` struct), so the generic `*_field` builders
/// below can read/write the shared fields without knowing the concrete type —
/// and without forcing any type into a shared storage struct.
pub trait PanelLike: EntityType {
    /// Which kind of panel-like entity this is.
    const KIND: EventKind;

    fn name(d: &Self::InternalData) -> &String;
    fn name_mut(d: &mut Self::InternalData) -> &mut String;
    fn description(d: &Self::InternalData) -> &Option<String>;
    fn description_mut(d: &mut Self::InternalData) -> &mut Option<String>;
    fn note(d: &Self::InternalData) -> &Option<String>;
    fn note_mut(d: &mut Self::InternalData) -> &mut Option<String>;
    fn code(d: &Self::InternalData) -> &PanelUniqId;
    fn code_mut(d: &mut Self::InternalData) -> &mut PanelUniqId;
}

/// Extension of [`PanelLike`] that exposes timing as a *virtual* [`TimeRange`].
///
/// Implementors store time however is natural — Break/Panel hold a real
/// `TimeRange`, Timeline holds a single `Option<NaiveDateTime>` and
/// wraps/unwraps it here — and the get/set pair presents a uniform interface.
/// The timing field builders read–modify–write through these, so no type is
/// forced to store a `TimeRange` it does not want.
pub trait PanelLikeTimed: PanelLike {
    /// The current timing as a [`TimeRange`] (synthesised if stored otherwise).
    fn time_slot(d: &Self::InternalData) -> TimeRange;
    /// Replace the timing from a [`TimeRange`] (projected back to storage).
    fn set_time_slot(d: &mut Self::InternalData, time_slot: TimeRange);
}

// ── Shared field builders ──────────────────────────────────────────────────────
//
// Each returns a `FieldDescriptor<E>` for one shared field. They are `const fn`
// so the result can be assigned to a per-type `static`, keeping the existing
// `inventory::submit!` registration. Conversion semantics mirror
// `accessor_field_properties!` / `callback_field_properties!` exactly.

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

/// `description` — optional text. The marker `M` selects the value flavour:
/// [`AsString`] for Break/Timeline, [`AsText`](crate::query::converter::AsText)
/// for Panel (long prose, stored as a CRDT text field).
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

/// `note` — optional text displayed verbatim. `M` selects the value flavour as
/// for [`description_field`].
#[must_use]
pub const fn note_field<E: PanelLike, M: FieldTypeMapping<Output = String>>(
    order: u32,
    aliases: &'static [&'static str],
) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "note",
            display: "Note",
            description: "Extra note displayed verbatim.",
            aliases,
            field_type: FieldType(FieldCardinality::Optional, M::FIELD_TYPE_ITEM),
            example: "Vendor hall stays open",
            order,
        },
        crdt_type: M::CRDT_TYPE,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                E::note(d)
                    .as_ref()
                    .map(|x| FieldValue::Single(M::to_field_value_item(x.clone())))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                *E::note_mut(d) = convert_optional::<M>(v)?;
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}

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

// ── Unified panel-like view & lookup ─────────────────────────────────────────

use crate::entity::RuntimeEntityId;
use crate::schedule::Schedule;
use crate::tables::breaks::{BreakEntityType, BreakInternalData};
use crate::tables::panel::{PanelEntityType, PanelInternalData};
use crate::tables::timeline::{TimelineEntityType, TimelineInternalData};

/// A borrowed view over any one of the three panel-like entities.
///
/// Lets callers treat a Panel, Break, or Timeline uniformly when the kind does
/// not matter (shared fields, timing), while still being able to match on the
/// variant when it does. Obtained from [`Schedule::iter_panel_like`].
#[derive(Debug, Clone, Copy)]
pub enum PanelLikeRef<'a> {
    Panel(&'a PanelInternalData),
    Break(&'a BreakInternalData),
    Timeline(&'a TimelineInternalData),
}

impl PanelLikeRef<'_> {
    /// Which kind of panel-like entity this is.
    #[must_use]
    pub fn kind(&self) -> EventKind {
        match self {
            Self::Panel(_) => EventKind::Panel,
            Self::Break(_) => EventKind::Break,
            Self::Timeline(_) => EventKind::Timeline,
        }
    }

    /// The parsed Uniq ID.
    #[must_use]
    pub fn code(&self) -> &PanelUniqId {
        match self {
            Self::Panel(d) => PanelEntityType::code(d),
            Self::Break(d) => BreakEntityType::code(d),
            Self::Timeline(d) => TimelineEntityType::code(d),
        }
    }

    /// The shared `name`.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Panel(d) => PanelEntityType::name(d),
            Self::Break(d) => BreakEntityType::name(d),
            Self::Timeline(d) => TimelineEntityType::name(d),
        }
    }

    /// The shared optional `description`.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        match self {
            Self::Panel(d) => PanelEntityType::description(d).as_deref(),
            Self::Break(d) => BreakEntityType::description(d).as_deref(),
            Self::Timeline(d) => TimelineEntityType::description(d).as_deref(),
        }
    }

    /// The shared optional `note`.
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        match self {
            Self::Panel(d) => PanelEntityType::note(d).as_deref(),
            Self::Break(d) => BreakEntityType::note(d).as_deref(),
            Self::Timeline(d) => TimelineEntityType::note(d).as_deref(),
        }
    }

    /// Timing as a [`TimeRange`]. All three kinds expose this (Timeline as a
    /// start-only range); see [`PanelLikeTimed`].
    #[must_use]
    pub fn time_slot(&self) -> TimeRange {
        match self {
            Self::Panel(d) => PanelEntityType::time_slot(d),
            Self::Break(d) => BreakEntityType::time_slot(d),
            Self::Timeline(d) => TimelineEntityType::time_slot(d),
        }
    }
}

impl Schedule {
    /// Iterate every panel-like entity (Panel, Break, Timeline) uniformly,
    /// yielding `(RuntimeEntityId, PanelLikeRef)` pairs.
    pub fn iter_panel_like(&self) -> impl Iterator<Item = (RuntimeEntityId, PanelLikeRef<'_>)> {
        let panels = self
            .iter_entities::<PanelEntityType>()
            .map(|(id, d)| (id.into(), PanelLikeRef::Panel(d)));
        let breaks = self
            .iter_entities::<BreakEntityType>()
            .map(|(id, d)| (id.into(), PanelLikeRef::Break(d)));
        let timelines = self
            .iter_entities::<TimelineEntityType>()
            .map(|(id, d)| (id.into(), PanelLikeRef::Timeline(d)));
        panels.chain(breaks).chain(timelines)
    }

    /// Resolve a Uniq ID code to a panel-like entity, regardless of kind
    /// (case-insensitive). Returns the first match; in well-formed data codes
    /// are unique across the three kinds.
    #[must_use]
    pub fn find_panel_like_by_code(&self, code: &str) -> Option<RuntimeEntityId> {
        let upper = code.to_uppercase();
        self.iter_panel_like().find_map(|(id, r)| {
            (r.code().full_id().to_uppercase() == upper).then_some(id)
        })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::entity::EntityTyped;
    use crate::schedule::Schedule;
    use crate::tables::breaks::BreakBuilder;
    use crate::tables::panel_like::EventKind;
    use crate::tables::timeline::TimelineBuilder;

    #[test]
    fn find_panel_like_by_code_across_kinds() {
        let mut sched = Schedule::default();
        BreakBuilder::new()
            .with_code("BREAK001")
            .with_name("Lunch")
            .build(&mut sched)
            .unwrap();
        TimelineBuilder::new()
            .with_code("TL001")
            .with_name("Opening")
            .build(&mut sched)
            .unwrap();

        // Case-insensitive lookup resolves regardless of kind.
        let brk = sched.find_panel_like_by_code("break001").unwrap();
        assert_eq!(brk.entity_type_name(), "break");
        let tl = sched.find_panel_like_by_code("TL001").unwrap();
        assert_eq!(tl.entity_type_name(), "timeline");
        assert!(sched.find_panel_like_by_code("ZZ999").is_none());
    }

    #[test]
    fn iter_panel_like_exposes_shared_fields_uniformly() {
        let mut sched = Schedule::default();
        BreakBuilder::new()
            .with_code("BREAK001")
            .with_name("Lunch")
            .build(&mut sched)
            .unwrap();

        let (_, r) = sched
            .iter_panel_like()
            .find(|(_, r)| r.kind() == EventKind::Break)
            .unwrap();
        assert_eq!(r.name(), "Lunch");
        assert_eq!(r.code().full_id(), "BREAK001");
    }
}
