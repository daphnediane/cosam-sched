/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! The shared `code` (Uniq ID) field, its `old_codes` history sibling, and the
//! [`CodeHistory`] value type that ties the two together.
//!
//! A panel-like entity is identified by a [`PanelUniqId`] (`code`) and remembers
//! the Uniq IDs it previously held (the `Old Uniq Id` history). Because writing a
//! new code and recording the prior one are a single logical operation, both are
//! stored in one [`CodeHistory`] value and surfaced through one [`HasCode`]
//! capability. The current code is kept decoded (it is read constantly — full
//! id, prefix, part/session); the history is kept as canonical id strings and
//! only decoded on demand (it is rarely inspected).

use crate::entity::EntityType;
use crate::field::{
    AddFn, CommonFieldData, FieldCallbacks, FieldDescriptor, ReadFn, RemoveFn, WriteFn,
};
use crate::field_value;
use crate::query::converter::{AsString, FieldTypeMapping};
use crate::value::uniq_id::PanelUniqId;
use crate::value::{
    ConversionError, FieldCardinality, FieldError, FieldType, FieldTypeItem, FieldValue,
    FieldValueItem,
};

// ── CodeHistory ─────────────────────────────────────────────────────────────

/// An entity's current Uniq ID together with the stack of codes it previously
/// held.
///
/// Conceptually a stack: the top is the *current* code (which may be absent),
/// and beneath it is the history of vacated codes, oldest first (the
/// most-recently-vacated code is last). Two invariants hold across every
/// mutation:
///
/// - **No duplicates:** a code appears at most once across the current slot and
///   the history — never simultaneously live and historical.
/// - **History holds vacated codes only:** setting a new current pushes the
///   prior current down into the history; re-assigning a code already in the
///   history *reclaims* it (drops the historical copy) instead of duplicating
///   it.
///
/// The history is *not* a reservation: entries never block another entity from
/// being assigned a code.
///
/// The current code is stored decoded ([`PanelUniqId`]) because it is read
/// constantly; the history is stored as canonical id strings and decoded only
/// when needed via [`CodeHistory::old_codes_decoded`].
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CodeHistory {
    /// The current Uniq ID, or `None` when no code is assigned.
    current: Option<PanelUniqId>,
    /// Previously-held codes as canonical id strings, oldest first. Never
    /// contains the current code.
    history: Vec<String>,
}

impl CodeHistory {
    /// A history whose only code is `current`, with no past codes.
    #[must_use]
    pub fn new(current: PanelUniqId) -> Self {
        Self {
            current: Some(current),
            history: Vec::new(),
        }
    }

    /// The current code, if any.
    #[must_use]
    pub fn current(&self) -> Option<&PanelUniqId> {
        self.current.as_ref()
    }

    /// Whether a current code is assigned.
    #[must_use]
    pub fn has_code(&self) -> bool {
        self.current.is_some()
    }

    /// Previously-held codes as canonical id strings, oldest first.
    #[must_use]
    pub fn old_codes(&self) -> &[String] {
        &self.history
    }

