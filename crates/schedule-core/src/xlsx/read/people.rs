/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashMap;

use anyhow::Result;
use umya_spreadsheet::Spreadsheet;

use crate::data::presenter::{PresenterGroup, PresenterMember, PresenterRank};
use crate::xlsx::columns::people;

use super::headers::{PresenterHeader, canonical_header};

pub(super) struct PresenterInfo {
    pub(super) rank: PresenterRank,
    pub(super) is_member: PresenterMember,
    pub(super) is_grouped: PresenterGroup,
}

/// Parse presenter data from a cell value, register it in the collection maps,
/// and return `(uid, is_credited)` if a presenter was found.
pub(super) fn parse_presenter_data(
    header: &PresenterHeader,
    rank: &str,
    data: &str,
    presenter_map: &mut HashMap<String, PresenterInfo>,
) -> Option<(String, bool)> {
    let data = data.trim();
    if data.is_empty() {
        return None;
    }

    // Check for * prefix → uncredited
    let (data, mut uncredited) = if let Some(rest) = data.strip_prefix('*') {
        (rest.trim(), true)
    } else {
        (data, false)
    };

    // Determine encoded_name based on header type
    let encoded_name = match header {
        PresenterHeader::Named(header_name) => {
            // For named headers, the header IS the name
            // Check if data is "Unlisted" → uncredited
            if data.eq_ignore_ascii_case("unlisted") {
                uncredited = true;
            }
            header_name.clone()
        }
        PresenterHeader::Other => {
            // For Other headers, the cell data IS the name
            data.to_string()
        }
    };

    if encoded_name.is_empty() {
        return None;
    }

    // Split on first '=' to get presenter and optional group
    let (presenter_raw, group_raw) = if let Some(eq_pos) = encoded_name.find('=') {
        let name_part = encoded_name[..eq_pos].trim().to_string();
        let group_part = encoded_name[eq_pos + 1..].trim().to_string();
        (
            name_part,
            if group_part.is_empty() {
                None
            } else {
                Some(group_part)
            },
        )
    } else {
        (encoded_name, None)
    };

    // Check if presenter begins with '<' → always_grouped
    let (presenter_name, always_grouped) = if let Some(rest) = presenter_raw.strip_prefix('<') {
        (rest.trim().to_string(), true)
    } else {
        (presenter_raw, false)
    };

    // Check if group begins with '=' (original was '==') → always_shown_group
    let (group_name, always_shown_group) = match group_raw {
        Some(g) => {
            if let Some(rest) = g.strip_prefix('=') {
                (Some(rest.trim().to_string()), true)
            } else {
                (Some(g), false)
            }
        }
        None => (None, false),
    };
    // Filter out empty group after stripping
    let group_name = group_name.filter(|g| !g.is_empty());

    // Initialize group entry in presenter_map if needed
    if let Some(ref g) = group_name {
        let entry = presenter_map
            .entry(g.clone())
            .or_insert_with(|| PresenterInfo {
                rank: PresenterRank::from_str(rank),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
            });
        // Update existing group entry if needed
        match &mut entry.is_grouped {
            PresenterGroup::IsGroup(_, shown) => {
                *shown = *shown || always_shown_group;
            }
            PresenterGroup::NotGroup => {
                entry.is_grouped =
                    PresenterGroup::IsGroup(std::collections::BTreeSet::new(), always_shown_group);
            }
        }
    }

    // If presenter name is empty but group is present, the presenter IS the group
    if presenter_name.is_empty() || Some(presenter_name.clone()) == group_name {
        return match group_name {
            Some(ref g) => Some((g.clone(), !uncredited)),
            None => None,
        };
    }

    // Handle group membership first if we have a group
    if let Some(ref group_name) = group_name {
        // Get or create the group entry
        let group_entry =
            presenter_map
                .entry(group_name.clone())
                .or_insert_with(|| PresenterInfo {
                    rank: PresenterRank::from_str(rank),
                    is_member: PresenterMember::NotMember,
                    is_grouped: PresenterGroup::NotGroup,
                });

        // Add presenter to group's members
        if let PresenterGroup::IsGroup(members, _) = &mut group_entry.is_grouped {
            members.insert(presenter_name.clone());
        }
    }

    // Now register the presenter in the map
    let presenter_name_for_entry = presenter_name.clone();
    let entry = presenter_map
        .entry(presenter_name_for_entry)
        .or_insert_with(|| PresenterInfo {
            rank: PresenterRank::from_str(rank),
            is_member: PresenterMember::NotMember,
            is_grouped: PresenterGroup::NotGroup,
        });

    // Set presenter's group membership if we have a group
    if let Some(ref group_name) = group_name {
        match &mut entry.is_member {
            PresenterMember::IsMember(groups, grouped) => {
                groups.insert(group_name.clone());
                *grouped = *grouped || always_grouped;
            }
            PresenterMember::NotMember => {
                entry.is_member = PresenterMember::IsMember(
                    std::collections::BTreeSet::from([group_name.clone()]),
                    always_grouped,
                );
            }
        }
    }

    Some((presenter_name, !uncredited))
}

