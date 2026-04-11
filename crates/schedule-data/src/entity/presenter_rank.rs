/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Presenter rank enum for schedule-data

use serde::{Deserialize, Serialize, Serializer};

/// Presenter rank classification matching schedule-core
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PresenterRank {
    Guest,
    Judge,
    Staff,
    /// Invited / industry tier with an optional custom display label.
    /// `None` serializes as `"invited_panelist"`; `Some(label)` serializes as
    /// the label string directly (e.g. `"Sponsor"`, `"105th"`).
    InvitedGuest(Option<String>),
    #[default]
    Panelist,
    FanPanelist,
}

impl PresenterRank {
    /// Map a single tag character from the presenter credit string format to a
    /// `PresenterRank`.  Returns `None` for unknown characters.
    ///
    /// | Char | Rank |
    /// |------|------|
    /// | `G` / `g` | `Guest` |
    /// | `J` / `j` | `Judge` |
    /// | `S` / `s` | `Staff` |
    /// | `I` / `i` | `InvitedGuest(None)` |
    /// | `F` / `f` | `FanPanelist` |
    /// | `P` / `p` | `Panelist` |
    pub fn from_prefix_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'G' => Some(PresenterRank::Guest),
            'J' => Some(PresenterRank::Judge),
            'S' => Some(PresenterRank::Staff),
            'I' => Some(PresenterRank::InvitedGuest(None)),
            'P' => Some(PresenterRank::Panelist),
            'F' => Some(PresenterRank::FanPanelist),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            PresenterRank::Guest => "guest",
            PresenterRank::Judge => "judge",
            PresenterRank::Staff => "staff",
            PresenterRank::InvitedGuest(None) => "invited_panelist",
            PresenterRank::InvitedGuest(Some(s)) => s.as_str(),
            PresenterRank::Panelist => "panelist",
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
            PresenterRank::Panelist => 4,
            PresenterRank::FanPanelist => 5,
        }
    }

    pub fn parse_rank(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "guest" => PresenterRank::Guest,
            "judge" => PresenterRank::Judge,
            "staff" => PresenterRank::Staff,
            "invited_panelist" | "invitedpanelist" => PresenterRank::InvitedGuest(None),
            "panelist" => PresenterRank::Panelist,
            "fan_panelist" | "fanpanelist" => PresenterRank::FanPanelist,
            _ => PresenterRank::InvitedGuest(Some(s.to_string())),
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
            PresenterRank::Panelist,
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
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(PresenterRank::parse_rank(&s))
    }
}

impl std::fmt::Display for PresenterRank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
