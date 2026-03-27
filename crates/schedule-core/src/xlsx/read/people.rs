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

/// Intermediate presenter data read from the People/Presenter sheet.
#[derive(Default, Clone)]
pub(super) struct PresenterInfo {
    pub(super) rank: PresenterRank,
    pub(super) is_member: PresenterMember,
    pub(super) is_grouped: PresenterGroup,
}

/// Read full presenter data from People/Presenter sheet including all columns
pub(super) fn read_presenter_data(
    book: &Spreadsheet,
    people_table: &str,
) -> Result<HashMap<String, PresenterInfo>> {
    let mut presenters = HashMap::new();

    // Use find_data_range for consistent table/sheet lookup with fallbacks
    let range = match super::find_data_range(
        book,
        people_table,
        &["Presenters", "Presenter", "People", "Person"],
    ) {
        Some(r) => r,
        None => return Ok(presenters),
    };

    let ws = book.get_sheet_by_name(&range.sheet_name).unwrap();
    let (raw_headers, canonical_headers, _col_map) = super::build_column_map(ws, &range);

    let highest_row = ws.get_highest_row();
    for row in 2..=highest_row {
        let row_data = super::row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        // Get presenter name
        if let Some(name) = super::get_field_def(&row_data, &people::NAME) {
            let name = name.trim();
            if !name.is_empty() {
                let mut info = PresenterInfo::default();

                // Read classification/rank
                if let Some(classification) =
                    super::get_field_def(&row_data, &people::CLASSIFICATION)
                {
                    info.rank =
                        crate::data::presenter::PresenterRank::from_classification(classification);
                }

                // Read group relationships
                if let Some(is_group_val) = super::get_field_def(&row_data, &people::IS_GROUP) {
                    if super::is_truthy(is_group_val) {
                        info.is_grouped = crate::data::presenter::PresenterGroup::IsGroup(
                            std::collections::BTreeSet::new(),
                            false,
                        );
                    }
                }

                // Read members
                if let Some(members_val) = super::get_field_def(&row_data, &people::MEMBERS) {
                    if !matches!(
                        info.is_grouped,
                        crate::data::presenter::PresenterGroup::IsGroup(_, _)
                    ) {
                        info.is_grouped = crate::data::presenter::PresenterGroup::IsGroup(
                            std::collections::BTreeSet::new(),
                            false,
                        );
                    }
                    if let crate::data::presenter::PresenterGroup::IsGroup(ref mut members, _) =
                        info.is_grouped
                    {
                        for member in members_val.split(',') {
                            let member = member.trim();
                            if !member.is_empty() {
                                members.insert(member.to_string());
                            }
                        }
                    }
                }

                // Read groups
                if let Some(groups_val) = super::get_field_def(&row_data, &people::GROUPS) {
                    let groups: std::collections::BTreeSet<String> = groups_val
                        .split(',')
                        .map(|g| g.trim().to_string())
                        .filter(|g| !g.is_empty())
                        .collect();

                    if !groups.is_empty() {
                        info.is_member =
                            crate::data::presenter::PresenterMember::IsMember(groups, false);
                    }
                }

                // Read always_grouped
                if let Some(always_grouped_val) =
                    super::get_field_def(&row_data, &people::ALWAYS_GROUPED)
                {
                    if super::is_truthy(always_grouped_val) {
                        match &mut info.is_member {
                            crate::data::presenter::PresenterMember::IsMember(
                                _,
                                always_grouped,
                            ) => {
                                *always_grouped = true;
                            }
                            crate::data::presenter::PresenterMember::NotMember => {
                                info.is_member = crate::data::presenter::PresenterMember::IsMember(
                                    std::collections::BTreeSet::new(),
                                    true,
                                );
                            }
                        }
                    }
                }

                // Read always_shown
                if let Some(always_shown_val) =
                    super::get_field_def(&row_data, &people::ALWAYS_SHOWN)
                {
                    if super::is_truthy(always_shown_val) {
                        match &mut info.is_grouped {
                            crate::data::presenter::PresenterGroup::IsGroup(_, always_shown) => {
                                *always_shown = true;
                            }
                            crate::data::presenter::PresenterGroup::NotGroup => {
                                info.is_grouped = crate::data::presenter::PresenterGroup::IsGroup(
                                    std::collections::BTreeSet::new(),
                                    true,
                                );
                            }
                        }
                    }
                }

                presenters.insert(name.to_string(), info);
            }
        }
    }

    Ok(presenters)
}

/// Read presenters from People/Presenter sheet into the edit context
pub(super) fn read_presenters_into(
    book: &Spreadsheet,
    people_table: &str,
    _file_path: &str,
    ctx: &mut crate::edit::EditContext,
) -> Result<()> {
    let presenter_data = read_presenter_data(book, people_table)?;

    for (row_index, (name, info)) in presenter_data.into_iter().enumerate() {
        ctx.find_or_create_presenter(
            &name,
            &crate::edit::find::PresenterOptions {
                rank: Some(info.rank.clone()),
                add_groups: match &info.is_member {
                    crate::data::presenter::PresenterMember::IsMember(groups, _) => {
                        groups.iter().cloned().collect()
                    }
                    crate::data::presenter::PresenterMember::NotMember => Vec::new(),
                },
                add_members: match &info.is_grouped {
                    crate::data::presenter::PresenterGroup::IsGroup(members, _) => {
                        members.iter().cloned().collect()
                    }
                    crate::data::presenter::PresenterGroup::NotGroup => Vec::new(),
                },
                is_group: Some(matches!(
                    info.is_grouped,
                    crate::data::presenter::PresenterGroup::IsGroup(_, _)
                )),
                always_grouped: match &info.is_member {
                    crate::data::presenter::PresenterMember::IsMember(_, grouped) => Some(*grouped),
                    crate::data::presenter::PresenterMember::NotMember => Some(false),
                },
                always_shown: match &info.is_grouped {
                    crate::data::presenter::PresenterGroup::IsGroup(_, shown) => Some(*shown),
                    crate::data::presenter::PresenterGroup::NotGroup => Some(false),
                },
                sort_rank: Some(crate::data::presenter::PresenterSortRank::people(
                    row_index as u32,
                )),
                metadata: None,
                source: Some(crate::data::source_info::SourceInfo {
                    file_path: Some(_file_path.to_string()),
                    sheet_name: Some(people_table.to_string()),
                    row_index: None,
                }),
                change_state: Some(crate::data::source_info::ChangeState::Converted),
            },
        );
    }

    Ok(())
}

// Tests for update_or_create_presenter (which replaced parse_presenter_data)
// live in crate::edit::tests — see test_update_or_create_presenter_* functions.
