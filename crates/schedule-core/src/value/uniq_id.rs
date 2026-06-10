/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`PanelUniqId`] — parsed representation of a spreadsheet "Uniq ID" value.
//!
//! The raw string (e.g. `"GW093P1AS4B"`) is parsed into its structural
//! components: an uppercase prefix (preserved verbatim), a numeric base,
//! optional part/session numbers, and optional trailing-alpha suffix
//! characters.

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
/// - `PREFIX` — one or more uppercase letters, preserved **verbatim** (e.g.
///   `SPLIT`, `BREAK`, `GP`). Use [`PanelUniqId::type_prefix`] for the
///   normalized 2-character panel-type lookup key.
/// - `NUM` — one or more digits (the base number for the panel series).
/// - `P<n>` — optional part number.
/// - `S<n>` — optional session number (repeat / re-run).
/// - `<suffix>` — optional trailing alpha characters after any P/S tags
///   (e.g. the `"AB"` from `"GW093P1AS4B"`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PanelUniqId {
    /// Raw uppercase prefix as typed in the spreadsheet (e.g. `"GP"`, `"SPLIT"`,
    /// `"BREAK"`). The normalized 2-character lookup key is [`Self::type_prefix`].
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
    /// Parse a raw Uniq ID string into its components, best-effort.
    ///
    /// Returns `None` only for blank input (empty or whitespace-only). Any other
    /// string parses: a code is never required and must never cause a row to be
    /// dropped (see BUGFIX-145 and the "no required fields" principle in
    /// FEATURE-043). Each structural part is optional —
    ///
    /// - `prefix` — leading run of ASCII letters (may be empty).
    /// - `prefix_num` — the digit run that follows, or `0` if absent.
    /// - `suffix` — whatever remains after pulling `P<n>`/`S<n>` tags, preserved
    ///   verbatim (it may contain non-alphanumeric characters, e.g. `"-01"`).
    ///
    /// e.g. `"123A"` → prefix `""`, num `123`, suffix `"A"`; `"BREAK"` → prefix
    /// `"BREAK"`, num `0`, no suffix; `"GP001-01"` → prefix `"GP"`, num `1`,
    /// suffix `"-01"`.
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return None;
        }
        let id_upper = trimmed.to_uppercase();

        // Leading letters → prefix (verbatim; `type_prefix()` normalizes to the
        // 2-char lookup key). The digit run that follows → number (0 if absent).
        // Everything else is the tail, parsed for P/S tags below. ASCII letters
        // and digits are single-byte, so these byte slices land on char bounds.
        let prefix: String = id_upper
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let after_prefix = &id_upper[prefix.len()..];
        let num_str: String = after_prefix
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let prefix_num: u32 = num_str.parse().unwrap_or(0);
        let mut tail = after_prefix[num_str.len()..].to_string();

        let mut part_num: Option<u32> = None;
        let mut session_num: Option<u32> = None;
        let ps_re = Regex::new(r"[PS]\d+").ok()?;
        while let Some(m) = ps_re.find(&tail) {
            let matched = m.as_str();
            let (tag, num_str) = matched.split_at(1);
            let num: u32 = num_str.parse().unwrap_or(0);
            match tag {
                "P" => part_num = Some(num),
                "S" => session_num = Some(num),
                _ => {}
            }
            tail = format!("{}{}", &tail[..m.start()], &tail[m.end()..]);
        }

        let suffix = if tail.is_empty() { None } else { Some(tail) };

        Some(PanelUniqId {
            prefix,
            prefix_num,
            part_num,
            session_num,
            suffix,
        })
    }

    /// Normalized 2-character panel-type lookup key derived from the raw
    /// [`prefix`](Self::prefix) (e.g. `"SPLIT"` → `"SP"`, `"BR"` → `"BR"`).
    ///
    /// Prefixes shorter than two characters are returned unchanged.
    #[must_use]
    pub fn type_prefix(&self) -> &str {
        if self.prefix.len() > 2 {
            &self.prefix[..2]
        } else {
            &self.prefix
        }
    }

    /// Canonical base ID: raw prefix + zero-padded 3-digit number.
    ///
    /// e.g. `"GW"` + `97` → `"GW097"`, `"SPLIT"` + `1` → `"SPLIT001"`.
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
    fn parse_long_prefix_preserved() {
        let pid = PanelUniqId::parse("SPLIT01").unwrap();
        // Raw prefix is preserved verbatim (BUGFIX-131); only the panel-type
        // lookup key is normalized to two characters.
        assert_eq!(pid.prefix, "SPLIT");
        assert_eq!(pid.type_prefix(), "SP");
        assert_eq!(pid.prefix_num, 1);
        assert_eq!(pid.base_id(), "SPLIT001");
    }

    #[test]
    fn long_prefix_full_id_round_trips() {
        // BUGFIX-131 reproduction: the raw spreadsheet value must round-trip.
        let pid = PanelUniqId::parse("SPLIT001").unwrap();
        assert_eq!(pid.full_id(), "SPLIT001");
        assert_eq!(pid.type_prefix(), "SP");

        let pid = PanelUniqId::parse("BREAK001").unwrap();
        assert_eq!(pid.full_id(), "BREAK001");
        assert_eq!(pid.type_prefix(), "BR");
    }

    #[test]
    fn type_prefix_passthrough_for_two_char() {
        let pid = PanelUniqId::parse("GP002").unwrap();
        assert_eq!(pid.prefix, "GP");
        assert_eq!(pid.type_prefix(), "GP");
    }

    #[test]
    fn parse_case_insensitive() {
        let pid = PanelUniqId::parse("gp002").unwrap();
        assert_eq!(pid.prefix, "GP");
        assert_eq!(pid.base_id(), "GP002");
    }

    #[test]
    fn parse_blank_returns_none() {
        // Only blank input is rejected — a code is never required (BUGFIX-145).
        assert!(PanelUniqId::parse("").is_none());
        assert!(PanelUniqId::parse("   ").is_none());
        assert!(PanelUniqId::parse("\t\n").is_none());
    }

    #[test]
    fn parse_is_total_for_nonblank() {
        // "Invalid"-looking codes still parse best-effort and round-trip, so the
        // import never drops the row (BUGFIX-145).

        // All letters, no number → prefix kept, num defaults to 0.
        let pid = PanelUniqId::parse("INVALID").unwrap();
        assert_eq!(pid.prefix, "INVALID");
        assert_eq!(pid.prefix_num, 0);
        assert!(pid.suffix.is_none());
        assert_eq!(pid.full_id(), "INVALID000");

        // Numberless break marker → type_prefix "BR".
        let pid = PanelUniqId::parse("BREAK").unwrap();
        assert_eq!(pid.prefix, "BREAK");
        assert_eq!(pid.type_prefix(), "BR");
        assert_eq!(pid.prefix_num, 0);
        assert_eq!(pid.full_id(), "BREAK000");

        // No prefix → empty prefix, number, then alpha suffix.
        let pid = PanelUniqId::parse("123A").unwrap();
        assert_eq!(pid.prefix, "");
        assert_eq!(pid.prefix_num, 123);
        assert_eq!(pid.suffix, Some("A".to_string()));
        assert_eq!(pid.full_id(), "123A");

        // Bare number.
        let pid = PanelUniqId::parse("123").unwrap();
        assert_eq!(pid.prefix, "");
        assert_eq!(pid.prefix_num, 123);
        assert!(pid.suffix.is_none());
        assert_eq!(pid.full_id(), "123");

        // Uniquifying "-NN" suffix is preserved verbatim and round-trips.
        let pid = PanelUniqId::parse("GP001-01").unwrap();
        assert_eq!(pid.prefix, "GP");
        assert_eq!(pid.prefix_num, 1);
        assert_eq!(pid.suffix, Some("-01".to_string()));
        assert_eq!(pid.full_id(), "GP001-01");
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
