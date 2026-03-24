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

/// Represents a panel session within a part
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelSession {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_num: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prereq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_panelist: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub room_ids: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    pub duration: u32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_full: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seats_sold: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_reg_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simple_tix_event: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hide_panelist: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub credited_presenters: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uncredited_presenters: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes_non_printing: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workshop_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_needs: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub sewing_machines: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
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

impl PanelSession {
    /// Returns true if this session is scheduled (has start time, room, and duration/end time)
    pub fn is_scheduled(&self) -> bool {
        let has_time = self.start_time.is_some();
        let has_room = !self.room_ids.is_empty();
        let has_duration_or_end = self.duration > 0 || self.end_time.is_some();
        has_time && has_room && has_duration_or_end
    }
}

/// Represents a part of a panel
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_num: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prereq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_panelist: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub credited_presenters: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uncredited_presenters: Vec<String>,
    pub sessions: Vec<PanelSession>,
    #[serde(skip)]
    pub change_state: ChangeState,
}

/// Represents a base panel
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prereq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_panelist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_reg_max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket_url: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_free: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_kids: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub credited_presenters: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uncredited_presenters: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simple_tix_event: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub have_ticket_image: Option<bool>,
    pub parts: Vec<PanelPart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ExtraFields>,
    #[serde(skip)]
    pub change_state: ChangeState,
}

impl Panel {
    /// Create a new empty panel with the given ID
    pub fn new(id: String) -> Self {
        Panel {
            id,
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
            is_free: false,
            is_kids: false,
            credited_presenters: Vec::new(),
            uncredited_presenters: Vec::new(),
            simple_tix_event: None,
            have_ticket_image: None,
            parts: Vec::new(),
            metadata: None,
            change_state: ChangeState::Unchanged,
        }
    }

    /// Find or create a part within this panel
    pub fn find_or_create_part(&mut self, part_num: Option<u32>) -> &mut PanelPart {
        if let Some(idx) = self.parts.iter().position(|p| p.part_num == part_num) {
            &mut self.parts[idx]
        } else {
            let part = PanelPart {
                part_num,
                description: None,
                note: None,
                prereq: None,
                alt_panelist: None,
                credited_presenters: Vec::new(),
                uncredited_presenters: Vec::new(),
                sessions: Vec::new(),
                change_state: ChangeState::Unchanged,
            };
            self.parts.push(part);
            self.parts.last_mut().unwrap()
        }
    }
}

impl PanelPart {
    /// Always create a new session (used during import to avoid overwriting)
    pub fn create_new_session(
        &mut self,
        session_num: Option<u32>,
        id: String,
    ) -> &mut PanelSession {
        let session = PanelSession {
            id,
            session_num,
            description: None,
            note: None,
            prereq: None,
            alt_panelist: None,
            room_ids: Vec::new(),
            start_time: None,
            end_time: None,
            duration: 60,
            is_full: false,
            capacity: None,
            seats_sold: None,
            pre_reg_max: None,
            ticket_url: None,
            simple_tix_event: None,
            hide_panelist: false,
            credited_presenters: Vec::new(),
            uncredited_presenters: Vec::new(),
            notes_non_printing: None,
            workshop_notes: None,
            power_needs: None,
            sewing_machines: false,
            av_notes: None,
            source: None,
            change_state: ChangeState::Unchanged,
            conflicts: Vec::new(),
            metadata: IndexMap::new(),
        };
        self.sessions.push(session);
        self.sessions.last_mut().unwrap()
    }
}

/// Implements the common-prefix algorithm for factoring shared text.
///
/// Returns `(new_suffix, narrowed_old_suffix)`:
/// - `new_suffix`: the unique portion of `new_value` after the shared prefix
/// - `narrowed_old_suffix`: if the prefix narrowed, the portion stripped from the
///   previous stored value — the caller must prepend this to all existing siblings
///
/// Neither the stored prefix nor the returned suffixes include the separator space;
/// that space is added back by the join step when reconstructing effective values.
pub fn apply_common_prefix(
    existing: &mut Option<String>,
    new_value: &str,
) -> (String, Option<String>) {
    match existing {
        None => {
            *existing = Some(new_value.to_string());
            (String::new(), None)
        }
        Some(existing_text) => {
            if existing_text.is_empty() {
                return (new_value.to_string(), None);
            }
            if new_value.is_empty() {
                return (String::new(), None);
            }

            let old_value = existing_text.clone();
            match find_common_prefix_boundary(&old_value, new_value) {
                None => {
                    // No word-boundary common prefix — clear stored prefix entirely
                    *existing = Some(String::new());
                    (new_value.to_string(), Some(old_value))
                }
                Some((prefix_end, suffix_start)) if prefix_end < old_value.len() => {
                    // Prefix narrows: split both strings at the boundary
                    *existing = Some(old_value[..prefix_end].to_string());
                    let old_suffix = old_value[suffix_start..].to_string();
                    let new_suffix = new_value[suffix_start..].to_string();
                    (new_suffix, Some(old_suffix))
                }
                Some((_, suffix_start)) => {
                    // Existing is already the prefix (or exact match)
                    (new_value[suffix_start..].to_string(), None)
                }
            }
        }
    }
}

