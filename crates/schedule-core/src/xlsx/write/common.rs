/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashSet;

use indexmap::IndexMap;
use umya_spreadsheet::structs::{Table, TableColumn, TableStyleInfo, Worksheet};

use crate::data::panel::ExtraValue;
use crate::data::schedule::Schedule;
use crate::data::source_info::{ChangeState, SourceInfo};

pub(super) const SCHEDULE_FIXED_HEADERS: &[&str] = &[
    "Uniq ID",
    "Name",
    "Description",
    "Start Time",
    "End Time",
    "Duration",
    "Room",
    "Kind",
    "Cost",
    "Capacity",
    "Difficulty",
    "Note",
    "Prereq",
    "Ticket Sale",
    "Full",
    "Hide Panelist",
    "Alt Panelist",
];

/// Unified flattened session struct used by both export and update paths.
pub(super) struct FlatSession {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) start_time: Option<String>,
    pub(super) end_time: Option<String>,
    pub(super) duration: u32,
    pub(super) room_id: Option<u32>,
    pub(super) panel_type: Option<String>,
    pub(super) cost: Option<String>,
    pub(super) capacity: Option<String>,
    pub(super) difficulty: Option<String>,
    pub(super) note: Option<String>,
    pub(super) prereq: Option<String>,
    pub(super) ticket_url: Option<String>,
    pub(super) is_full: bool,
    pub(super) hide_panelist: bool,
    pub(super) alt_panelist: Option<String>,
    pub(super) all_presenters: Vec<String>,
    pub(super) credited_set: HashSet<String>,
    pub(super) metadata: IndexMap<String, ExtraValue>,
    pub(super) change_state: ChangeState,
    pub(super) source: Option<SourceInfo>,
}

/// Flatten the `panel_sets` into `FlatSession` rows, one per flat [`Panel`].
///
/// When `include_deleted` is `false` (export path) panels with
/// `ChangeState::Deleted` are skipped.  When `true` (update path) they are
/// included so their spreadsheet rows can be marked.
pub(super) fn flatten_panel_sessions(
    schedule: &Schedule,
    include_deleted: bool,
) -> Vec<FlatSession> {
    let mut sessions = Vec::new();

    for ps in schedule.panel_sets.values() {
        for panel in &ps.panels {
            if !include_deleted && panel.change_state == ChangeState::Deleted {
                continue;
            }

            let credited_set: HashSet<String> = panel.credited_presenters.iter().cloned().collect();
            let mut all_set: HashSet<String> = credited_set.clone();
            for p in &panel.uncredited_presenters {
                all_set.insert(p.clone());
            }
            let all_presenters: Vec<String> = all_set.into_iter().collect();

            let room_id = panel.room_ids.first().copied();

            sessions.push(FlatSession {
                id: panel.id.clone(),
                name: panel.name.clone(),
                description: panel.description.clone(),
                start_time: panel.start_time.clone(),
                end_time: panel.end_time.clone(),
                duration: panel.duration,
                room_id,
                panel_type: panel.panel_type.clone(),
                cost: panel.cost.clone(),
                capacity: panel.capacity.clone(),
                difficulty: panel.difficulty.clone(),
                note: panel.note.clone(),
                prereq: panel.prereq.clone(),
                ticket_url: panel.ticket_url.clone(),
                is_full: panel.is_full,
                hide_panelist: panel.hide_panelist,
                alt_panelist: panel.alt_panelist.clone(),
                all_presenters,
                credited_set,
                metadata: panel.metadata.clone(),
                change_state: panel.change_state,
                source: panel.source.clone(),
            });
        }
    }

    sessions
}

pub(super) fn add_table(ws: &mut Worksheet, name: &str, headers: &[&str], last_data_row: u32) {
    let num_cols = headers.len() as u32;
    let last_row = last_data_row.max(2);
    let mut table = Table::new(name, ((1u32, 1u32), (num_cols, last_row)));
    table.set_display_name(name);
    for header in headers {
        table.add_column(TableColumn::new(header));
    }
    let style = TableStyleInfo::new("TableStyleMedium2", false, false, true, false);
    table.set_style_info(Some(style));
    ws.add_table(table);
}

pub(super) fn update_table_areas(worksheet: &mut Worksheet, new_last_row: u32) {
    let last_row = new_last_row.max(2);
    let last_col = worksheet.get_highest_column().max(1);
    for table in worksheet.get_tables_mut() {
        let (start, end) = table.get_area();
        let start_col = *start.get_col_num();
        let start_row = *start.get_row_num();
        let end_col = (*end.get_col_num()).max(last_col);
        table.set_area(((start_col, start_row), (end_col, last_row)));
    }
}
