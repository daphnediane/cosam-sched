/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::source_info::{ChangeState, SourceInfo};

/// Represents extra fields from non-standard spreadsheet columns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExtraValue {
    String(String),
    Formula(FormulaValue),
}

/// Represents a formula with its evaluated value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormulaValue {
    pub formula: String,
    pub value: String,
}

/// Additional non-standard spreadsheet columns
pub type ExtraFields = IndexMap<String, ExtraValue>;

/// A fully self-contained panel entry in the flat model.
///
/// Each panel belongs to a [`super::panel_set::PanelSet`] identified by
/// `base_id`.  A panel may carry optional `part_num` / `session_num` to
/// reflect XLSX part/session numbering, but those are informational only —
/// the combination (`base_id`, `part_num`, `session_num`) forms a logical
/// key while `id` is the canonical unique identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    /// Full unique identifier (e.g. `"GP002P1S2"`).
    pub id: String,
    /// Base ID of the containing [`super::panel_set::PanelSet`] (e.g. `"GP002"`).
    pub base_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_num: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_num: Option<u32>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prereq: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alt_panelist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_reg_max: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simple_tix_event: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub have_ticket_image: Option<bool>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_free: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_kids: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_full: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hide_panelist: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sewing_machines: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub room_ids: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(default)]
    pub duration: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats_sold: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credited_presenters: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uncredited_presenters: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes_non_printing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workshop_notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_needs: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub av_notes: Option<String>,
    #[serde(skip)]
    pub source: Option<SourceInfo>,
    #[serde(skip)]
    pub change_state: ChangeState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<super::event::EventConflict>,
    #[serde(default, alias = "extras", skip_serializing_if = "IndexMap::is_empty")]
    pub metadata: ExtraFields,
}

impl Panel {
    /// Create a new empty panel.
    pub fn new(id: impl Into<String>, base_id: impl Into<String>) -> Self {
        Panel {
            id: id.into(),
            base_id: base_id.into(),
            part_num: None,
            session_num: None,
            name: String::new(),
            panel_type: None,
            description: None,
            note: None,
            prereq: None,
            alt_panelist: None,
            cost: None,
            capacity: None,
            pre_reg_max: None,
            difficulty: None,
            ticket_url: None,
            simple_tix_event: None,
            have_ticket_image: None,
            is_free: false,
            is_kids: false,
            is_full: false,
            hide_panelist: false,
            sewing_machines: false,
            room_ids: Vec::new(),
            start_time: None,
            end_time: None,
            duration: 60,
            seats_sold: None,
            credited_presenters: Vec::new(),
            uncredited_presenters: Vec::new(),
            notes_non_printing: None,
            workshop_notes: None,
            power_needs: None,
            av_notes: None,
            source: None,
            change_state: ChangeState::Unchanged,
            conflicts: Vec::new(),
            metadata: IndexMap::new(),
        }
    }

    /// Returns `true` if this panel has scheduling information (time + room + duration/end).
    pub fn is_scheduled(&self) -> bool {
        let has_time = self.start_time.is_some();
        let has_room = !self.room_ids.is_empty();
        let has_duration_or_end = self.duration > 0 || self.end_time.is_some();
        has_time && has_room && has_duration_or_end
    }
}
