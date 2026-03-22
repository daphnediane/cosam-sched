/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Represents a parsed panel ID with prefix, number, part, session, and suffix
///
/// # Examples
///
/// - `"GP002"` → prefix `"GP"`, prefix_num 2
/// - `"GW097P1"` → prefix `"GW"`, prefix_num 97, part_num 1
/// - `"GW093P1AS4B"` → prefix `"GW"`, prefix_num 93, part_num 1, session_num 4, suffix `"AB"`
/// - `"SPLIT01"` → prefix `"SP"`, prefix_num 1 (normalized from SPLIT)
/// - `"BREAK09"` → prefix `"BR"`, prefix_num 9 (normalized from BREAK)
///
/// # Unsupported Formats for future
///
/// - `"SPLIT"` (no number) - prefix `"SP"`, prefix_num None (will be assigned after table processed)
/// - `"BREAK"` (no number) - prefix `"BR"`, prefix_num None (will be assigned after table processed) 
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PanelId {
    /// Normalized 2-letter uppercase prefix (e.g. "GP", "SP", "BR")
    pub prefix: String,
    /// Numeric portion after the prefix
    pub prefix_num: u32,
    /// Part number from P<n> component, if present
    pub part_num: Option<u32>,
    /// Session number from S<n> component, if present
    pub session_num: Option<u32>,
    /// Collected trailing alpha characters (e.g. "AB" from "…P1AS4B")
    pub suffix: Option<String>,
}

impl PanelId {
    /// Parse a Uniq ID string into its components
    ///
    /// The raw alpha prefix is normalized to 2 uppercase characters
    /// (e.g. SPLIT→SP, BREAK→BR, EVENT→EV). Prefixes that are already
    /// ≤2 characters are kept as-is.
    ///
    /// Suffix characters that appear after part/session numbers are
    /// collected into a single `suffix` field.
    pub fn parse(id: &str) -> Option<Self> {
        let id_upper = id.to_uppercase();

        // Phase 1: match alpha prefix + digits, capture the rest as tail
        let head_re = Regex::new(r"^([A-Z]+)(\d+)([A-Z0-9]*)$").ok()?;
        let caps = head_re.captures(&id_upper)?;

        let raw_prefix = caps.get(1)?.as_str();
        let prefix = if raw_prefix.len() > 2 {
            raw_prefix[..2].to_string()
        } else {
            raw_prefix.to_string()
        };
        let prefix_num: u32 = caps.get(2)?.as_str().parse().ok()?;

        // Phase 2: iteratively extract P\d+ and S\d+ from the tail,
        // collecting remaining alpha characters as suffix.
        let mut tail = caps.get(3).map(|m| m.as_str()).unwrap_or("").to_string();
        let mut part_num: Option<u32> = None;
        let mut session_num: Option<u32> = None;
        let ps_re = Regex::new(r"[PS]\d+").ok()?;
        while let Some(m) = ps_re.find(&tail) {
            let matched = m.as_str();
            let (tag, num_str) = matched.split_at(1);
            let num: u32 = num_str.parse().ok()?;
            match tag {
                "P" => part_num = Some(num),
                "S" => session_num = Some(num),
                _ => {}
            }
            tail = format!("{}{}", &tail[..m.start()], &tail[m.end()..]);
        }

        let suffix = if tail.is_empty() {
            None
        } else if tail.chars().all(|c| c.is_ascii_alphabetic()) {
            Some(tail)
        } else {
            return None; // leftover digits without P/S tag → invalid
        };

        Some(PanelId {
            prefix,
            prefix_num,
            part_num,
            session_num,
            suffix,
        })
    }

    /// Computed base ID: normalized prefix + zero-padded 3-digit number
    ///
    /// e.g. prefix "GW" + prefix_num 97 → "GW097"
    pub fn base_id(&self) -> String {
        format!("{}{:03}", self.prefix, self.prefix_num)
    }

    /// Get the full ID string including part, session, and suffix
    ///
    /// @TODO: Is this needed? full ID is the given ID not the canonical form
    /// This should probably be called canonical, but it losses the order of
    /// suffixes which might have a meaning again in the future depending
    /// on how the programmer coordinator uses the system.
    pub fn full_id(&self) -> String {
        let mut id = self.base_id();
        if let Some(part) = self.part_num {
            id.push_str(&format!("P{}", part));
        }
        if let Some(session) = self.session_num {
            id.push_str(&format!("S{}", session));
        }
        if let Some(ref s) = self.suffix {
            // Suffix goes after session when both present
            id.push_str(s);
        }
        id
    }

