/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::panel::ExtraFields;
use super::source_info::{ChangeState, SourceInfo};

#[derive(Debug, Clone, PartialEq)]
pub enum PresenterRank {
    Guest,
    Judge,
    Staff,
    /// Invited / industry tier with an optional custom display label.
    /// `None` serializes as `"invited_panelist"`; `Some(label)` serializes as
    /// the label string directly (e.g. `"Sponsor"`, `"105th"`).
    InvitedGuest(Option<String>),
    FanPanelist,
}

impl PresenterRank {
    pub fn as_str(&self) -> &str {
        match self {
            PresenterRank::Guest => "guest",
            PresenterRank::Judge => "judge",
            PresenterRank::Staff => "staff",
            PresenterRank::InvitedGuest(None) => "invited_panelist",
            PresenterRank::InvitedGuest(Some(s)) => s.as_str(),
            PresenterRank::FanPanelist => "fan_panelist",
        }
    }

    /// Numeric priority: lower value = higher rank tier.
    /// Used to resolve conflicts between schedule-prefix rank and People-sheet
    /// classification — the rank with the lower priority number wins.
    pub fn priority(&self) -> u8 {
        match self {
            PresenterRank::Guest => 0,
            PresenterRank::Judge => 1,
            PresenterRank::Staff => 2,
            PresenterRank::InvitedGuest(_) => 3,
            PresenterRank::FanPanelist => 4,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "guest" => PresenterRank::Guest,
            "judge" => PresenterRank::Judge,
            "staff" => PresenterRank::Staff,
            "invited_guest" | "invited_panelist" => PresenterRank::InvitedGuest(None),
            "fan_panelist" => PresenterRank::FanPanelist,
            _ => PresenterRank::InvitedGuest(Some(s.to_string())),
        }
    }

    pub fn prefix_char(&self) -> char {
        match self {
            PresenterRank::Guest => 'G',
            PresenterRank::Judge => 'J',
            PresenterRank::Staff => 'S',
            PresenterRank::InvitedGuest(_) => 'I',
            PresenterRank::FanPanelist => 'P',
        }
    }

    /// Map the single-character column prefix used in XLSX presenter column
    /// headers (`G`, `J`, `S`, `I`, `P`) to the corresponding `PresenterRank`.
    /// Case-insensitive. Returns `None` for unknown characters.
    pub fn from_prefix_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'G' => Some(PresenterRank::Guest),
            'J' => Some(PresenterRank::Judge),
            'S' => Some(PresenterRank::Staff),
            'I' => Some(PresenterRank::InvitedGuest(None)),
            'P' => Some(PresenterRank::FanPanelist),
            _ => None,
        }
    }

    /// Parse a classification string from a spreadsheet cell.
    /// Accepts display names ("Fan Panelist", "Invited Guest"), internal names
    /// ("fan_panelist", "invited_guest"), and single-character prefix codes
    /// ("G", "P", etc.).  Falls back to `InvitedGuest(Some(label))` for anything
    /// unrecognized so the display string is preserved.
    pub fn from_classification(s: &str) -> Self {
        let lower = s.trim().to_lowercase();
        match lower.as_str() {
            "guest" | "g" => PresenterRank::Guest,
            "judge" | "j" => PresenterRank::Judge,
            "staff" | "s" => PresenterRank::Staff,
            "invited" | "invited guest" | "invited_guest" | "invited panelist"
            | "invited_panelist" | "i" => PresenterRank::InvitedGuest(None),
            "fan" | "fan panelist" | "fan_panelist" | "p" => PresenterRank::FanPanelist,
            _ => PresenterRank::InvitedGuest(Some(s.trim().to_string())),
        }
    }

    /// All standard ranks in priority order used for column layout.
    /// `InvitedGuest(None)` is the representative for the entire invited tier.
    pub fn standard_ranks() -> &'static [PresenterRank] {
        &[
            PresenterRank::Guest,
            PresenterRank::Judge,
            PresenterRank::Staff,
            PresenterRank::InvitedGuest(None),
            PresenterRank::FanPanelist,
        ]
    }
}

impl Serialize for PresenterRank {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PresenterRank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(PresenterRank::from_str(&s))
    }
}

impl Default for PresenterRank {
    fn default() -> Self {
        PresenterRank::FanPanelist
    }
}

/// Ordering key for a presenter, recording where it was first defined.
///
/// - `column_index`: 0 for the People table, schedule column number otherwise.
/// - `row_index`: row in the People table, or position in a comma-separated
///   presenter list on the schedule sheet.
/// - `member_index`: position within a group's member list (0 for the group
///   itself or for standalone presenters; 1+ for individual members).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PresenterSortRank {
    pub column_index: u32,
    pub row_index: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub member_index: u32,
}

fn is_zero(v: &u32) -> bool {
    *v == 0
}

impl PresenterSortRank {
    pub fn new(column_index: u32, row_index: u32, member_index: u32) -> Self {
        Self {
            column_index,
            row_index,
            member_index,
        }
    }

    /// People table rank: column 0, given row, member_index 0.
    pub fn people(row_index: u32) -> Self {
        Self::new(0, row_index, 0)
    }

    /// Schedule column rank for a group entry.
    pub fn schedule_group(column_index: u32, row_index: u32) -> Self {
        Self::new(column_index, row_index, 0)
    }

