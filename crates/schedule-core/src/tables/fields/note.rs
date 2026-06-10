/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! The shared `note` fields, parameterised by [`NoteKind`].
//!
//! Several entities carry notes that are *structurally identical* but differ in
//! audience: a public note shown to attendees, internal staff notes, workshop
//! notes, A/V notes. Rather than a bespoke field per note, they are modelled as
//! one concept — a [`NoteKind`] discriminant — stored together in a [`NoteBag`]
//! (`HashMap<NoteKind, String>`) on each entity's `InternalData`.
//!
//! A single generic [`note_field`] builder serves every kind; the kind is
//! carried by a zero-size [`NoteSlot`] marker type (`PublicNote`, … ) because
//! the read/write callbacks are non-capturing function pointers and can only
//! see *generic parameters*, not a runtime `kind` argument. Each entity type
//! declares the subset it supports via
//! [`PanelLike::SUPPORTED_NOTES`](crate::tables::panel_like::PanelLike::SUPPORTED_NOTES).

use std::collections::HashMap;

use crate::field::{CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, WriteFn};
use crate::query::converter::{convert_optional, FieldTypeMapping};
use crate::tables::panel_like::PanelLike;
use crate::value::{FieldCardinality, FieldType, FieldValue};

// ── NoteKind ─────────────────────────────────────────────────────────────────

/// The audience / purpose of a note. Each variant owns its canonical field
/// name, aliases, display string, description, and *printing* semantics
/// (whether it appears in attendee-facing exports).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteKind {
    /// Attendee-facing note, displayed verbatim. The only *printing* kind.
    Public,
    /// Internal note not shown to the public.
    NonPrinting,
    /// Note for workshop staff.
    Workshop,
    /// Audio/visual setup note.
    Av,
}

impl NoteKind {
    /// Every note kind, in canonical order.
    pub const ALL: &'static [NoteKind] = &[
        NoteKind::Public,
        NoteKind::NonPrinting,
        NoteKind::Workshop,
        NoteKind::Av,
    ];

    /// Canonical serialized / field-system name.
    #[must_use]
    pub const fn field_name(self) -> &'static str {
        match self {
            NoteKind::Public => "note",
            NoteKind::NonPrinting => "notes_non_printing",
            NoteKind::Workshop => "workshop_notes",
            NoteKind::Av => "av_notes",
        }
    }

    /// Human-facing display label.
    #[must_use]
    pub const fn display(self) -> &'static str {
        match self {
            NoteKind::Public => "Note",
            NoteKind::NonPrinting => "Notes (Non Printing)",
            NoteKind::Workshop => "Workshop Notes",
            NoteKind::Av => "AV Notes",
        }
    }

    /// One-line field description.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            NoteKind::Public => "Extra note displayed verbatim.",
            NoteKind::NonPrinting => "Internal notes not shown to the public.",
            NoteKind::Workshop => "Notes for workshop staff.",
            NoteKind::Av => "Audio/visual setup notes.",
        }
    }

    /// Alternate names accepted by the field system.
    #[must_use]
    pub const fn aliases(self) -> &'static [&'static str] {
        match self {
            NoteKind::NonPrinting => &["internal_notes"],
            NoteKind::Av => &["av"],
            NoteKind::Public | NoteKind::Workshop => &[],
        }
    }

    /// Example value for documentation.
    #[must_use]
    pub const fn example(self) -> &'static str {
        match self {
            NoteKind::Public => "Vendor hall stays open",
            NoteKind::NonPrinting => "Internal note for staff",
            NoteKind::Workshop => "Staff notes for workshop",
            NoteKind::Av => "Projector needed",
        }
    }

    /// Whether this note appears in attendee-facing (printed) exports.
    #[must_use]
    pub const fn is_printing(self) -> bool {
        matches!(self, NoteKind::Public)
    }
}

// ── NoteSlot markers ──────────────────────────────────────────────────────────

/// Zero-size marker tying a [`note_field`] instantiation to one [`NoteKind`].
///
/// Needed because the field read/write callbacks are non-capturing `fn`
/// pointers: they cannot close over a runtime `kind`, but they *can* reference
/// the generic `K::KIND` associated const.
pub trait NoteSlot: 'static {
    /// The kind this slot selects.
    const KIND: NoteKind;
}

/// Marker for [`NoteKind::Public`].
#[derive(Debug)]
pub struct PublicNote;
impl NoteSlot for PublicNote {
    const KIND: NoteKind = NoteKind::Public;
}

/// Marker for [`NoteKind::NonPrinting`].
#[derive(Debug)]
pub struct NonPrintingNote;
impl NoteSlot for NonPrintingNote {
    const KIND: NoteKind = NoteKind::NonPrinting;
}

/// Marker for [`NoteKind::Workshop`].
#[derive(Debug)]
pub struct WorkshopNote;
impl NoteSlot for WorkshopNote {
    const KIND: NoteKind = NoteKind::Workshop;
}

/// Marker for [`NoteKind::Av`].
#[derive(Debug)]
pub struct AvNote;
impl NoteSlot for AvNote {
    const KIND: NoteKind = NoteKind::Av;
}

// ── NoteBag ────────────────────────────────────────────────────────────────────

/// All of an entity's notes, keyed by [`NoteKind`]. Absent kind == no note.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoteBag(HashMap<NoteKind, String>);

impl NoteBag {
    /// The note for `kind`, if present.
    #[must_use]
    pub fn get(&self, kind: NoteKind) -> Option<&str> {
        self.0.get(&kind).map(String::as_str)
    }

    /// The note for `kind` as an owned `Option<String>` (convenient for export
    /// sinks that take `&Option<String>`).
    #[must_use]
    pub fn get_owned(&self, kind: NoteKind) -> Option<String> {
        self.0.get(&kind).cloned()
    }

    /// Set (or, with `None`, clear) the note for `kind`.
    pub fn set(&mut self, kind: NoteKind, value: Option<String>) {
        match value {
            Some(v) => {
                self.0.insert(kind, v);
            }
            None => {
                self.0.remove(&kind);
            }
        }
    }

    /// True when no notes are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ── note_field builder ──────────────────────────────────────────────────────────

/// One note field, selected by the [`NoteSlot`] marker `K`. The marker `M`
/// chooses the value flavour: [`AsString`](crate::query::converter::AsString)
/// for plain notes (Break/Timeline), [`AsText`](crate::query::converter::AsText)
/// for long prose stored as a CRDT text field (Panel).
#[must_use]
pub const fn note_field<E, K, M>(order: u32) -> FieldDescriptor<E>
where
    E: PanelLike,
    K: NoteSlot,
    M: FieldTypeMapping<Output = String>,
{
    FieldDescriptor {
        data: CommonFieldData {
            name: K::KIND.field_name(),
            display: K::KIND.display(),
            description: K::KIND.description(),
            aliases: K::KIND.aliases(),
            field_type: FieldType(FieldCardinality::Optional, M::FIELD_TYPE_ITEM),
            example: K::KIND.example(),
            order,
        },
        crdt_type: M::CRDT_TYPE,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                E::notes(d)
                    .get(K::KIND)
                    .map(|s| FieldValue::Single(M::to_field_value_item(s.to_string())))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                let value = convert_optional::<M>(v)?;
                E::notes_mut(d).set(K::KIND, value);
                Ok(())
            })),
            add_fn: None,
            remove_fn: None,
        },
    }
}