pub(super) fn read_presenter_ranks(
    book: &Spreadsheet,
    _file_path: &str,
) -> Result<HashMap<String, String>> {
    let mut ranks = HashMap::new();

    // Try People sheet first, then legacy Presenters sheet name
    let ws = book
        .get_sheet_by_name("People")
        .or_else(|| book.get_sheet_by_name("Presenters"));

    if let Some(ws) = ws {
        let max_col = ws.get_highest_column();
        let mut header_map: HashMap<String, u32> = HashMap::new();
        for col in 1..=max_col {
            let value = ws.get_value((col, 1));
            if let Some(key) = canonical_header(&value) {
                header_map.entry(key).or_insert(col);
            }
        }

        let name_col = people::NAME.keys().find_map(|k| header_map.get(k).copied());
        let rank_col = people::CLASSIFICATION
            .keys()
            .find_map(|k| header_map.get(k).copied());

        if let (Some(name_col), Some(rank_col)) = (name_col, rank_col) {
            let highest_row = ws.get_highest_row();
            for row in 2..=highest_row {
                let name = ws.get_value((name_col, row)).trim().to_string();
                let rank = ws.get_value((rank_col, row)).trim().to_string();

                if !name.is_empty() && !rank.is_empty() {
                    ranks.insert(name, rank);
                }
            }
        }
    }

    Ok(ranks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_presenter_map() -> HashMap<String, PresenterInfo> {
        HashMap::new()
    }

    #[test]
    fn test_parse_data_named_simple() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("Yaya Han".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "Yaya Han");
        assert!(credited);
        assert!(pm.contains_key("Yaya Han"));
    }

    #[test]
    fn test_parse_data_named_unlisted() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("Secret Guest".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Unlisted", &mut pm).expect("should parse");
        assert_eq!(uid, "Secret Guest");
        assert!(!credited, "Unlisted should be uncredited");
    }

    #[test]
    fn test_parse_data_named_star_uncredited() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("Helper".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "*Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "Helper");
        assert!(!credited, "* prefix should be uncredited");
    }

    #[test]
    fn test_parse_data_named_with_double_eq_group() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("John==UNC Staff".to_string());
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "John");
        let mut expected_groups = std::collections::BTreeSet::new();
        expected_groups.insert("UNC Staff".to_string());
        let john_groups = match &pm["John"].is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert_eq!(john_groups, &expected_groups);
        let john_always_grouped = match &pm["John"].is_member {
            PresenterMember::IsMember(_, always_grouped) => *always_grouped,
            PresenterMember::NotMember => false,
        };
        assert!(!john_always_grouped);
        // Check that UNC Staff group was created with always_shown=true
        if let Some(unc_staff_info) = pm.get("UNC Staff") {
            let is_always_shown = match &unc_staff_info.is_grouped {
                PresenterGroup::IsGroup(_, always_shown) => *always_shown,
                PresenterGroup::NotGroup => false,
            };
            assert!(
                is_always_shown,
                "UNC Staff should be always_shown due to == prefix"
            );
        } else {
            panic!("UNC Staff group should have been created");
        }
    }

    #[test]
    fn test_parse_data_named_lt_always_grouped() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("<Jane=UNC Staff".to_string());
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "Jane");
        let jane_always_grouped = match &pm["Jane"].is_member {
            PresenterMember::IsMember(_, always_grouped) => *always_grouped,
            PresenterMember::NotMember => false,
        };
        assert!(jane_always_grouped, "< prefix should set always_grouped");
        let mut expected_jane_groups = std::collections::BTreeSet::new();
        expected_jane_groups.insert("UNC Staff".to_string());
        let jane_groups = match &pm["Jane"].is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert_eq!(jane_groups, &expected_jane_groups);
        // Check that UNC Staff group was created but NOT always_shown (single =)
        if let Some(unc_staff_info) = pm.get("UNC Staff") {
            let is_always_shown = match &unc_staff_info.is_grouped {
                PresenterGroup::IsGroup(_, always_shown) => *always_shown,
                PresenterGroup::NotGroup => false,
            };
            assert!(
                !is_always_shown,
                "single = should not set always_shown_group"
            );
        }
    }

    #[test]
    fn test_parse_data_named_lt_double_eq_combined() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("<Bob==Team".to_string());
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "Bob");
        assert!(
            match &pm["Bob"].is_member {
                PresenterMember::IsMember(_, always_grouped) => *always_grouped,
                PresenterMember::NotMember => false,
            },
            "< prefix should set always_grouped"
        );
        // Check that Team group was created with always_shown=true
        if let Some(team_info) = pm.get("Team") {
            let is_always_shown = match &team_info.is_grouped {
                PresenterGroup::IsGroup(_, always_shown) => *always_shown,
                PresenterGroup::NotGroup => false,
            };
            assert!(is_always_shown, "== should set always_shown_group");
        }
    }

    #[test]
    fn test_parse_data_other_simple() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Other;
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Alice", &mut pm).expect("should parse");
        assert_eq!(uid, "Alice");
        assert!(credited);
        assert!(pm.contains_key("Alice"));
    }

    #[test]
    fn test_parse_data_other_with_group() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Other;
        let (uid, _credited) =
            parse_presenter_data(&header, "guest", "Triffin Morris=UNC Staff", &mut pm)
                .expect("should parse");
        assert_eq!(uid, "Triffin Morris");
        let mut expected_triffin_groups = std::collections::BTreeSet::new();
        expected_triffin_groups.insert("UNC Staff".to_string());
        let triffin_groups = match &pm["Triffin Morris"].is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert_eq!(triffin_groups, &expected_triffin_groups);
        // Check that UNC Staff group was created but NOT always_shown (single =)
        if let Some(unc_staff_info) = pm.get("UNC Staff") {
            let is_always_shown = match &unc_staff_info.is_grouped {
                PresenterGroup::IsGroup(_, always_shown) => *always_shown,
                PresenterGroup::NotGroup => false,
            };
            assert!(
                !is_always_shown,
                "single = should not set always_shown_group"
            );
        }
    }

    #[test]
    fn test_parse_data_other_with_double_eq_group() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Other;
        let (uid, _credited) = parse_presenter_data(
            &header,
            "guest",
            &"Triffin Morris==UNC Staff".replace("==", "=="),
            &mut pm,
        )
        .expect("should parse");
        assert_eq!(uid, "Triffin Morris");
        // Check that UNC Staff group was created with always_shown=true
        if let Some(unc_staff_info) = pm.get("UNC Staff") {
            let is_always_shown = match &unc_staff_info.is_grouped {
                PresenterGroup::IsGroup(_, always_shown) => *always_shown,
                PresenterGroup::NotGroup => false,
            };
            assert!(is_always_shown, "== should set always_shown_group");
        }
    }

    #[test]
    fn test_parse_data_other_star_uncredited() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Other;
        let (uid, credited) = parse_presenter_data(
            &header,
            "guest",
            &"*Triffin Morris=UNC Staff".replace("=", "=="),
            &mut pm,
        )
        .expect("should parse");
        assert_eq!(uid, "Triffin Morris");
        assert!(!credited, "* prefix should be uncredited");
    }

    #[test]
    fn test_parse_data_blank_returns_none() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Other;
        assert!(parse_presenter_data(&header, "guest", "", &mut pm).is_none());
        assert!(parse_presenter_data(&header, "guest", "  ", &mut pm).is_none());
    }

    #[test]
    fn test_parse_data_empty_name_with_group() {
        let mut pm = empty_presenter_map();
        let header = PresenterHeader::Named("==UNC Staff".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");
        assert_eq!(uid, "UNC Staff", "empty name should use group as uid");
        assert!(credited);
        assert!(pm.contains_key("UNC Staff"));

        // Verify that UNC Staff is NOT a member of itself (no circular reference)
        let unc_staff_info = pm.get("UNC Staff").unwrap();
        let unc_staff_groups = match &unc_staff_info.is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert!(
            unc_staff_groups.is_empty(),
            "UNC Staff should not have itself as a group"
        );

        // Verify that UNC Staff is not in group_members as a member of itself
        let group_members = match &unc_staff_info.is_grouped {
            PresenterGroup::IsGroup(members, _) => members,
            PresenterGroup::NotGroup => &std::collections::BTreeSet::new(),
        };
        assert!(
            !group_members.contains(&"UNC Staff".to_string()),
            "UNC Staff should not be in group_members as its own member"
        );

        // Verify that UNC Staff is in always_shown_groups (due to == prefix)
        let is_always_shown = match &unc_staff_info.is_grouped {
            PresenterGroup::IsGroup(_, always_shown) => *always_shown,
            PresenterGroup::NotGroup => false,
        };
        assert!(is_always_shown, "UNC Staff should be always_shown");
    }

    #[test]
    fn test_parse_unc_staff_circular_reference_bug() {
        let mut pm = empty_presenter_map();

        // Test case that caused the bug: G:==UNC Staff
        let header = PresenterHeader::Named("==UNC Staff".to_string());
        let (uid, credited) =
            parse_presenter_data(&header, "guest", "Yes", &mut pm).expect("should parse");

        assert_eq!(uid, "UNC Staff");
        assert!(credited);

        // Verify the presenter is registered
        assert!(pm.contains_key("UNC Staff"));
        let presenter_info = pm.get("UNC Staff").unwrap();

        // CRITICAL: UNC Staff should not be a member of itself
        let presenter_groups = match &presenter_info.is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert!(
            presenter_groups.is_empty(),
            "UNC Staff should not have any groups when it's the group itself"
        );

        // CRITICAL: UNC Staff should not have itself as a member
        let group_members = match &presenter_info.is_grouped {
            PresenterGroup::IsGroup(members, _) => members,
            PresenterGroup::NotGroup => &std::collections::BTreeSet::new(),
        };
        assert!(
            !group_members.contains(&"UNC Staff".to_string()),
            "UNC Staff should not be listed as a member of itself"
        );

        // UNC Staff should be always_shown due to == prefix
        let is_always_shown = match &presenter_info.is_grouped {
            PresenterGroup::IsGroup(_, always_shown) => *always_shown,
            PresenterGroup::NotGroup => false,
        };
        assert!(is_always_shown, "UNC Staff should be always_shown");
    }

    #[test]
    fn test_parse_presenter_with_prefixes() {
        let mut presenter_map: HashMap<String, PresenterInfo> = HashMap::new();

        // Test <Name prefix (always_grouped)
        let header = PresenterHeader::Other;
        let result = parse_presenter_data(
            &header,
            "fan_panelist",
            "<John Doe=Test Group",
            &mut presenter_map,
        );
        assert_eq!(result, Some(("John Doe".to_string(), true)));

        // Check John Doe's always_grouped status and group membership
        let john_presenter = presenter_map.get("John Doe").unwrap();
        let is_always_grouped = match &john_presenter.is_member {
            PresenterMember::IsMember(_, always_grouped) => *always_grouped,
            PresenterMember::NotMember => false,
        };
        assert!(is_always_grouped);
        let mut expected_groups = std::collections::BTreeSet::new();
        expected_groups.insert("Test Group".to_string());
        let john_groups = match &john_presenter.is_member {
            PresenterMember::IsMember(groups, _) => groups,
            PresenterMember::NotMember => &std::collections::BTreeSet::new(),
        };
        assert_eq!(john_groups, &expected_groups);

        // Check Test Group exists and has John as member
        let test_group = presenter_map.get("Test Group").unwrap();
        if let PresenterGroup::IsGroup(members, _) = &test_group.is_grouped {
            assert!(members.contains("John Doe"));
        } else {
            panic!("Test Group should be a group");
        }
    }
}
