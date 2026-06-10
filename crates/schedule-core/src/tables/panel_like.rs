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
//! - The shared field *definitions* themselves live in [`crate::tables::fields`]
//!   (one `const fn` builder per field, generic over these traits); entity
//!   modules instantiate them as per-type statics (with their own
//!   `order`/`aliases`) and register them through the usual `inventory::submit!`.

use crate::entity::EntityType;
use crate::tables::fields::note::{NoteBag, NoteKind};
use crate::value::time::TimeRange;
use crate::value::uniq_id::PanelUniqId;

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

    /// The note kinds this entity type supports. Panel carries the full set;
    /// Break/Timeline carry only [`NoteKind::Public`]. Used to scope which
    /// [`note_field`](crate::tables::fields::note::note_field)s an entity wires
    /// up (and, in future, to validate writes).
    const SUPPORTED_NOTES: &'static [NoteKind];

    fn name(d: &Self::InternalData) -> &String;
    fn name_mut(d: &mut Self::InternalData) -> &mut String;
    fn description(d: &Self::InternalData) -> &Option<String>;
    fn description_mut(d: &mut Self::InternalData) -> &mut Option<String>;
    /// All notes for this entity, keyed by [`NoteKind`].
    fn notes(d: &Self::InternalData) -> &NoteBag;
    fn notes_mut(d: &mut Self::InternalData) -> &mut NoteBag;
    fn code(d: &Self::InternalData) -> &PanelUniqId;
    fn code_mut(d: &mut Self::InternalData) -> &mut PanelUniqId;

    /// Whether `kind` is in [`Self::SUPPORTED_NOTES`].
    #[must_use]
    fn supports_note(kind: NoteKind) -> bool {
        Self::SUPPORTED_NOTES.contains(&kind)
    }
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