    /// Get the ID for the part level (includes part if present, not session)
    pub fn part_id(&self) -> String {
        let mut id = self.base_id();
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
        assert_eq!(pid.prefix, "GP");
        assert_eq!(pid.prefix_num, 2);
        assert_eq!(pid.base_id(), "GP002");
        assert_eq!(pid.part_num, None);
        assert_eq!(pid.session_num, None);
        assert_eq!(pid.suffix, None);

        // IDs with parts
        let pid = PanelId::parse("GW097P1").unwrap();
        assert_eq!(pid.prefix, "GW");
        assert_eq!(pid.prefix_num, 97);
        assert_eq!(pid.base_id(), "GW097");
        assert_eq!(pid.part_num, Some(1));
        assert_eq!(pid.session_num, None);
        assert_eq!(pid.suffix, None);

        // IDs with parts and sessions
        let pid = PanelId::parse("GW097P2S3").unwrap();
        assert_eq!(pid.base_id(), "GW097");
        assert_eq!(pid.part_num, Some(2));
        assert_eq!(pid.session_num, Some(3));
        assert_eq!(pid.suffix, None);

        // IDs with suffix after session
        let pid = PanelId::parse("GW097P2S3A").unwrap();
        assert_eq!(pid.base_id(), "GW097");
        assert_eq!(pid.part_num, Some(2));
        assert_eq!(pid.session_num, Some(3));
        assert_eq!(pid.suffix, Some("A".to_string()));

        // IDs with suffix after both part and session (user example)
        let pid = PanelId::parse("GW093P1AS4B").unwrap();
        assert_eq!(pid.prefix, "GW");
        assert_eq!(pid.prefix_num, 93);
        assert_eq!(pid.part_num, Some(1));
        assert_eq!(pid.session_num, Some(4));
        assert_eq!(pid.suffix, Some("AB".to_string()));

        // IDs with suffix after both session and part (user example)
        // ensure that session can come before part
        let pid = PanelId::parse("GW093S4AP1B").unwrap();
        assert_eq!(pid.prefix, "GW");
        assert_eq!(pid.prefix_num, 93);
        assert_eq!(pid.part_num, Some(1));
        assert_eq!(pid.session_num, Some(4));
        assert_eq!(pid.suffix, Some("AB".to_string()));

        // Long prefix normalization (user example)
        let pid = PanelId::parse("SPLIT01").unwrap();
        assert_eq!(pid.prefix, "SP");
        assert_eq!(pid.prefix_num, 1);
        assert_eq!(pid.base_id(), "SP001");
        assert_eq!(pid.part_num, None);
        assert_eq!(pid.session_num, None);
        assert_eq!(pid.suffix, None);

        let pid = PanelId::parse("BREAK09").unwrap();
        assert_eq!(pid.prefix, "BR");
        assert_eq!(pid.prefix_num, 9);
        assert_eq!(pid.base_id(), "BR009");

        let pid = PanelId::parse("EVENT01").unwrap();
        assert_eq!(pid.prefix, "EV");
        assert_eq!(pid.prefix_num, 1);
        assert_eq!(pid.base_id(), "EV001");

        // Case insensitivity
        let id = PanelId::parse("gp002").unwrap();
        assert_eq!(id.prefix, "GP");
        assert_eq!(id.base_id(), "GP002");

        // Invalid formats
        assert!(PanelId::parse("").is_none());
        assert!(PanelId::parse("INVALID").is_none());
        assert!(PanelId::parse("123").is_none());

        // Multi-letter suffix after session
        let pid = PanelId::parse("GW097P2S3XYZ").unwrap();
        assert_eq!(pid.base_id(), "GW097");
        assert_eq!(pid.part_num, Some(2));
        assert_eq!(pid.session_num, Some(3));
        assert_eq!(pid.suffix, Some("XYZ".to_string()));
    }

    #[test]
    fn test_prefix_normalization() {
        // BREAK09 should normalize to BR prefix
        let pid = PanelId::parse("BREAK09").unwrap();
        assert_eq!(pid.prefix, "BR");
        assert_eq!(pid.prefix_num, 9);
        assert_eq!(pid.base_id(), "BR009");

        // EVENT01 should normalize to EV prefix
        let pid = PanelId::parse("EVENT01").unwrap();
        assert_eq!(pid.prefix, "EV");
        assert_eq!(pid.base_id(), "EV001");

        // PANEL12 should normalize to PA prefix
        let pid = PanelId::parse("PANEL12").unwrap();
        assert_eq!(pid.prefix, "PA");
        assert_eq!(pid.base_id(), "PA012");

        // Already normalized IDs should stay the same
        let pid = PanelId::parse("BR009").unwrap();
        assert_eq!(pid.prefix, "BR");
        assert_eq!(pid.base_id(), "BR009");

        // IDs with parts should normalize prefix too
        let pid = PanelId::parse("BREAK09P1").unwrap();
        assert_eq!(pid.prefix, "BR");
        assert_eq!(pid.base_id(), "BR009");
        assert_eq!(pid.part_num, Some(1));
    }
}
