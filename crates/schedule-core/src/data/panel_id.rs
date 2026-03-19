/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Represents a parsed panel ID with base, part, and session components
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PanelId {
    pub base_id: String,
    pub part_num: Option<u32>,
    pub session_num: Option<u32>,
}

impl PanelId {
    /// Parse a Uniq ID string into its components
    ///
    /// Examples:
    /// - "GP002" -> base_id="GP002", part_num=None, session_num=None
    /// - "GW097P1" -> base_id="GW097", part_num=Some(1), session_num=None
    /// - "GW097P2S3" -> base_id="GW097", part_num=Some(2), session_num=Some(3)
    /// - "GW097P2S3A" -> base_id="GW097", part_num=Some(2), session_num=Some(3), suffix="A"
    /// - "BREAK09" -> base_id="BR009", part_num=None, session_num=None (normalized from BREAK09)
    pub fn parse(id: &str) -> Option<Self> {
        // First normalize numeric prefixes like BREAK09 -> BR009
        let normalized_id = Self::normalize_numeric_prefix(id);

        let re = Regex::new(r"^([A-Za-z]+\d+)(?:P(\d+))?(?:S(\d+))?([A-Za-z])?$").ok()?;
        let caps = re.captures(&normalized_id)?;

        let base_id = caps.get(1)?.as_str().to_uppercase();
        let part_num = caps.get(2).and_then(|m| m.as_str().parse().ok());
        let session_num = caps.get(3).and_then(|m| m.as_str().parse().ok());
        // Note: suffix is captured but not stored in the current struct
        // Could be added later if needed: let suffix = caps.get(4).map(|m| m.as_str());

        Some(PanelId {
            base_id,
            part_num,
            session_num,
        })
    }

    /// Normalize numeric prefixes in IDs like BREAK09 -> BR009
    fn normalize_numeric_prefix(id: &str) -> String {
        let id_upper = id.to_uppercase();
        let re = Regex::new(r"^([A-Za-z]+)(\d+)((?:P\d+)?(?:S\d+)?[A-Za-z]?)$").ok();
        if let Some(caps) = re.and_then(|r| r.captures(&id_upper)) {
            let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let number = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let rest = caps.get(3).map(|m| m.as_str()).unwrap_or("");

            if let Ok(num) = number.parse::<u32>() {
                let normalized_prefix = if prefix.len() > 2 {
                    &prefix[..2]
                } else {
                    prefix
                };
                return format!("{}{:03}{}", normalized_prefix, num, rest);
            }
        }
        id_upper
    }

    /// Get the full ID string
    pub fn full_id(&self) -> String {
        let mut id = self.base_id.clone();
        if let Some(part) = self.part_num {
            id.push_str(&format!("P{}", part));
        }
        if let Some(session) = self.session_num {
            id.push_str(&format!("S{}", session));
        }
        id
    }

    /// Get the ID for the part level (includes part if present, not session)
    pub fn part_id(&self) -> String {
        let mut id = self.base_id.clone();
        if let Some(part) = self.part_num {
            id.push_str(&format!("P{}", part));
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_panel_id() {
        // Basic IDs
        let pid = PanelId::parse("GP002").unwrap();
        assert_eq!(pid.base_id, "GP002");
        assert_eq!(pid.part_num, None);
        assert_eq!(pid.session_num, None);

        // IDs with parts
        let pid = PanelId::parse("GW097P1").unwrap();
        assert_eq!(pid.base_id, "GW097");
        assert_eq!(pid.part_num, Some(1));
        assert_eq!(pid.session_num, None);

        // IDs with parts and sessions
        let pid = PanelId::parse("GW097P2S3").unwrap();
        assert_eq!(pid.base_id, "GW097");
        assert_eq!(pid.part_num, Some(2));
        assert_eq!(pid.session_num, Some(3));

        // IDs with trailing letters (valid in older sheets)
        let pid = PanelId::parse("GW097P2S3A").unwrap();
        assert_eq!(pid.base_id, "GW097");
        assert_eq!(pid.part_num, Some(2));
        assert_eq!(pid.session_num, Some(3));

        // Numeric prefix normalization
        let pid = PanelId::parse("BREAK09").unwrap();
        assert_eq!(pid.base_id, "BR009");
        assert_eq!(pid.part_num, None);
        assert_eq!(pid.session_num, None);

        let pid = PanelId::parse("EVENT01").unwrap();
        assert_eq!(pid.base_id, "EV001");
        assert_eq!(pid.part_num, None);
        assert_eq!(pid.session_num, None);

        // Case insensitivity
        let id = PanelId::parse("gp002").unwrap();
        assert_eq!(id.base_id, "GP002");

        // Invalid formats
        assert!(PanelId::parse("").is_none());
        assert!(PanelId::parse("INVALID").is_none());
        assert!(PanelId::parse("123").is_none());
        // @todo GW097P2S3XYZ is valid as is GW097XYZP2S3
        assert!(PanelId::parse("GW097P2S3XYZ").is_none()); // Too many trailing letters
    }

    #[test]
    fn test_normalize_numeric_prefix() {
        // Test with a direct reference to the private method through parse
        // BREAK09 should normalize to BR009
        let pid = PanelId::parse("BREAK09").unwrap();
        assert_eq!(pid.base_id, "BR009");

        // EVENT01 should normalize to EV001
        let pid = PanelId::parse("EVENT01").unwrap();
        assert_eq!(pid.base_id, "EV001");

        // PANEL12 should normalize to PA012
        let pid = PanelId::parse("PANEL12").unwrap();
        assert_eq!(pid.base_id, "PA012");

        // Already normalized IDs should stay the same
        let pid = PanelId::parse("BR009").unwrap();
        assert_eq!(pid.base_id, "BR009");

        // IDs with parts should not normalize the base
        let pid = PanelId::parse("BREAK09P1").unwrap();
        assert_eq!(pid.base_id, "BR009");
        assert_eq!(pid.part_num, Some(1));
    }
}
