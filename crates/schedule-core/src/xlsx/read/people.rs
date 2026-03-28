/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashMap;

use anyhow::Result;
use umya_spreadsheet::Spreadsheet;

use crate::data::presenter::PresenterRank;
use crate::xlsx::columns::people;

/// Intermediate presenter data read from the People/Presenter sheet.
#[derive(Default, Clone)]
pub(super) struct PresenterInfo {
    pub(super) rank: PresenterRank,
    pub(super) is_group: bool,
    pub(super) always_grouped: bool,
    pub(super) always_shown: bool,
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
                    info.is_group = super::is_truthy(is_group_val);
                }

                // Note: Members and Groups columns are processed later to build relationships
                // The actual member/group relationships are handled in the calling code

                // Read always_grouped
                if let Some(always_grouped_val) =
                    super::get_field_def(&row_data, &people::ALWAYS_GROUPED)
                {
                    info.always_grouped = super::is_truthy(always_grouped_val);
                }

                // Read always_shown
                if let Some(always_shown_val) =
                    super::get_field_def(&row_data, &people::ALWAYS_SHOWN)
                {
                    info.always_shown = super::is_truthy(always_shown_val);
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
                sort_rank: None,
                metadata: None,
                source: Some(crate::data::source_info::SourceInfo {
                    file_path: Some(_file_path.to_string()),
                    sheet_name: Some(people_table.to_string()),
                    row_index: Some(row_index as u32 + 1),
                }),
                change_state: Default::default(),

                // Relationship fields (empty for XLSX import)
                add_groups: Vec::new(),
                add_members: Vec::new(),
                is_group: None,
                always_grouped: None,
                always_shown: None,
            },
        );
    }

    Ok(())
}

// Tests for update_or_create_presenter (which replaced parse_presenter_data)
// live in crate::edit::tests — see test_update_or_create_presenter_* functions.
