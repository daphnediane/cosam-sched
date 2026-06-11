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
//! - [`PanelLike`] and the per-field capability traits it builds on
//!   ([`HasName`], [`HasDescription`], [`HasNotes`], plus [`HasStartTime`] /
//!   [`HasDuration`] for timing)
//!   expose the shared fields generically by redirecting into whatever
//!   common-data each entity already stores — no shared storage struct is
//!   imposed — so a single field descriptor can be **defined once and used by
//!   every panel-like entity type**.
//! - The shared field *definitions* themselves live in [`crate::tables::fields`]
//!   (one `const fn` builder per field, generic over these traits); entity
//!   modules instantiate them as per-type statics (with their own
//!   `order`/`aliases`) and register them through the usual `inventory::submit!`.

use crate::tables::fields::description::HasDescription;
use crate::tables::fields::duration::HasDuration;
use crate::tables::fields::name::HasName;
use crate::tables::fields::note::{HasNotes, NoteBag, NoteKind};
use crate::tables::fields::time::HasStartTime;
use crate::value::time::TimeRange;
use crate::value::uniq_id::PanelUniqId;

// ── EventKind ──────────────────────────────────────────────────────────────────

/// The "mode" of a panel-like entity — which of the three kinds it is.
///
/// Each [`PanelLike`] type reports its kind as [`PanelLike::KIND`]. Generic code
/// that handles all three uniformly can branch on this when behavior must
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
/// The shared accessors live on focused capability traits that any entity can
/// implement: [`HasName`] / [`HasDescription`] / [`HasNotes`], plus
/// [`HasStartTime`] — every panel-like entity is placed at a start instant.
/// `PanelLike` is their intersection plus what is genuinely panel-like-only: the
/// [`EventKind`] discriminant and the parsed Uniq ID (`code`). (Duration is *not*
/// here: a Timeline is a single instant, so [`HasDuration`] stays an opt-in that
/// only Panel and Break implement.) The accessors redirect into whatever
/// common-data each type already stores, so the generic `*_field` builders can
/// read/write the shared fields without knowing the concrete type — and without
/// forcing any type into a shared storage struct.
pub trait PanelLike: HasName + HasDescription + HasNotes + HasStartTime {
    /// Which kind of panel-like entity this is.
    const KIND: EventKind;

    /// The parsed Uniq ID (`code`) — panel-like entities are identified by a
    /// [`PanelUniqId`]; other entity types are not.
    fn code(d: &Self::InternalData) -> &PanelUniqId;
    /// Mutable access to the parsed Uniq ID.
    fn code_mut(d: &mut Self::InternalData) -> &mut PanelUniqId;
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

    /// All notes for this entity, keyed by [`NoteKind`].
    #[must_use]
    pub fn notes(&self) -> &NoteBag {
        match self {
            Self::Panel(d) => PanelEntityType::notes(d),
            Self::Break(d) => BreakEntityType::notes(d),
            Self::Timeline(d) => TimelineEntityType::notes(d),
        }
    }

    /// The shared optional public ([`NoteKind::Public`]) note.
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        self.notes().get(NoteKind::Public)
    }

    /// Timing as a [`TimeRange`]. Panel/Break expose their full range
    /// ([`HasDuration`]); a Timeline has no duration, so its single instant
    /// ([`HasStartTime`]) is presented here as a start-only range.
    #[must_use]
    pub fn time_slot(&self) -> TimeRange {
        match self {
            Self::Panel(d) => PanelEntityType::time_range(d),
            Self::Break(d) => BreakEntityType::time_range(d),
            Self::Timeline(d) => {
                let mut ts = TimeRange::default();
                if let Some(t) = TimelineEntityType::start_time(d) {
                    ts.add_start_time(t);
                }
                ts
            }
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
        self.iter_panel_like()
            .find_map(|(id, r)| (r.code().full_id().to_uppercase() == upper).then_some(id))
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