    /// Schedule column rank for an individual member entry.
    pub fn schedule_member(column_index: u32, row_index: u32) -> Self {
        Self::new(column_index, row_index, 1)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Presenter {
    pub id: Option<u32>,
    pub name: String,
    pub rank: PresenterRank,
    /// Ordering key recording where this presenter was first defined.
    /// `None` if no source information is available.
    pub sort_rank: Option<PresenterSortRank>,
    pub metadata: Option<ExtraFields>,
    pub source: Option<SourceInfo>,
    pub change_state: ChangeState,
}

impl Presenter {
    /// Sort key for ordering presenters (e.g. in credits).
    /// Compares by classification rank, then sort_rank fields, then name.
    pub fn sort_key(&self) -> (u8, u32, u32, u32, &str) {
        match &self.sort_rank {
            Some(sr) => (
                self.rank.priority(),
                sr.column_index,
                sr.row_index,
                sr.member_index,
                &self.name,
            ),
            None => (
                self.rank.priority(),
                u32::MAX,
                u32::MAX,
                u32::MAX,
                &self.name,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presenter_deserialize_minimal() {
        let json = r#"{"name": "Yaya Han", "rank": "guest"}"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "Yaya Han");
        assert_eq!(p.rank, PresenterRank::Guest);
        assert_eq!(p.id, None);
    }

    #[test]
    fn test_presenter_deserialize_full() {
        let json = r#"{
            "name": "Pros and Cons Cosplay",
            "rank": "guest",
            "is_group": true,
            "members": ["Pro", "Con"],
            "groups": [],
            "always_grouped": false
        }"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "Pros and Cons Cosplay");
        assert_eq!(p.rank, PresenterRank::Guest);
    }

    #[test]
    fn test_presenter_custom_rank() {
        let json = r#"{"name": "CUT/SEW", "rank": "Sponsor"}"#;
        let p: Presenter = serde_json::from_str(json).unwrap();
        assert_eq!(p.name, "CUT/SEW");
        assert_eq!(p.rank.as_str(), "Sponsor");
        assert_eq!(p.rank.prefix_char(), 'I');
        assert_eq!(
            p.rank,
            PresenterRank::InvitedGuest(Some("Sponsor".to_string()))
        );
    }

    #[test]
    fn test_presenter_rank_from_str() {
        assert_eq!(PresenterRank::from_str("guest"), PresenterRank::Guest);
        assert_eq!(
            PresenterRank::from_str("invited_guest"),
            PresenterRank::InvitedGuest(None)
        );
        assert_eq!(
            PresenterRank::from_str("invited_panelist"),
            PresenterRank::InvitedGuest(None)
        );
        assert_eq!(
            PresenterRank::from_str("SPONSOR"),
            PresenterRank::InvitedGuest(Some("SPONSOR".to_string()))
        );
        assert_eq!(
            PresenterRank::from_str("industry"),
            PresenterRank::InvitedGuest(Some("industry".to_string()))
        );
    }

    #[test]
    fn test_presenter_rank_from_classification() {
        assert_eq!(
            PresenterRank::from_classification("Guest"),
            PresenterRank::Guest
        );
        assert_eq!(
            PresenterRank::from_classification("guest"),
            PresenterRank::Guest
        );
        assert_eq!(
            PresenterRank::from_classification("G"),
            PresenterRank::Guest
        );
        assert_eq!(
            PresenterRank::from_classification("Fan Panelist"),
            PresenterRank::FanPanelist
        );
        assert_eq!(
            PresenterRank::from_classification("fan_panelist"),
            PresenterRank::FanPanelist
        );
        assert_eq!(
            PresenterRank::from_classification("P"),
            PresenterRank::FanPanelist
        );
        assert_eq!(
            PresenterRank::from_classification("Invited Guest"),
            PresenterRank::InvitedGuest(None)
        );
        assert_eq!(
            PresenterRank::from_classification("invited_guest"),
            PresenterRank::InvitedGuest(None)
        );
        assert_eq!(
            PresenterRank::from_classification("Invited Panelist"),
            PresenterRank::InvitedGuest(None)
        );
        assert_eq!(
            PresenterRank::from_classification("I"),
            PresenterRank::InvitedGuest(None)
        );
        assert_eq!(
            PresenterRank::from_classification("Staff"),
            PresenterRank::Staff
        );
        assert_eq!(
            PresenterRank::from_classification("S"),
            PresenterRank::Staff
        );
        assert_eq!(
            PresenterRank::from_classification("Judge"),
            PresenterRank::Judge
        );
        assert_eq!(
            PresenterRank::from_classification("J"),
            PresenterRank::Judge
        );
        assert_eq!(
            PresenterRank::from_classification("Sponsor"),
            PresenterRank::InvitedGuest(Some("Sponsor".to_string()))
        );
        assert_eq!(
            PresenterRank::from_classification("105th"),
            PresenterRank::InvitedGuest(Some("105th".to_string()))
        );
    }

    #[test]
    fn test_presenter_roundtrip() {
        let p = Presenter {
            id: Some(5),
            name: "Sayakat Cosplay".into(),
            rank: PresenterRank::from_str("fan_panelist"),
            sort_rank: Some(PresenterSortRank::people(3)),
            metadata: None,
            source: None,
            change_state: ChangeState::Unchanged,
        };
        let json = serde_json::to_string(&p).unwrap();
        let p2: Presenter = serde_json::from_str(&json).unwrap();
        assert_eq!(p, p2);
    }
}
