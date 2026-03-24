/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use regex::Regex;

use crate::data::presenter::PresenterRank;

/// Normalise an Excel column header to a canonical key used for lookup.
///
/// Steps applied in order:
/// 1. Split at camelCase lowercase→uppercase boundaries (`PanelKind` → `Panel Kind`).
/// 2. Split at uppercase-run → UpperCamelCase boundaries (`AVNotes` → `AV Notes`).
/// 3. Convert runs of whitespace/punctuation/underscores to `_` and trim.
///
/// Returns `None` for empty or whitespace-only input.
pub(crate) fn canonical_header(header: &str) -> Option<String> {
    let trimmed = header.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Step 1: lowercase → uppercase boundary (e.g. "PanelKind" → "Panel Kind")
    let re_lc_uc = Regex::new(r"([a-z])([A-Z])").expect("valid regex");
    let s = re_lc_uc.replace_all(trimmed, "${1} ${2}");
    // Step 2: uppercase-run before UpperCamelCase word (e.g. "AVNotes" → "AV Notes")
    let re_uc_run = Regex::new(r"([A-Z]+)([A-Z][a-z])").expect("valid regex");
    let s = re_uc_run.replace_all(&s, "${1} ${2}");
    // Step 3: normalise separators (whitespace, underscore, punctuation) to a single `_`
    let re_sep = Regex::new(r"[\s_/:().,]+").expect("valid regex");
    let s = re_sep.replace_all(&s, "_");
    let s = s.trim_matches('_');
    if s.is_empty() {
        return None;
    }
    Some(s.to_string())
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PresenterHeader {
    Named(String),
    Other,
}

#[derive(Debug)]
pub(crate) struct PresenterColumn {
    pub(crate) col: u32,
    pub(crate) rank: PresenterRank,
    pub(crate) header: PresenterHeader,
}

pub(crate) fn parse_presenter_header(header: &str, col: u32) -> Option<PresenterColumn> {
    let header = header.trim();
    if header.is_empty() {
        return None;
    }

    // Kind:Rest format — [GJSIP]:...
    let re_kind = Regex::new(r"(?i)^([GJSIP]):(.+)$").expect("valid regex");
    if let Some(caps) = re_kind.captures(header) {
        let prefix_char = caps[1].chars().next()?;
        let rank = PresenterRank::from_prefix_char(prefix_char)?;
        let rest = caps[2].trim().to_string();
        if rest.is_empty() {
            return None;
        }
        let header_kind = if rest.eq_ignore_ascii_case("other") {
            PresenterHeader::Other
        } else {
            PresenterHeader::Named(rest)
        };
        return Some(PresenterColumn {
            col,
            rank,
            header: header_kind,
        });
    }

    // "Other Guests" → guest other, "Other Staff" → staff other
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

    // "Fan Panelist" or generic "Other"/"Others"
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
    fn test_canonical_header() {
        // Existing space/underscore normalization
        assert_eq!(canonical_header("Start Time"), Some("Start_Time".into()));
        assert_eq!(canonical_header("Start_Time"), Some("Start_Time".into()));
        assert_eq!(canonical_header("  Room  "), Some("Room".into()));
        assert_eq!(canonical_header("Uniq ID"), Some("Uniq_ID".into()));
        assert_eq!(canonical_header(""), None);
        assert_eq!(canonical_header("   "), None);
        // CamelCase splitting — lowercase→uppercase boundary
        assert_eq!(canonical_header("RoomName"), Some("Room_Name".into()));
        assert_eq!(canonical_header("PanelKind"), Some("Panel_Kind".into()));
        assert_eq!(canonical_header("SortKey"), Some("Sort_Key".into()));
        assert_eq!(canonical_header("HotelRoom"), Some("Hotel_Room".into()));
        // CamelCase splitting — UniqID type (lowercase→uppercase before all-caps)
        assert_eq!(canonical_header("UniqID"), Some("Uniq_ID".into()));
        // CamelCase splitting — uppercase-run before UpperCamelCase (e.g. AVNotes)
        assert_eq!(canonical_header("AVNotes"), Some("AV_Notes".into()));
        assert_eq!(canonical_header("AVNote"), Some("AV_Note".into()));
        // Already underscore/space — unchanged by camelCase step
        assert_eq!(canonical_header("AV_Notes"), Some("AV_Notes".into()));
        assert_eq!(canonical_header("AV Notes"), Some("AV_Notes".into()));
        // Punctuation stripping
        assert_eq!(
            canonical_header("Notes (Non Printing)"),
            Some("Notes_Non_Printing".into())
        );
        // Multi-word camelCase forms from old spreadsheets
        assert_eq!(canonical_header("IsTimeLine"), Some("Is_Time_Line".into()));
        assert_eq!(canonical_header("Is Timeline"), Some("Is_Timeline".into()));
        assert_eq!(canonical_header("IsBreak"), Some("Is_Break".into()));
        assert_eq!(
            canonical_header("IsRoomHours"),
            Some("Is_Room_Hours".into())
        );
        assert_eq!(canonical_header("IsPrivate"), Some("Is_Private".into()));
        // PreReg split
        assert_eq!(canonical_header("PreReg Max"), Some("Pre_Reg_Max".into()));
        assert_eq!(canonical_header("PreRegMax"), Some("Pre_Reg_Max".into()));
        // SimpleTix
        assert_eq!(
            canonical_header("SimpleTix Event"),
            Some("Simple_Tix_Event".into())
        );
    }

    #[test]
    fn test_parse_presenter_header_kind_name() {
        let col = parse_presenter_header("G:Yaya Han", 5).expect("should parse");
        assert_eq!(col.rank, PresenterRank::Guest);
        assert_eq!(col.header, PresenterHeader::Named("Yaya Han".to_string()));
    }

    #[test]
    fn test_parse_presenter_header_kind_name_with_group() {
        // Header stores full rest including =Group; parsing happens in parse_presenter_data
        let col = parse_presenter_header("G:John==UNC Staff", 1).expect("should parse");
        assert_eq!(col.rank, PresenterRank::Guest);
        assert_eq!(
            col.header,
            PresenterHeader::Named("John==UNC Staff".to_string())
        );
    }

    #[test]
    fn test_parse_presenter_header_kind_other() {
        let col = parse_presenter_header("S:Other", 3).expect("should parse");
        assert_eq!(col.rank, PresenterRank::Staff);
        assert_eq!(col.header, PresenterHeader::Other);
    }

    #[test]
    fn test_parse_presenter_header_other_guests() {
        let col = parse_presenter_header("Other Guests", 0).expect("should parse");
        assert_eq!(col.rank, PresenterRank::Guest);
        assert_eq!(col.header, PresenterHeader::Other);
    }

    #[test]
    fn test_parse_presenter_header_fan_panelist() {
        let col = parse_presenter_header("Fan Panelist", 0).expect("should parse");
        assert_eq!(col.rank, PresenterRank::FanPanelist);
        assert_eq!(col.header, PresenterHeader::Other);
    }

    #[test]
    fn test_parse_presenter_header_not_presenter() {
        assert!(parse_presenter_header("Room", 0).is_none());
        assert!(parse_presenter_header("Name", 0).is_none());
        assert!(parse_presenter_header("Duration", 0).is_none());
        assert!(parse_presenter_header("g1", 0).is_none());
        assert!(parse_presenter_header("Guest1", 0).is_none());
    }
}