    /// Previously-held codes, decoded on demand, oldest first.
    pub fn old_codes_decoded(&self) -> impl Iterator<Item = PanelUniqId> + '_ {
        self.history.iter().filter_map(|s| PanelUniqId::parse(s))
    }

    /// The most-recently-vacated previous code string, if any.
    #[must_use]
    pub fn previous(&self) -> Option<&str> {
        self.history.last().map(String::as_str)
    }

    /// Set the current code, maintaining the stack invariants.
    ///
    /// Reclaims `new` from the history if present, then pushes the prior current
    /// (if any) onto the history. Returns the code displaced from the current
    /// slot (the new "previous"), or `None` if nothing changed.
    pub fn set_current(&mut self, new: Option<PanelUniqId>) -> Option<PanelUniqId> {
        if self.current == new {
            return None;
        }
        // Reclaim: a returning code must not also remain in the history.
        if let Some(ref n) = new {
            let n_id = n.full_id();
            self.history.retain(|c| *c != n_id);
        }
        let displaced = self.current.take();
        if let Some(ref prev) = displaced {
            self.history.push(prev.full_id());
        }
        self.current = new;
        displaced
    }

    /// Replace the history wholesale (used when loading persisted state).
    ///
    /// Drops any entry equal to the current code and any duplicates, preserving
    /// the no-duplicate invariant and the given order otherwise.
    pub fn set_old_codes(&mut self, codes: Vec<String>) {
        let current_id = self.current.as_ref().map(PanelUniqId::full_id);
        let mut seen = std::collections::HashSet::new();
        self.history = codes
            .into_iter()
            .filter(|c| Some(c) != current_id.as_ref())
            .filter(|c| seen.insert(c.clone()))
            .collect();
    }

    /// Append codes to the history, preserving the stack invariants.
    ///
    /// Each code is appended (oldest-first order) unless it is already the
    /// current code — adding the current code is ignored, exactly as if
    /// [`set_old_codes`](Self::set_old_codes) were given a list containing it —
    /// or already present in the history (no duplicates).
    pub fn add_old_codes(&mut self, codes: Vec<String>) {
        let current_id = self.current.as_ref().map(PanelUniqId::full_id);
        for code in codes {
            if Some(&code) == current_id.as_ref() || self.history.contains(&code) {
                continue;
            }
            self.history.push(code);
        }
    }

    /// Remove the given codes from the history. Codes not present are ignored.
    pub fn remove_old_codes(&mut self, codes: &[String]) {
        self.history.retain(|c| !codes.contains(c));
    }

    // ── Delegating accessors to the current code ──────────────────────────────

    /// Full canonical id of the current code, or `""` when unset.
    #[must_use]
    pub fn full_id(&self) -> String {
        self.current
            .as_ref()
            .map(PanelUniqId::full_id)
            .unwrap_or_default()
    }

    /// Canonical base id of the current code, or `""` when unset.
    #[must_use]
    pub fn base_id(&self) -> String {
        self.current
            .as_ref()
            .map(PanelUniqId::base_id)
            .unwrap_or_default()
    }

    /// Normalized panel-type lookup prefix of the current code, or `""`.
    #[must_use]
    pub fn type_prefix(&self) -> &str {
        self.current.as_ref().map_or("", |c| c.type_prefix())
    }

    /// Part number of the current code, if any.
    #[must_use]
    pub fn part_num(&self) -> Option<u32> {
        self.current.as_ref().and_then(|c| c.part_num)
    }

    /// Session number of the current code, if any.
    #[must_use]
    pub fn session_num(&self) -> Option<u32> {
        self.current.as_ref().and_then(|c| c.session_num)
    }
}

impl From<PanelUniqId> for CodeHistory {
    fn from(current: PanelUniqId) -> Self {
        Self::new(current)
    }
}

// ── HasCode capability ────────────────────────────────────────────────────────

/// Entity types identified by a Uniq ID with a history of prior codes.
pub trait HasCode: EntityType {
    /// The entity's code history (current + previously-held codes).
    fn code(d: &Self::InternalData) -> &CodeHistory;
    /// Mutable access to the entity's code history.
    fn code_mut(d: &mut Self::InternalData) -> &mut CodeHistory;
}

// ── Field builders ──────────────────────────────────────────────────────────

