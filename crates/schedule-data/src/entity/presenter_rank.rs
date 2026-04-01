/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Presenter rank enum for schedule-data

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Presenter rank classification matching schedule-core
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
            "invited_panelist" | "invitedpanelist" => PresenterRank::InvitedGuest(None),
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
        Ok(PresenterRank::from_str(&s))
    }
}

impl Default for PresenterRank {
    fn default() -> Self {
        PresenterRank::FanPanelist
    }
}

impl std::fmt::Display for PresenterRank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