/// Find where to split two strings on a word boundary.
///
/// Returns `Some((prefix_end, suffix_start))` where:
/// - `prefix_end` is the end of the common part (exclusive), NOT including the space
/// - `suffix_start` is the start of the unique portion in each string, AFTER the space
///
/// The space between prefix and suffix is not stored in either side; it is added
/// back by the join step (`join_parts` uses `" ".join(...)`).
fn find_common_prefix_boundary(a: &str, b: &str) -> Option<(usize, usize)> {
    let mut common_end = 0;

    for (c1, c2) in a.chars().zip(b.chars()) {
        if c1 != c2 {
            break;
        }
        common_end += c1.len_utf8();
    }

    if common_end == 0 {
        return None;
    }

    // One string is a full prefix of the other
    if common_end == a.len() {
        if common_end == b.len() {
            // Exact match — no suffix
            return Some((common_end, common_end));
        }
        // `a` is a prefix of `b`; check if the next char in `b` is a space
        if b.as_bytes().get(common_end) == Some(&b' ') {
            return Some((common_end, common_end + 1));
        }
        // Not a word boundary — fall through to backtrack
    }

    // Backtrack to the last whitespace within the matched region
    let prefix_candidate = &a[..common_end];
    if let Some(ws_pos) = prefix_candidate.rfind(char::is_whitespace) {
        Some((ws_pos, ws_pos + 1))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_common_prefix() {
        // First entry: stores full value, no suffix
        let mut existing = None;
        let (new_suffix, narrowed) = apply_common_prefix(&mut existing, "ABC DEF GH");
        assert_eq!(existing.as_deref(), Some("ABC DEF GH"));
        assert_eq!(new_suffix, "");
        assert!(narrowed.is_none());

        // Second entry: prefix narrows to "ABC" (no trailing space stored)
        let (new_suffix, narrowed) = apply_common_prefix(&mut existing, "ABC LMO IJK");
        assert_eq!(existing.as_deref(), Some("ABC"));
        assert_eq!(new_suffix, "LMO IJK");
        assert_eq!(narrowed.as_deref(), Some("DEF GH"));

        // Third entry: "ABC" is an exact prefix of "ABC DEF GHI", no narrowing
        let (new_suffix, narrowed) = apply_common_prefix(&mut existing, "ABC DEF GHI");
        assert_eq!(existing.as_deref(), Some("ABC"));
        assert_eq!(new_suffix, "DEF GHI");
        assert!(narrowed.is_none());

        // Fourth entry: no word-boundary common prefix between "ABC" and "A XYZ"
        let (new_suffix, narrowed) = apply_common_prefix(&mut existing, "A XYZ");
        assert_eq!(existing.as_deref(), Some(""));
        assert_eq!(new_suffix, "A XYZ");
        assert_eq!(narrowed.as_deref(), Some("ABC"));
    }

    #[test]
    fn test_apply_common_prefix_sentence_boundary() {
        // Typical case: sentence-level split preserves full sentence in base
        let mut existing = None;
        apply_common_prefix(&mut existing, "Learn to sew. Bring your own fabric.");
        let (new_suffix, narrowed) = apply_common_prefix(
            &mut existing,
            "Learn to sew. Last day: projects not claimed will be tossed.",
        );
        assert_eq!(existing.as_deref(), Some("Learn to sew."));
        assert_eq!(new_suffix, "Last day: projects not claimed will be tossed.");
        assert_eq!(narrowed.as_deref(), Some("Bring your own fabric."));
    }

    #[test]
    fn test_find_common_prefix_boundary() {
        // Exact match — no suffix on either side
        assert_eq!(
            find_common_prefix_boundary("ABC DEF", "ABC DEF"),
            Some((7, 7))
        );

        // Diverge mid-word, backtrack to space — prefix excludes space
        assert_eq!(
            find_common_prefix_boundary("ABC DEF GH", "ABC DEF IJK"),
            Some((7, 8))
        );

        // Diverge right after the space — backtrack to that space
        assert_eq!(
            find_common_prefix_boundary("ABC DEF", "ABC XYZ"),
            Some((3, 4))
        );

        // No common word-boundary prefix
        assert_eq!(find_common_prefix_boundary("ABC", "XYZ"), None);

        // One string is a prefix of the other, followed by a space
        assert_eq!(find_common_prefix_boundary("ABC", "ABC DEF"), Some((3, 4)));

        // One string is a prefix, next char is NOT a space — backtrack
        assert_eq!(find_common_prefix_boundary("ABC", "ABCDEF"), None);
    }
}