/// `code` (Uniq ID) — the current code, exposed as a string.
#[must_use]
pub const fn code_field<E: HasCode>(order: u32) -> FieldDescriptor<E> {
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
                        E::code_mut(d).set_current(Some(parsed));
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

/// Parse a single field item into a canonical Uniq ID string.
fn item_to_code_string(item: &FieldValueItem) -> Result<String, FieldError> {
    let s = match item {
        FieldValueItem::String(s) | FieldValueItem::Text(s) => s.as_str(),
        _ => {
            return Err(ConversionError::WrongVariant {
                expected: "String",
                got: "other",
            }
            .into())
        }
    };
    PanelUniqId::parse(s).map(|c| c.full_id()).ok_or_else(|| {
        ConversionError::ParseError {
            message: format!("could not parse Uniq ID {s:?}"),
        }
        .into()
    })
}

/// Coerce a field value into a list of canonical code strings. A `List` maps
/// element-wise; a single `String`/`Text` becomes a one-element list.
fn value_to_code_strings(v: FieldValue) -> Result<Vec<String>, FieldError> {
    match v {
        FieldValue::List(items) => items.iter().map(item_to_code_string).collect(),
        FieldValue::Single(item) => Ok(vec![item_to_code_string(&item)?]),
    }
}

/// `old_codes` — the list of previously-held Uniq IDs for a panel-like entity.
///
/// Stored as a CRDT list of canonical id strings (e.g. `["GP001", "BR003"]`).
/// The list is *history*, not a reservation: entries never block another entity
/// from being assigned a code.
#[must_use]
pub const fn old_codes_field<E: HasCode>(order: u32) -> FieldDescriptor<E> {
    FieldDescriptor {
        data: CommonFieldData {
            name: "old_codes",
            display: "Old Uniq Id",
            description: "Previously-held Uniq IDs for this entity (history).",
            aliases: &["old_uniq_id", "old_code", "old_id"],
            field_type: FieldType(FieldCardinality::List, FieldTypeItem::String),
            example: "[\"GP001\"]",
            order,
        },
        crdt_type: crate::crdt::CrdtFieldType::List,
        required: false,
        cb: FieldCallbacks {
            read_fn: Some(ReadFn::Bare(|d| {
                let items: Vec<FieldValueItem> = E::code(d)
                    .old_codes()
                    .iter()
                    .map(|c| FieldValueItem::String(c.clone()))
                    .collect();
                Some(FieldValue::List(items))
            })),
            write_fn: Some(WriteFn::Bare(|d, v| {
                E::code_mut(d).set_old_codes(value_to_code_strings(v)?);
                Ok(())
            })),
            add_fn: Some(AddFn::Bare(|d, v| {
                E::code_mut(d).add_old_codes(value_to_code_strings(v)?);
                Ok(())
            })),
            remove_fn: Some(RemoveFn::Bare(|d, v| {
                E::code_mut(d).remove_old_codes(&value_to_code_strings(v)?);
                Ok(())
            })),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(s: &str) -> PanelUniqId {
        PanelUniqId::parse(s).unwrap()
    }

    #[test]
    fn new_has_current_and_no_history() {
        let h = CodeHistory::new(pid("GP001"));
        assert_eq!(h.full_id(), "GP001");
        assert!(h.old_codes().is_empty());
        assert_eq!(h.previous(), None);
    }

    #[test]
    fn set_current_pushes_prior_into_history() {
        let mut h = CodeHistory::new(pid("GP001"));
        let displaced = h.set_current(Some(pid("GP002")));
        assert_eq!(
            displaced.as_ref().map(PanelUniqId::full_id),
            Some("GP001".into())
        );
        assert_eq!(h.full_id(), "GP002");
        assert_eq!(h.old_codes(), &["GP001".to_string()]);
        assert_eq!(h.previous(), Some("GP001"));
    }

    #[test]
    fn set_current_same_code_is_noop() {
        let mut h = CodeHistory::new(pid("GP001"));
        assert_eq!(h.set_current(Some(pid("GP001"))), None);
        assert!(h.old_codes().is_empty());
    }

    #[test]
    fn reassigning_a_historical_code_reclaims_it() {
        let mut h = CodeHistory::new(pid("GP001"));
        h.set_current(Some(pid("GP002"))); // history: [GP001]
        h.set_current(Some(pid("GP001"))); // reclaim GP001, history: [GP002]
        assert_eq!(h.full_id(), "GP001");
        assert_eq!(h.old_codes(), &["GP002".to_string()]);
    }

    #[test]
    fn add_old_codes_ignores_current_and_dups() {
        let mut h = CodeHistory::new(pid("GP001"));
        h.add_old_codes(vec!["GP002".into()]);
        h.add_old_codes(vec![
            "GP001".into(), // equals current → ignored
            "GP002".into(), // already in history → ignored
            "GP003".into(),
        ]);
        assert_eq!(h.old_codes(), &["GP002".to_string(), "GP003".to_string()]);
        assert_eq!(h.full_id(), "GP001");
    }

    #[test]
    fn remove_old_codes_drops_listed_only() {
        let mut h = CodeHistory::new(pid("GP001"));
        h.set_old_codes(vec!["GP002".into(), "GP003".into(), "GP004".into()]);
        h.remove_old_codes(&["GP003".to_string(), "GP999".to_string()]);
        assert_eq!(h.old_codes(), &["GP002".to_string(), "GP004".to_string()]);
    }

    #[test]
    fn set_old_codes_drops_current_and_dups() {
        let mut h = CodeHistory::new(pid("GP001"));
        h.set_old_codes(vec![
            "GP002".into(),
            "GP001".into(), // equals current → dropped
            "GP002".into(), // duplicate → dropped
            "GP003".into(),
        ]);
        assert_eq!(h.old_codes(), &["GP002".to_string(), "GP003".to_string()]);
    }
}
