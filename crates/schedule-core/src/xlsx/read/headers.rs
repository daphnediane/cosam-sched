/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use regex::Regex;

use crate::data::presenter::PresenterRank;

/// Normalise an Excel column header to a canonical key used for lookup.
/// Converts runs of whitespace/punctuation to `_` and trims leading/trailing `_`.
/// Returns `None` for empty or whitespace-only input.
pub(crate) fn canonical_header(header: &str) -> Option<String> {
    let trimmed = header.trim();
    if trimmed.is_empty() {
        return None;
    }
    let result = Regex::new(r"[\s/:().,]+")
        .expect("valid regex")
        .replace_all(trimmed, "_");
    let result = result.trim_matches('_');
    if result.is_empty() {
        return None;
    }
    Some(result.to_string())
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
        assert_eq!(canonical_header("Start Time"), Some("Start_Time".into()));
        assert_eq!(canonical_header("Start_Time"), Some("Start_Time".into()));
        assert_eq!(canonical_header("  Room  "), Some("Room".into()));
        assert_eq!(canonical_header("Uniq ID"), Some("Uniq_ID".into()));
        assert_eq!(canonical_header(""), None);
        assert_eq!(canonical_header("   "), None);
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
