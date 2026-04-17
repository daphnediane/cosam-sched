/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`PanelUniqId`] — parsed representation of a spreadsheet "Uniq ID" value.
//!
//! The raw string (e.g. `"GW093P1AS4B"`) is parsed into its structural
//! components: a two-letter prefix, a numeric base, optional part/session
//! numbers, and optional trailing-alpha suffix characters.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Parsed components of a panel's spreadsheet "Uniq ID".
///
/// # Format
///
/// ```text
/// <PREFIX><NUM>[P<part>][S<session>][<suffix>]
/// ```
///
/// - `PREFIX` — two or more uppercase letters, **normalized to 2 characters**
///   (e.g. `SPLIT` → `SP`, `BREAK` → `BR`).
/// - `NUM` — one or more digits (the base number for the panel series).
/// - `P<n>` — optional part number.
/// - `S<n>` — optional session number (repeat / re-run).
/// - `<suffix>` — optional trailing alpha characters after any P/S tags
///   (e.g. the `"AB"` from `"GW093P1AS4B"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PanelUniqId {
    /// Normalized 2-letter uppercase prefix (e.g. `"GP"`, `"SP"`, `"BR"`).
    pub prefix: String,
    /// Numeric portion of the base ID.
    pub prefix_num: u32,
    /// Part number from a `P<n>` component, if present.
    pub part_num: Option<u32>,
    /// Session number from an `S<n>` component, if present.
    pub session_num: Option<u32>,
    /// Collected trailing alpha characters (e.g. `"AB"` from `"…P1AS4B"`).
    pub suffix: Option<String>,
}

impl PanelUniqId {
    /// Parse a raw Uniq ID string into its components.
    ///
    /// Returns `None` if the string does not match the expected format.
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        let id_upper = id.to_uppercase();

        let head_re = Regex::new(r"^([A-Z]+)(\d+)([A-Z0-9]*)$").ok()?;
        let caps = head_re.captures(&id_upper)?;

        let raw_prefix = caps.get(1)?.as_str();
        let prefix = if raw_prefix.len() > 2 {
            raw_prefix[..2].to_string()
        } else {
            raw_prefix.to_string()
        };
        let prefix_num: u32 = caps.get(2)?.as_str().parse().ok()?;

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
            return None;
        };

        Some(PanelUniqId {
            prefix,
            prefix_num,
            part_num,
            session_num,
            suffix,
        })
    }

    /// Canonical base ID: normalized prefix + zero-padded 3-digit number.
    ///
    /// e.g. `"GW"` + `97` → `"GW097"`.
    #[must_use]
    pub fn base_id(&self) -> String {
        format!("{}{:03}", self.prefix, self.prefix_num)
    }

    /// Full canonical ID including optional part, session, and suffix.
    #[must_use]
    pub fn full_id(&self) -> String {
        let mut id = self.base_id();
        if let Some(p) = self.part_num {
            id.push_str(&format!("P{p}"));
        }
        if let Some(s) = self.session_num {
            id.push_str(&format!("S{s}"));
        }
        if let Some(ref s) = self.suffix {
            id.push_str(s);
        }
        id
    }

    /// ID at the part level (base + part number, no session or suffix).
    #[must_use]
    pub fn part_id(&self) -> String {
        let mut id = self.base_id();
        if let Some(p) = self.part_num {
            id.push_str(&format!("P{p}"));
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_id() {
        let pid = PanelUniqId::parse("GP002").unwrap();
        assert_eq!(pid.prefix, "GP");
        assert_eq!(pid.prefix_num, 2);
        assert_eq!(pid.base_id(), "GP002");
        assert!(pid.part_num.is_none());
        assert!(pid.session_num.is_none());
        assert!(pid.suffix.is_none());
    }

    #[test]
    fn parse_with_part() {
        let pid = PanelUniqId::parse("GW097P1").unwrap();
        assert_eq!(pid.prefix, "GW");
        assert_eq!(pid.prefix_num, 97);
        assert_eq!(pid.part_num, Some(1));
        assert!(pid.session_num.is_none());
    }

    #[test]
    fn parse_with_part_session_and_suffix() {
        let pid = PanelUniqId::parse("GW093P1AS4B").unwrap();
        assert_eq!(pid.prefix, "GW");
        assert_eq!(pid.prefix_num, 93);
        assert_eq!(pid.part_num, Some(1));
        assert_eq!(pid.session_num, Some(4));
        assert_eq!(pid.suffix, Some("AB".to_string()));
    }

    #[test]
    fn parse_long_prefix_normalized() {
        let pid = PanelUniqId::parse("SPLIT01").unwrap();
        assert_eq!(pid.prefix, "SP");
        assert_eq!(pid.prefix_num, 1);
        assert_eq!(pid.base_id(), "SP001");
    }

    #[test]
    fn parse_case_insensitive() {
        let pid = PanelUniqId::parse("gp002").unwrap();
        assert_eq!(pid.prefix, "GP");
        assert_eq!(pid.base_id(), "GP002");
    }

    #[test]
    fn parse_invalid_returns_none() {
        assert!(PanelUniqId::parse("").is_none());
        assert!(PanelUniqId::parse("INVALID").is_none());
        assert!(PanelUniqId::parse("123").is_none());
        assert!(PanelUniqId::parse("GP001-1").is_none());
    }

    #[test]
    fn full_id_round_trip() {
        let pid = PanelUniqId::parse("GW097P2S3XYZ").unwrap();
        assert_eq!(pid.full_id(), "GW097P2S3XYZ");
    }

    #[test]
    fn serde_round_trip() {
        let pid = PanelUniqId::parse("GW097P1").unwrap();
        let json = serde_json::to_string(&pid).unwrap();
        let back: PanelUniqId = serde_json::from_str(&json).unwrap();
        assert_eq!(pid, back);
    }
}
