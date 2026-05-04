/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Spreadsheet column header normalization and presenter-column detection.

use regex::Regex;

use crate::tables::presenter::PresenterRank;

/// Normalise an Excel column header to a canonical lookup key.
///
/// Steps applied in order:
/// 1. Split at camelCase lowercase→uppercase boundaries (`PanelKind` → `Panel Kind`).
/// 2. Split at uppercase-run → UpperCamelCase boundaries (`AVNotes` → `AV Notes`).
/// 3. Convert runs of whitespace / punctuation / underscores to `_` and trim.
///
/// Returns `None` for empty or whitespace-only input.
pub fn canonical_header(header: &str) -> Option<String> {
    let trimmed = header.trim();
    if trimmed.is_empty() {
        return None;
    }
    let re_lc_uc = Regex::new(r"([a-z])([A-Z])").expect("valid regex");
    let s = re_lc_uc.replace_all(trimmed, "${1} ${2}");
    let re_uc_run = Regex::new(r"([A-Z]+)([A-Z][a-z])").expect("valid regex");
    let s = re_uc_run.replace_all(&s, "${1} ${2}");
    let re_sep = Regex::new(r"[\s_/:().,]+").expect("valid regex");
    let s = re_sep.replace_all(&s, "_");
    let s = s.trim_matches('_');
    if s.is_empty() {
        return None;
    }
    Some(s.to_string())
}

/// A detected presenter column on the schedule sheet.
#[derive(Debug)]
pub(crate) struct PresenterColumn {
    /// 1-based column index in the worksheet.
    pub(crate) col: u32,
    /// Rank implied by the column prefix (`G:`, `S:`, etc.).
    pub(crate) rank: PresenterRank,
    /// Whether this is a named-individual column or an "Other" bucket.
    pub(crate) header: PresenterHeader,
}

/// The semantic kind of a presenter column.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PresenterHeader {
    /// A column for a single named individual.  The string is the full rest of
    /// the header after the `Kind:` prefix, which may include `=Group` or
    /// `==Group` modifiers and is passed directly to
    /// `find_or_create_tagged_presenter`.
    Named(String),
    /// An "Other" bucket: the cell contains a comma-separated list of names.
    Other,
}

/// Parse a raw header string into a [`PresenterColumn`], or return `None` if
/// the header is not a presenter column.
///
/// Recognised formats:
/// - `G:Name`, `G:Name==Group`, `G:Other` (tagged format, 2022+)
/// - `Other Guests`, `Other Staff` (legacy fallback)
/// - `Fan Panelist`, `Other`, `Others` (legacy fallback)
pub(crate) fn parse_presenter_header(header: &str, col: u32) -> Option<PresenterColumn> {
    let header = header.trim();
    if header.is_empty() {
        return None;
    }

    // Tagged format: `[GJSIPF]:Rest`
    let re_kind = Regex::new(r"(?i)^([GJSIPF]):(.+)$").expect("valid regex");
    if let Some(caps) = re_kind.captures(header) {
        let prefix_char = caps[1].chars().next()?;
        let rank = PresenterRank::from_prefix_char(prefix_char)?;
        let rest = caps[2].trim().to_string();
        if rest.is_empty() {
            return None;
        }
        let kind = if rest.eq_ignore_ascii_case("other") {
            PresenterHeader::Other
        } else {
            PresenterHeader::Named(rest)
        };
        return Some(PresenterColumn {
            col,
            rank,
            header: kind,
        });
    }

    // Legacy fallbacks
    let lower = header.to_lowercase();
    if lower == "other guests" || lower == "other guest" {
        return Some(PresenterColumn {
            col,
            rank: PresenterRank::Guest,
            header: PresenterHeader::Other,
        });
    }
    if lower == "other staff" {
        return Some(PresenterColumn {
            col,
            rank: PresenterRank::Staff,
            header: PresenterHeader::Other,
        });
    }
    if lower == "fan panelist" || lower.starts_with("other") {
        return Some(PresenterColumn {
            col,
            rank: PresenterRank::FanPanelist,
            header: PresenterHeader::Other,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_header_basic() {
        assert_eq!(canonical_header("Start Time"), Some("Start_Time".into()));
        assert_eq!(canonical_header("Start_Time"), Some("Start_Time".into()));
        assert_eq!(canonical_header("  Room  "), Some("Room".into()));
        assert_eq!(canonical_header("Uniq ID"), Some("Uniq_ID".into()));
        assert_eq!(canonical_header(""), None);
        assert_eq!(canonical_header("   "), None);
    }

    #[test]
    fn test_canonical_header_camel_case() {
        assert_eq!(canonical_header("RoomName"), Some("Room_Name".into()));
        assert_eq!(canonical_header("AVNotes"), Some("AV_Notes".into()));
        assert_eq!(canonical_header("UniqID"), Some("Uniq_ID".into()));
        assert_eq!(
            canonical_header("Notes (Non Printing)"),
            Some("Notes_Non_Printing".into())
        );
        assert_eq!(canonical_header("PreReg Max"), Some("Pre_Reg_Max".into()));
        assert_eq!(
            canonical_header("SimpleTix Event"),
            Some("Simple_Tix_Event".into())
        );
    }

    #[test]
    fn test_parse_presenter_header_tagged_name() {
        let col = parse_presenter_header("G:Yaya Han", 5).unwrap();
        assert_eq!(col.rank, PresenterRank::Guest);
        assert_eq!(col.header, PresenterHeader::Named("Yaya Han".to_string()));
    }

    #[test]
    fn test_parse_presenter_header_tagged_name_with_group() {
        let col = parse_presenter_header("G:John==UNC Staff", 1).unwrap();
        assert_eq!(col.rank, PresenterRank::Guest);
        assert_eq!(
            col.header,
            PresenterHeader::Named("John==UNC Staff".to_string())
        );
    }

    #[test]
    fn test_parse_presenter_header_other() {
        let col = parse_presenter_header("S:Other", 3).unwrap();
        assert_eq!(col.rank, PresenterRank::Staff);
        assert_eq!(col.header, PresenterHeader::Other);
    }

    #[test]
    fn test_parse_presenter_header_legacy_other_guests() {
        let col = parse_presenter_header("Other Guests", 0).unwrap();
        assert_eq!(col.rank, PresenterRank::Guest);
        assert_eq!(col.header, PresenterHeader::Other);
    }

    #[test]
    fn test_parse_presenter_header_not_presenter() {
        assert!(parse_presenter_header("Room", 0).is_none());
        assert!(parse_presenter_header("Name", 0).is_none());
        assert!(parse_presenter_header("Duration", 0).is_none());
    }
}
