/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! XLSX export implementation (FEATURE-029).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use chrono::{NaiveDateTime, Timelike};
use umya_spreadsheet::structs::{
    Border, HorizontalAlignmentValues, RichText, TextElement, VerticalAlignmentValues, Worksheet,
};

use crate::entity::{EntityType, EntityUuid};
use crate::schedule::Schedule;
use crate::tables::event_room::{self, EventRoomEntityType};
use crate::tables::hotel_room::HotelRoomEntityType;
use crate::tables::panel::{self, compute_credits, PanelEntityType, PanelInternalData};
use crate::tables::panel_type::PanelTypeEntityType;
use crate::tables::presenter::{
    PresenterEntityType, PresenterId, PresenterInternalData, EDGE_GROUPS,
};
use crate::xlsx::columns::{panel_types, people, room_map, schedule as sched_cols, FieldDef};

use super::common::{add_table, set_headers, set_opt, set_str};

const MIN_PANELS_FOR_NAMED_COLUMN: usize = 3;
const TIME_FMT: &str = "%Y-%m-%dT%H:%M:%S";

// Number of fixed data columns before presenter columns.
const FIXED_COL_COUNT: u32 = sched_cols::ALL.len() as u32;

/// Derive the 1-based column number of `field` within `all`.
///
/// Panics at runtime if the field is not present, which indicates a programmer
/// error (a field was listed in code but omitted from the ALL slice).
fn col_of(all: &[FieldDef], field: &FieldDef) -> u32 {
    all.iter()
        .position(|f| f.canonical == field.canonical)
        .unwrap_or_else(|| panic!("FieldDef '{}' not in column list", field.export)) as u32
        + 1
}

/// Collect all unique extra-field keys across all entities of type `E`,
/// in stable insertion order (first key seen wins ordering).
fn collect_extra_keys<E: EntityType>(schedule: &Schedule) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut keys: Vec<String> = Vec::new();
    for (id, _) in schedule.iter_entities::<E>() {
        for (k, _) in schedule.list_extra_fields(E::TYPE_NAME, id.entity_uuid()) {
            if seen.insert(k.clone()) {
                keys.push(k);
            }
        }
    }
    keys
}

/// Write extra-field values for a single entity row, given the ordered key list
/// and the starting column number for extra fields.
fn write_extra_fields(
    ws: &mut Worksheet,
    row: u32,
    type_name: &str,
    uuid: uuid::NonNilUuid,
    extra_keys: &[String],
    extra_start_col: u32,
    schedule: &Schedule,
) {
    for (i, key) in extra_keys.iter().enumerate() {
        if let Some(val) = schedule.read_extra_field(type_name, uuid, key) {
            if !val.is_empty() {
                ws.get_cell_mut((extra_start_col + i as u32, row))
                    .set_value(val);
            }
        }
    }
}

struct ExportPresenterColumn {
    header: String,
    presenter_id: Option<PresenterId>,
    rank_prefix: char,
    is_other: bool,
}

pub(super) fn export_xlsx(schedule: &Schedule, path: &Path) -> Result<()> {
    let mut book = umya_spreadsheet::new_file();

    let presenter_cols = build_presenter_columns(schedule);

    let panel_extra_keys = collect_extra_keys::<PanelEntityType>(schedule);
    let room_extra_keys = collect_extra_keys::<EventRoomEntityType>(schedule);
    let presenter_extra_keys = collect_extra_keys::<PresenterEntityType>(schedule);
    let panel_type_extra_keys = collect_extra_keys::<PanelTypeEntityType>(schedule);

    // ── Schedule sheet ────────────────────────────────────────────────────────
    {
        let ws = book
            .get_sheet_mut(&0)
            .ok_or_else(|| anyhow::anyhow!("No default sheet in new workbook"))?;
        ws.set_name("Schedule");

        let last_row = write_schedule_sheet(ws, schedule, &presenter_cols, &panel_extra_keys);

        let mut all_headers: Vec<&str> = sched_cols::ALL.iter().map(|f| f.export).collect();
        for col in &presenter_cols {
            all_headers.push(col.header.as_str());
        }
        for k in &panel_extra_keys {
            all_headers.push(k.as_str());
        }
        for fc in sched_cols::FORMULA_COLUMNS {
            all_headers.push(fc.export);
        }

        add_table(ws, "Schedule", &all_headers, last_row);
    }

    // ── Rooms sheet ───────────────────────────────────────────────────────────
    {
        let ws = book
            .new_sheet("Rooms")
            .map_err(|e| anyhow::anyhow!("Cannot create Rooms sheet: {e}"))?;
        let last_row = write_rooms_sheet(ws, schedule, &room_extra_keys);
        let mut headers: Vec<&str> = room_map::ALL.iter().map(|f| f.export).collect();
        for k in &room_extra_keys {
            headers.push(k.as_str());
        }
        add_table(ws, "RoomMap", &headers, last_row);
    }

    // ── People sheet ──────────────────────────────────────────────────────────
    {
        let ws = book
            .new_sheet("People")
            .map_err(|e| anyhow::anyhow!("Cannot create People sheet: {e}"))?;
        let last_row = write_people_sheet(ws, schedule, &presenter_extra_keys);
        let mut headers: Vec<&str> = people::ALL.iter().map(|f| f.export).collect();
        for k in &presenter_extra_keys {
            headers.push(k.as_str());
        }
        add_table(ws, "Presenters", &headers, last_row);
    }

    // ── PanelTypes sheet ──────────────────────────────────────────────────────
    {
        let ws = book
            .new_sheet("PanelTypes")
            .map_err(|e| anyhow::anyhow!("Cannot create PanelTypes sheet: {e}"))?;
        let last_row = write_panel_types_sheet(ws, schedule, &panel_type_extra_keys);
        let mut headers: Vec<&str> = panel_types::ALL.iter().map(|f| f.export).collect();
        for k in &panel_type_extra_keys {
            headers.push(k.as_str());
        }
        add_table(ws, "Prefix", &headers, last_row);
    }

    // ── Grid reference sheets (one per logical day) ─────────────────────────
    {
        let mut used_sheet_names: HashSet<String> = HashSet::new();
        for (day_label, day_panels) in split_panels_by_logical_day(schedule) {
            let base = grid_sheet_name(&day_label);
            let sheet_name = unique_sheet_name(base, &used_sheet_names);
            used_sheet_names.insert(sheet_name.clone());
            let ws = book
                .new_sheet(&sheet_name)
                .map_err(|e| anyhow::anyhow!("Cannot create grid sheet '{sheet_name}': {e}"))?;
            write_grid_sheet(ws, schedule, &day_label, &day_panels);
        }
    }

    umya_spreadsheet::writer::xlsx::write(&book, path)
        .map_err(|e| anyhow::anyhow!("Failed to write XLSX {}: {e}", path.display()))?;

    Ok(())
}

// ── Presenter column layout ───────────────────────────────────────────────────

fn build_presenter_columns(schedule: &Schedule) -> Vec<ExportPresenterColumn> {
    // Count panel appearances (credited + uncredited) per presenter.
    let mut panel_count: std::collections::HashMap<PresenterId, usize> =
        std::collections::HashMap::new();
    for (panel_id, _) in schedule.iter_entities::<PanelEntityType>() {
        for p_id in schedule
            .connected_entities::<PresenterEntityType>(panel_id, panel::EDGE_CREDITED_PRESENTERS)
        {
            *panel_count.entry(p_id).or_insert(0) += 1;
        }
        for p_id in schedule
            .connected_entities::<PresenterEntityType>(panel_id, panel::EDGE_UNCREDITED_PRESENTERS)
        {
            *panel_count.entry(p_id).or_insert(0) += 1;
        }
    }

    let mut columns = Vec::new();

    // Group presenters by rank prefix to create named columns and "Other" columns.
    for std_rank in crate::tables::presenter::PresenterRank::standard_ranks() {
        let prefix_char = std_rank.prefix_char();

        // Collect presenters for this rank tier who appear on at least one panel.
        let mut named: Vec<(PresenterId, &PresenterInternalData)> = Vec::new();
        let mut has_other = false;

        let mut rank_presenters: Vec<(PresenterId, &PresenterInternalData)> = schedule
            .iter_entities::<PresenterEntityType>()
            .filter(|(_, p)| p.data.rank.prefix_char() == prefix_char && !p.data.is_explicit_group)
            .collect();

        // Sort by name for stable column ordering.
        rank_presenters.sort_by(|(_, a), (_, b)| a.data.name.cmp(&b.data.name));

        for (p_id, p) in rank_presenters {
            let count = panel_count.get(&p_id).copied().unwrap_or(0);
            if count == 0 {
                continue;
            }

            // Heuristic: Guests and Staff always get named columns if they have any panels.
            // Other ranks only get named columns if they have >= MIN_PANELS_FOR_NAMED_COLUMN.
            let show_individually = p.data.show_individually;
            let is_guest_or_staff = matches!(
                p.data.rank,
                crate::tables::presenter::PresenterRank::Guest
                    | crate::tables::presenter::PresenterRank::Staff
            );

            if is_guest_or_staff || count >= MIN_PANELS_FOR_NAMED_COLUMN || show_individually {
                named.push((p_id, p));
            } else {
                has_other = true;
            }
        }

        // Sort named presenters by panel count desc, then name asc.
        named.sort_by(|(id_a, a), (id_b, b)| {
            let ca = panel_count.get(id_a).copied().unwrap_or(0);
            let cb = panel_count.get(id_b).copied().unwrap_or(0);
            cb.cmp(&ca).then_with(|| a.data.name.cmp(&b.data.name))
        });

        for (p_id, p) in named {
            // Build header with optional group suffix.
            let group_ids = schedule.connected_entities::<PresenterEntityType>(p_id, EDGE_GROUPS);
            let group_name = group_ids
                .first()
                .and_then(|gid| schedule.get_internal::<PresenterEntityType>(*gid))
                .map(|g| g.data.name.as_str());

            let show_individually = p.data.show_individually;

            // Check if group has subsumes_members flag
            let group_subsumes = group_ids
                .first()
                .and_then(|gid| {
                    schedule
                        .get_internal::<PresenterEntityType>(*gid)
                        .map(|g| g.data.subsumes_members)
                })
                .unwrap_or(false);

            let header = match (group_name, show_individually, group_subsumes) {
                (Some(group), true, _) => {
                    // Member has show_individually → output <Name syntax
                    format!("{prefix_char}:<{}={}", p.data.name, group)
                }
                (Some(group), false, true) => {
                    // Group has subsumes_members → output ==Group syntax
                    format!("{prefix_char}:{}=={}", p.data.name, group)
                }
                (Some(group), false, false) => {
                    format!("{prefix_char}:{}={}", p.data.name, group)
                }
                (None, _, _) => format!("{prefix_char}:{}", p.data.name),
            };

            columns.push(ExportPresenterColumn {
                header,
                presenter_id: Some(p_id),
                rank_prefix: prefix_char,
                is_other: false,
            });
        }

        if has_other {
            columns.push(ExportPresenterColumn {
                header: format!("{prefix_char}:Other"),
                presenter_id: None,
                rank_prefix: prefix_char,
                is_other: true,
            });
        }
    }

    columns
}

// ── Sheet writers ─────────────────────────────────────────────────────────────

fn write_schedule_sheet(
    ws: &mut Worksheet,
    schedule: &Schedule,
    presenter_cols: &[ExportPresenterColumn],
    extra_keys: &[String],
) -> u32 {
    // Write headers.
    let fixed_headers: Vec<&str> = sched_cols::ALL.iter().map(|f| f.export).collect();
    set_headers(ws, &fixed_headers);
    for (i, col) in presenter_cols.iter().enumerate() {
        let c = FIXED_COL_COUNT + i as u32 + 1;
        ws.get_cell_mut((c, 1)).set_value(col.header.as_str());
    }
    // Extra data-field headers after presenter columns.
    let extra_start_col = FIXED_COL_COUNT + presenter_cols.len() as u32 + 1;
    for (i, k) in extra_keys.iter().enumerate() {
        ws.get_cell_mut((extra_start_col + i as u32, 1))
            .set_value(k.as_str());
    }
    // Formula column headers last.
    let formula_start_col = extra_start_col + extra_keys.len() as u32;
    for (i, fc) in sched_cols::FORMULA_COLUMNS.iter().enumerate() {
        ws.get_cell_mut((formula_start_col + i as u32, 1))
            .set_value(fc.export);
    }
    let lstart_col = formula_start_col;
    let lend_col = formula_start_col + 1;

    // Build presenter ID lookup: named presenter_id → column index.
    // Also track which columns are "Other" columns.
    let named_col_lookup: std::collections::HashMap<PresenterId, u32> = presenter_cols
        .iter()
        .enumerate()
        .filter_map(|(i, col)| {
            col.presenter_id
                .filter(|_| !col.is_other)
                .map(|pid| (pid, FIXED_COL_COUNT + i as u32 + 1))
        })
        .collect();

    let named_ids: HashSet<PresenterId> = named_col_lookup.keys().copied().collect();

    // Sort panels by start_time (None last), then by code.
    let mut panels: Vec<(_, &PanelInternalData)> =
        schedule.iter_entities::<PanelEntityType>().collect();
    panels.sort_by(|(_, a), (_, b)| {
        let at = a.time_slot.start_time();
        let bt = b.time_slot.start_time();
        match (at, bt) {
            (Some(ta), Some(tb)) => ta
                .cmp(&tb)
                .then_with(|| a.code.full_id().cmp(&b.code.full_id())),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.code.full_id().cmp(&b.code.full_id()),
        }
    });

    // Pre-compute column numbers from schedule::ALL — avoids coupling data
    // writes to the literal position of each field in the array.
    let c = |f: &FieldDef| col_of(sched_cols::ALL, f);
    let c_uniq_id = c(&sched_cols::UNIQ_ID);
    let c_name = c(&sched_cols::NAME);
    let c_room = c(&sched_cols::ROOM);
    let c_start_time = c(&sched_cols::START_TIME);
    let c_duration = c(&sched_cols::DURATION);
    let c_end_time = c(&sched_cols::END_TIME);
    let c_description = c(&sched_cols::DESCRIPTION);
    let c_prereq = c(&sched_cols::PREREQ);
    let c_note = c(&sched_cols::NOTE);
    let c_notes_np = c(&sched_cols::NOTES_NON_PRINTING);
    let c_workshop_notes = c(&sched_cols::WORKSHOP_NOTES);
    let c_power_needs = c(&sched_cols::POWER_NEEDS);
    let c_sewing_machines = c(&sched_cols::SEWING_MACHINES);
    let c_av_notes = c(&sched_cols::AV_NOTES);
    let c_difficulty = c(&sched_cols::DIFFICULTY);
    let c_cost = c(&sched_cols::COST);
    let c_pre_reg_max = c(&sched_cols::PRE_REG_MAX);
    let c_capacity = c(&sched_cols::CAPACITY);
    let c_ticket_sale = c(&sched_cols::TICKET_SALE);
    let c_ticket_url = c(&sched_cols::TICKET_URL);
    let c_hide_panelist = c(&sched_cols::HIDE_PANELIST);
    let c_alt_panelist = c(&sched_cols::ALT_PANELIST);
    let c_kind = c(&sched_cols::KIND);
    let c_full = c(&sched_cols::FULL);

    let mut row = 2u32;
    for (panel_id, panel) in &panels {
        // ── Fixed data columns ────────────────────────────────────────────────
        set_str(ws, c_uniq_id, row, &panel.code.full_id());
        // OLD_UNIQ_ID: no old_code field in this data model; leave blank.
        set_str(ws, c_name, row, &panel.data.name);

        // Room: comma-join of connected event room names.
        let room_names: Vec<String> = schedule
            .connected_entities::<EventRoomEntityType>(*panel_id, panel::EDGE_EVENT_ROOMS)
            .into_iter()
            .filter_map(|rid| schedule.get_internal::<EventRoomEntityType>(rid))
            .map(|r| r.data.room_name.clone())
            .collect();
        if !room_names.is_empty() {
            set_str(ws, c_room, row, &room_names.join(", "));
        }

        // Timing.
        if let Some(start) = panel.time_slot.start_time() {
            set_str(ws, c_start_time, row, &start.format(TIME_FMT).to_string());
        }
        if let Some(dur) = panel.time_slot.duration() {
            set_str(ws, c_duration, row, &dur.num_minutes().to_string());
        }
        if let Some(end) = panel.time_slot.end_time() {
            set_str(ws, c_end_time, row, &end.format(TIME_FMT).to_string());
        }

        // Text fields.
        set_opt(ws, c_description, row, &panel.data.description);
        set_opt(ws, c_prereq, row, &panel.data.prereq);
        set_opt(ws, c_note, row, &panel.data.note);
        set_opt(ws, c_notes_np, row, &panel.data.notes_non_printing);
        set_opt(ws, c_workshop_notes, row, &panel.data.workshop_notes);
        set_opt(ws, c_power_needs, row, &panel.data.power_needs);
        if panel.data.sewing_machines {
            set_str(ws, c_sewing_machines, row, "Yes");
        }
        set_opt(ws, c_av_notes, row, &panel.data.av_notes);
        set_opt(ws, c_difficulty, row, &panel.data.difficulty);

        // Cost: synthesize display value from typed fields.
        // Included always writes "$0" so a blank cell on re-import is not
        // misread as TBD for workshop panels.
        let cost_str = if panel.data.for_kids {
            Some("Kids".to_string())
        } else if matches!(
            panel.data.additional_cost,
            crate::value::AdditionalCost::Included
        ) {
            Some("$0".to_string())
        } else {
            crate::value::cost::additional_cost_to_string(&panel.data.additional_cost)
        };
        set_opt(ws, c_cost, row, &cost_str);

        // Seat counts.
        let pre_reg_max = panel.data.pre_reg_max.map(|n| n.to_string());
        let capacity = panel.data.capacity.map(|n| n.to_string());
        set_opt(ws, c_pre_reg_max, row, &pre_reg_max);
        set_opt(ws, c_capacity, row, &capacity);

        // Ticketing.
        set_opt(ws, c_ticket_sale, row, &panel.data.ticket_url);
        set_opt(ws, c_ticket_url, row, &panel.data.ticket_url);

        // Presenter display overrides.
        if panel.data.hide_panelist {
            set_str(ws, c_hide_panelist, row, "Yes");
        }
        set_opt(ws, c_alt_panelist, row, &panel.data.alt_panelist);

        // Kind: panel type prefix (two-letter code).
        let kind_prefix = schedule
            .connected_entities::<PanelTypeEntityType>(*panel_id, panel::EDGE_PANEL_TYPE)
            .into_iter()
            .next()
            .and_then(|pt_id| schedule.get_internal::<PanelTypeEntityType>(pt_id))
            .map(|pt| pt.data.prefix.clone());
        set_opt(ws, c_kind, row, &kind_prefix);

        if panel.data.is_full {
            set_str(ws, c_full, row, "Yes");
        }

        // ── Presenter columns ─────────────────────────────────────────────────
        let credited_ids: HashSet<PresenterId> = schedule
            .connected_entities::<PresenterEntityType>(*panel_id, panel::EDGE_CREDITED_PRESENTERS)
            .into_iter()
            .collect();
        let uncredited_ids: HashSet<PresenterId> = schedule
            .connected_entities::<PresenterEntityType>(*panel_id, panel::EDGE_UNCREDITED_PRESENTERS)
            .into_iter()
            .collect();

        // Named presenter columns.
        for (&p_id, &col) in &named_col_lookup {
            if credited_ids.contains(&p_id) {
                set_str(ws, col, row, "Yes");
            } else if uncredited_ids.contains(&p_id) {
                set_str(ws, col, row, "*");
            }
        }

        // "Other" columns: unnamed presenters grouped by rank.
        let all_ids: HashSet<PresenterId> = credited_ids
            .iter()
            .chain(uncredited_ids.iter())
            .copied()
            .collect();

        for (i, col) in presenter_cols.iter().enumerate() {
            if !col.is_other {
                continue;
            }
            let col_num = FIXED_COL_COUNT + i as u32 + 1;
            let others: Vec<String> = all_ids
                .iter()
                .filter(|&&pid| {
                    if named_ids.contains(&pid) {
                        return false;
                    }
                    schedule
                        .get_internal::<PresenterEntityType>(pid)
                        .map(|p| p.data.rank.prefix_char() == col.rank_prefix)
                        .unwrap_or(false)
                })
                .map(|&pid| {
                    let name = schedule
                        .get_internal::<PresenterEntityType>(pid)
                        .map(|p| p.data.name.as_str())
                        .unwrap_or("?");
                    if credited_ids.contains(&pid) {
                        name.to_string()
                    } else {
                        format!("*{name}")
                    }
                })
                .collect();
            if !others.is_empty() {
                let mut sorted = others;
                sorted.sort();
                set_str(ws, col_num, row, &sorted.join(", "));
            }
        }

        // ── Lstart / Lend formula columns (driven by FORMULA_COLUMNS) ────────
        if let Some(lstart_fc) = sched_cols::FORMULA_COLUMNS.first() {
            if let Some(formula) = lstart_fc.regenerate {
                let cell = ws.get_cell_mut((lstart_col, row));
                cell.set_formula(formula);
                if let Some(start) = panel.time_slot.start_time() {
                    cell.set_value(start.format(TIME_FMT).to_string());
                } else {
                    cell.set_value("");
                }
            }
        }
        if let Some(lend_fc) = sched_cols::FORMULA_COLUMNS.get(1) {
            if let Some(formula) = lend_fc.regenerate {
                let cell = ws.get_cell_mut((lend_col, row));
                cell.set_formula(formula);
                if let Some(end) = panel.time_slot.end_time() {
                    cell.set_value(end.format(TIME_FMT).to_string());
                } else {
                    cell.set_value("");
                }
            }
        }

        write_extra_fields(
            ws,
            row,
            PanelEntityType::TYPE_NAME,
            panel_id.entity_uuid(),
            extra_keys,
            extra_start_col,
            schedule,
        );

        row += 1;
    }

    row - 1
}

fn write_rooms_sheet(ws: &mut Worksheet, schedule: &Schedule, extra_keys: &[String]) -> u32 {
    let mut headers: Vec<&str> = room_map::ALL.iter().map(|f| f.export).collect();
    for k in extra_keys {
        headers.push(k.as_str());
    }
    set_headers(ws, &headers);

    // Sort rooms by sort_key (None last), then room_name.
    let mut rooms: Vec<_> = schedule.iter_entities::<EventRoomEntityType>().collect();
    rooms.sort_by(|(_, a), (_, b)| match (a.data.sort_key, b.data.sort_key) {
        (Some(ka), Some(kb)) => ka
            .cmp(&kb)
            .then_with(|| a.data.room_name.cmp(&b.data.room_name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.data.room_name.cmp(&b.data.room_name),
    });

    let c = |f: &FieldDef| col_of(room_map::ALL, f);
    let c_room_name = c(&room_map::ROOM_NAME);
    let c_sort_key = c(&room_map::SORT_KEY);
    let c_long_name = c(&room_map::LONG_NAME);
    let c_hotel_room = c(&room_map::HOTEL_ROOM);
    let extra_start_col = room_map::ALL.len() as u32 + 1;

    let mut row = 2u32;
    for (room_id, room) in &rooms {
        set_str(ws, c_room_name, row, &room.data.room_name);
        let sk = room.data.sort_key.map(|n| n.to_string());
        set_opt(ws, c_sort_key, row, &sk);
        set_opt(ws, c_long_name, row, &room.data.long_name);

        // Hotel room: first connected hotel room's name.
        let hotel_name = schedule
            .connected_entities::<HotelRoomEntityType>(*room_id, event_room::EDGE_HOTEL_ROOMS)
            .into_iter()
            .next()
            .and_then(|hr_id| schedule.get_internal::<HotelRoomEntityType>(hr_id))
            .map(|hr| hr.data.hotel_room_name.clone());
        set_opt(ws, c_hotel_room, row, &hotel_name);

        write_extra_fields(
            ws,
            row,
            EventRoomEntityType::TYPE_NAME,
            room_id.entity_uuid(),
            extra_keys,
            extra_start_col,
            schedule,
        );

        row += 1;
    }
    row - 1
}

fn write_people_sheet(ws: &mut Worksheet, schedule: &Schedule, extra_keys: &[String]) -> u32 {
    let mut headers: Vec<&str> = people::ALL.iter().map(|f| f.export).collect();
    for k in extra_keys {
        headers.push(k.as_str());
    }
    set_headers(ws, &headers);

    // Sort by rank priority (ascending), then name.
    let mut presenters: Vec<_> = schedule.iter_entities::<PresenterEntityType>().collect();
    presenters.sort_by(|(_, a), (_, b)| {
        a.data
            .rank
            .priority()
            .cmp(&b.data.rank.priority())
            .then_with(|| a.data.name.cmp(&b.data.name))
    });

    let c = |f: &FieldDef| col_of(people::ALL, f);
    let c_person = c(&people::NAME);
    let c_classification = c(&people::CLASSIFICATION);
    let c_is_group = c(&people::IS_GROUP);
    let c_always_grouped = c(&people::ALWAYS_GROUPED);
    let c_always_shown = c(&people::ALWAYS_SHOWN);
    let extra_start_col = people::ALL.len() as u32 + 1;

    let mut row = 2u32;
    for (presenter_id, presenter) in &presenters {
        set_str(ws, c_person, row, &presenter.data.name);
        set_str(ws, c_classification, row, presenter.data.rank.as_str());
        if presenter.data.is_explicit_group {
            set_str(ws, c_is_group, row, "Yes");
        }
        if presenter.data.show_individually {
            set_str(ws, c_always_grouped, row, "Yes");
        }
        if presenter.data.subsumes_members {
            set_str(ws, c_always_shown, row, "Yes");
        }
        write_extra_fields(
            ws,
            row,
            PresenterEntityType::TYPE_NAME,
            presenter_id.entity_uuid(),
            extra_keys,
            extra_start_col,
            schedule,
        );
        row += 1;
    }
    row - 1
}

fn write_panel_types_sheet(ws: &mut Worksheet, schedule: &Schedule, extra_keys: &[String]) -> u32 {
    let mut headers: Vec<&str> = panel_types::ALL.iter().map(|f| f.export).collect();
    for k in extra_keys {
        headers.push(k.as_str());
    }
    set_headers(ws, &headers);

    // Sort by prefix.
    let mut types: Vec<_> = schedule.iter_entities::<PanelTypeEntityType>().collect();
    types.sort_by(|(_, a), (_, b)| a.data.prefix.cmp(&b.data.prefix));

    let c = |f: &FieldDef| col_of(panel_types::ALL, f);
    let c_prefix = c(&panel_types::PREFIX);
    let c_panel_kind = c(&panel_types::PANEL_KIND);
    let c_color = c(&panel_types::COLOR);
    let c_bw = c(&panel_types::BW_COLOR);
    let c_hidden = c(&panel_types::HIDDEN);
    let c_is_timeline = c(&panel_types::IS_TIMELINE);
    let c_is_private = c(&panel_types::IS_PRIVATE);
    let c_is_break = c(&panel_types::IS_BREAK);
    let c_is_workshop = c(&panel_types::IS_WORKSHOP);
    let c_is_rh = c(&panel_types::IS_ROOM_HOURS);
    let c_is_cafe = c(&panel_types::IS_CAFE);
    let extra_start_col = panel_types::ALL.len() as u32 + 1;

    let mut row = 2u32;
    for (pt_id, pt) in &types {
        set_str(ws, c_prefix, row, &pt.data.prefix);
        set_str(ws, c_panel_kind, row, &pt.data.panel_kind);
        set_opt(ws, c_color, row, &pt.data.color);
        set_opt(ws, c_bw, row, &pt.data.bw);
        if pt.data.hidden {
            set_str(ws, c_hidden, row, "Yes");
        }
        if pt.data.is_timeline {
            set_str(ws, c_is_timeline, row, "Yes");
        }
        if pt.data.is_private {
            set_str(ws, c_is_private, row, "Yes");
        }
        if pt.data.is_break {
            set_str(ws, c_is_break, row, "Yes");
        }
        if pt.data.is_workshop {
            set_str(ws, c_is_workshop, row, "Yes");
        }
        if pt.data.is_room_hours {
            set_str(ws, c_is_rh, row, "Yes");
        }
        if pt.data.is_cafe {
            set_str(ws, c_is_cafe, row, "Yes");
        }
        write_extra_fields(
            ws,
            row,
            PanelTypeEntityType::TYPE_NAME,
            pt_id.entity_uuid(),
            extra_keys,
            extra_start_col,
            schedule,
        );
        row += 1;
    }
    row - 1
}

// ── Grid reference sheet helpers ──────────────────────────────────────────────

/// Midnight-safe overnight-break threshold: gap > 4 hours or date line cross.
const OVERNIGHT_GAP_MINUTES: i64 = 240;

/// Internal panel summary used for grid layout, carrying only what we need.
struct GridPanel {
    name: String,
    credits: Vec<String>,
    start: NaiveDateTime,
    end: NaiveDateTime,
    room_ids: Vec<crate::tables::event_room::EventRoomId>,
    is_break: bool,
    /// CSS hex color from the panel type (e.g. `"#db2777"`), used as left-border accent.
    type_color: Option<String>,
}

/// Split all scheduled panels in the schedule into logical days.
///
/// A new logical day starts when **both** of these are true:
/// 1. The gap between the last panel end and the next panel start exceeds
///    `OVERNIGHT_GAP_MINUTES` (4 hours).
/// 2. The next panel starts on a different calendar date than the previous end.
///
/// This means a short gap crossing midnight (e.g. 2 h) keeps panels in the
/// same logical day, and a long gap within a single calendar date (e.g. setup
/// vs. programming on the same day) also does not split.
///
/// The day label is taken from the calendar date of the first panel in the
/// group, so a Thursday schedule that runs into Friday early AM is labeled
/// "Thu".
fn split_panels_by_logical_day(schedule: &Schedule) -> Vec<(String, Vec<GridPanel>)> {
    let mut all: Vec<GridPanel> = collect_grid_panels(schedule);
    all.sort_by_key(|a| a.start);

    let mut days: Vec<(String, Vec<GridPanel>)> = Vec::new();
    let mut current_max_end: Option<NaiveDateTime> = None;
    // Track the calendar date that opened the current logical day so that
    // a panel whose start is on a *different* calendar date (with a large
    // enough gap) correctly opens a new day even when `current_max_end`
    // has already advanced into that next date.
    let mut current_day_date: Option<chrono::NaiveDate> = None;

    for gp in all {
        let is_new_day = match (current_max_end, current_day_date) {
            (None, _) => true,
            (Some(prev_end), Some(day_date)) => {
                let gap = (gp.start - prev_end).num_minutes();
                gap > OVERNIGHT_GAP_MINUTES && gp.start.date() != day_date
            }
            _ => false,
        };

        if is_new_day {
            let label = format_day_label(gp.start);
            current_day_date = Some(gp.start.date());
            days.push((label, Vec::new()));
        }

        let end = gp.end;
        days.last_mut().unwrap().1.push(gp);
        current_max_end = Some(match current_max_end {
            Some(prev) if end > prev => end,
            Some(prev) => prev,
            None => end,
        });
    }

    days
}

/// Collect all panels (regular + break) that have both start and end times.
fn collect_grid_panels(schedule: &Schedule) -> Vec<GridPanel> {
    let mut out = Vec::new();

    for (panel_id, internal) in schedule.iter_entities::<PanelEntityType>() {
        let (start, end) = match (
            internal.time_slot.start_time(),
            internal.time_slot.end_time(),
        ) {
            (Some(s), Some(e)) => (s, e),
            _ => continue,
        };

        let panel_type_data = schedule
            .connected_entities::<PanelTypeEntityType>(panel_id, panel::EDGE_PANEL_TYPE)
            .into_iter()
            .next()
            .and_then(|pt_id| schedule.get_internal::<PanelTypeEntityType>(pt_id))
            .map(|pt| (pt.data.is_break, pt.data.is_timeline, pt.data.color.clone()));

        let is_break = panel_type_data.as_ref().map(|d| d.0).unwrap_or(false);
        let is_timeline = panel_type_data.as_ref().map(|d| d.1).unwrap_or(false);
        let type_color = panel_type_data.and_then(|d| d.2);

        // Timeline panels are scheduling artifacts; skip them in the grid.
        if is_timeline {
            continue;
        }

        let room_ids: Vec<_> = schedule
            .connected_entities::<EventRoomEntityType>(panel_id, panel::EDGE_EVENT_ROOMS)
            .into_iter()
            .collect();

        let credits = if !is_break {
            compute_credits(schedule, panel_id)
        } else {
            Vec::new()
        };

        out.push(GridPanel {
            name: internal.data.name.clone(),
            credits,
            start,
            end,
            room_ids,
            is_break,
            type_color,
        });
    }

    out
}

/// Human-readable day label for a grid sheet title, e.g. `"Fri Jun 27"`.
fn format_day_label(dt: NaiveDateTime) -> String {
    dt.format("%a %b %-d").to_string()
}

/// Sheet name for a grid day, e.g. `"Grid - Fri Jun 27"` (capped at 31 chars).
fn grid_sheet_name(day_label: &str) -> String {
    let full = format!("Grid - {day_label}");
    if full.len() <= 31 {
        full
    } else {
        full[..31].to_string()
    }
}

/// Return `base` if it is not in `used`; otherwise append " 2", " 3", … until unique.
///
/// The returned name is always ≤ 31 characters.
fn unique_sheet_name(base: String, used: &HashSet<String>) -> String {
    if !used.contains(&base) {
        return base;
    }
    for n in 2u32.. {
        let candidate = {
            let suffix = format!(" {n}");
            if base.len() + suffix.len() <= 31 {
                format!("{base}{suffix}")
            } else {
                format!("{}{suffix}", &base[..31 - suffix.len()])
            }
        };
        if !used.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

/// Convert a 1-based column number to an Excel column letter string (A, B, …, Z, AA, …).
fn col_letter(col: u32) -> String {
    let mut n = col;
    let mut result = String::new();
    while n > 0 {
        n -= 1;
        result.insert(0, char::from_u32(b'A' as u32 + n % 26).unwrap_or('A'));
        n /= 26;
    }
    result
}

/// Format a human-readable time label for a `NaiveDateTime`.
fn grid_time_label(dt: NaiveDateTime) -> String {
    let hour = dt.hour();
    let min = dt.minute();
    let (h12, suffix) = if hour == 0 {
        (12u32, "AM")
    } else if hour < 12 {
        (hour, "AM")
    } else if hour == 12 {
        (12, "PM")
    } else {
        (hour - 12, "PM")
    };
    if min == 0 {
        format!("{h12} {suffix}")
    } else {
        format!("{h12}:{min:02}")
    }
}

// ── Grid sheet color constants ────────────────────────────────────────────────

const COLOR_HEADER_BG: &str = "FF2B6CB0"; // dark blue header
const COLOR_HEADER_FG: &str = "FFFFFFFF"; // white text
const COLOR_TITLE_BG: &str = "FF1A365D"; // darker blue title row
const COLOR_TIME_BG: &str = "FFE2E8F0"; // light grey time column
const COLOR_TIME_FG: &str = "FF2D3748"; // dark grey time text
const COLOR_EVENT_BG: &str = "FFFFFFFF"; // white event cells
const COLOR_EVENT_FG: &str = "FF1A202C"; // near-black event text
const COLOR_BREAK_BG: &str = "FFF7FAFC"; // near-white break rows
const COLOR_EMPTY_BG: &str = "FFCCCCCC"; // grey empty cells
const COLOR_BREAK_FG: &str = "FF718096"; // muted grey break text
const COLOR_BORDER: &str = "FFB2C5D4"; // light blue-grey border

/// Convert a CSS hex color (`"#rrggbb"` or `"#rgb"`) to an ARGB string (`"FFrrggbb"`).
///
/// Returns a fully-opaque grey fallback if the input cannot be parsed.
fn css_hex_to_argb(css: &str) -> String {
    let hex = css.trim_start_matches('#');
    match hex.len() {
        6 => format!("FF{}", hex.to_uppercase()),
        3 => {
            let r = &hex[0..1];
            let g = &hex[1..2];
            let b = &hex[2..3];
            format!("FF{r}{r}{g}{g}{b}{b}").to_uppercase()
        }
        _ => "FF888888".to_string(),
    }
}

/// Apply a thin border on all four sides of a cell style.
fn apply_thin_border(style: &mut umya_spreadsheet::structs::Style) {
    let b = style.get_borders_mut();
    b.get_bottom_mut().set_border_style(Border::BORDER_THIN);
    b.get_bottom_mut().get_color_mut().set_argb(COLOR_BORDER);
    b.get_top_mut().set_border_style(Border::BORDER_THIN);
    b.get_top_mut().get_color_mut().set_argb(COLOR_BORDER);
    b.get_left_mut().set_border_style(Border::BORDER_THIN);
    b.get_left_mut().get_color_mut().set_argb(COLOR_BORDER);
    b.get_right_mut().set_border_style(Border::BORDER_THIN);
    b.get_right_mut().get_color_mut().set_argb(COLOR_BORDER);
}

/// Write a single grid reference sheet for one logical day.
///
/// Layout:
/// - Row 1: merged title spanning all columns.
/// - Row 2: header row — "Time" + one column per room.
/// - Row 3+: one row per unique time-slot boundary; panel cells use merged ranges.
fn write_grid_sheet(
    ws: &mut Worksheet,
    schedule: &Schedule,
    day_label: &str,
    panels: &[GridPanel],
) {
    if panels.is_empty() {
        return;
    }

    // ── Build sorted room list ────────────────────────────────────────────────
    let room_ids_used: HashSet<crate::tables::event_room::EventRoomId> = panels
        .iter()
        .filter(|p| !p.is_break)
        .flat_map(|p| p.room_ids.iter().copied())
        .collect();

    let mut rooms: Vec<_> = schedule
        .iter_entities::<EventRoomEntityType>()
        .filter(|(rid, r)| room_ids_used.contains(rid) && !r.data.is_pseudo)
        .collect();
    rooms.sort_by(|(_, a), (_, b)| match (a.data.sort_key, b.data.sort_key) {
        (Some(ka), Some(kb)) => ka
            .cmp(&kb)
            .then_with(|| a.data.room_name.cmp(&b.data.room_name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.data.room_name.cmp(&b.data.room_name),
    });

    let num_rooms = rooms.len() as u32;
    let total_cols = 1 + num_rooms; // col 1 = Time, cols 2.. = rooms

    // ── Collect unique time-slot boundaries ───────────────────────────────────
    let mut time_keys: Vec<NaiveDateTime> = panels
        .iter()
        .flat_map(|p| [p.start, p.end])
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    time_keys.sort();

    let time_index: HashMap<NaiveDateTime, usize> = time_keys
        .iter()
        .enumerate()
        .map(|(i, dt)| (*dt, i))
        .collect();

    let room_col_index: HashMap<crate::tables::event_room::EventRoomId, usize> = rooms
        .iter()
        .enumerate()
        .map(|(i, (rid, _))| (*rid, i))
        .collect();

    const DATA_ROW_OFFSET: u32 = 3; // rows 1=title, 2=header, 3+=data

    // ── Column widths ─────────────────────────────────────────────────────────
    ws.get_column_dimension_mut("A").set_width(10.0);
    for i in 0..num_rooms {
        ws.get_column_dimension_mut(&col_letter(2 + i))
            .set_width(22.0);
    }

    // ── Row 1: title ─────────────────────────────────────────────────────────
    ws.get_cell_mut((1u32, 1u32)).set_value(day_label);
    if total_cols > 1 {
        ws.add_merge_cells(format!("A1:{}1", col_letter(total_cols)));
    }
    ws.get_row_dimension_mut(&1).set_height(24.0);
    {
        let s = ws.get_style_mut("A1");
        s.set_background_color_solid(COLOR_TITLE_BG);
        s.get_font_mut().set_bold(true);
        s.get_font_mut().set_size(13.0);
        s.get_font_mut().get_color_mut().set_argb(COLOR_HEADER_FG);
        s.get_alignment_mut().set_wrap_text(true);
        s.get_alignment_mut()
            .set_vertical(VerticalAlignmentValues::Center);
    }

    // ── Row 2: header row ─────────────────────────────────────────────────────
    ws.get_row_dimension_mut(&2).set_height(36.0);
    ws.get_cell_mut((1u32, 2u32)).set_value("Time");
    for (i, (rid, room)) in rooms.iter().enumerate() {
        let col = 2 + i as u32;
        // Show hotel room name in parens if present, otherwise nothing.
        let hotel_name: Option<String> = schedule
            .connected_entities::<HotelRoomEntityType>(*rid, event_room::EDGE_HOTEL_ROOMS)
            .into_iter()
            .next()
            .and_then(|hr_id| schedule.get_internal::<HotelRoomEntityType>(hr_id))
            .map(|hr| hr.data.hotel_room_name.clone());
        let display = match (&room.data.long_name, &hotel_name) {
            (Some(long), Some(hotel)) => format!("{long}\n({hotel})"),
            (Some(long), None) => long.clone(),
            (None, Some(hotel)) => format!("{}\n({hotel})", room.data.room_name),
            (None, None) => room.data.room_name.clone(),
        };
        ws.get_cell_mut((col, 2u32)).set_value(display.as_str());
    }
    // Apply header style to all header cells.
    for col in 1..=total_cols {
        let addr = format!("{}{}", col_letter(col), 2);
        let s = ws.get_style_mut(addr.as_str());
        s.set_background_color_solid(COLOR_HEADER_BG);
        s.get_font_mut().set_bold(true);
        s.get_font_mut().set_size(10.0);
        s.get_font_mut().get_color_mut().set_argb(COLOR_HEADER_FG);
        s.get_alignment_mut().set_wrap_text(true);
        s.get_alignment_mut()
            .set_vertical(VerticalAlignmentValues::Center);
        s.get_alignment_mut()
            .set_horizontal(HorizontalAlignmentValues::Center);
        apply_thin_border(s);
    }

    // ── Row 3+: time-slot label column ────────────────────────────────────────
    for (ti, dt) in time_keys.iter().enumerate() {
        let row = ti as u32 + DATA_ROW_OFFSET;
        ws.get_cell_mut((1u32, row))
            .set_value(grid_time_label(*dt).as_str());
        ws.get_row_dimension_mut(&row).set_height(45.0);

        // Style the time cell.
        let addr = format!("A{row}");
        let s = ws.get_style_mut(addr.as_str());
        s.set_background_color_solid(COLOR_TIME_BG);
        s.get_font_mut().set_bold(true);
        s.get_font_mut().set_size(9.0);
        s.get_font_mut().get_color_mut().set_argb(COLOR_TIME_FG);
        s.get_alignment_mut().set_wrap_text(true);
        s.get_alignment_mut()
            .set_vertical(VerticalAlignmentValues::Top);
        apply_thin_border(s);

        // Style empty room cells with grey background and a border.
        for col in 2..=total_cols {
            let empty_addr = format!("{}{row}", col_letter(col));
            let es = ws.get_style_mut(empty_addr.as_str());
            es.set_background_color_solid(COLOR_EMPTY_BG);
            apply_thin_border(es);
        }
    }

    // ── Place panel cells ─────────────────────────────────────────────────────
    let mut covered: HashSet<(u32, u32)> = HashSet::new();

    for panel in panels {
        let row_start_idx = match time_index.get(&panel.start) {
            Some(&i) => i,
            None => continue,
        };
        let row_end_idx = match time_index.get(&panel.end) {
            Some(&i) => i,
            None => continue,
        };
        if row_end_idx <= row_start_idx {
            continue;
        }

        let xl_row_start = row_start_idx as u32 + DATA_ROW_OFFSET;
        let xl_row_end = row_end_idx as u32 + DATA_ROW_OFFSET - 1; // inclusive

        // Duration string: "X hr" / "X hr Y min" / "Y min"
        let duration_mins = (panel.end - panel.start).num_minutes();
        let duration_str = match (duration_mins / 60, duration_mins % 60) {
            (h, 0) if h > 0 => format!("{h} hr"),
            (0, m) => format!("{m} min"),
            (h, m) => format!("{h} hr {m} min"),
        };

        let credits_str = panel.credits.join(", ");
        let break_text = if credits_str.is_empty() {
            format!("{}  {duration_str}", panel.name)
        } else {
            format!("{}\n{credits_str}\n{duration_str}", panel.name)
        };

        if panel.is_break {
            if !covered.contains(&(xl_row_start, 1)) {
                ws.get_cell_mut((1u32, xl_row_start))
                    .set_value(break_text.as_str());
                let top_left = format!("A{xl_row_start}");
                let bottom_right = format!("{}{}", col_letter(total_cols), xl_row_end);
                if top_left != bottom_right {
                    ws.add_merge_cells(format!("{top_left}:{bottom_right}"));
                }
                // Style the break cell.
                let s = ws.get_style_mut(top_left.as_str());
                s.set_background_color_solid(COLOR_BREAK_BG);
                s.get_font_mut().set_italic(true);
                s.get_font_mut().set_size(9.0);
                s.get_font_mut().get_color_mut().set_argb(COLOR_BREAK_FG);
                s.get_alignment_mut().set_wrap_text(true);
                s.get_alignment_mut()
                    .set_vertical(VerticalAlignmentValues::Center);
                s.get_alignment_mut()
                    .set_horizontal(HorizontalAlignmentValues::Center);
                apply_thin_border(s);
                for r in xl_row_start..=xl_row_end {
                    for c in 1..=total_cols {
                        covered.insert((r, c));
                    }
                }
            }
        } else {
            for &room_id in &panel.room_ids {
                if let Some(&room_ci) = room_col_index.get(&room_id) {
                    let xl_col = 2 + room_ci as u32;
                    if covered.contains(&(xl_row_start, xl_col)) {
                        continue;
                    }
                    // Build rich text: bold name, then italic credits/duration.
                    let mut rt = RichText::default();
                    let mut name_el = TextElement::default();
                    name_el.set_text(panel.name.clone());
                    name_el.get_font_mut().set_bold(true);
                    name_el.get_font_mut().set_size(13.0);
                    name_el
                        .get_font_mut()
                        .get_color_mut()
                        .set_argb(COLOR_EVENT_FG);
                    rt.add_rich_text_elements(name_el);
                    let meta_text = if credits_str.is_empty() {
                        format!("\n{duration_str}")
                    } else {
                        format!("\n{credits_str}\n{duration_str}")
                    };
                    let mut meta_el = TextElement::default();
                    meta_el.set_text(meta_text);
                    meta_el.get_font_mut().set_italic(true);
                    meta_el.get_font_mut().set_size(10.0);
                    meta_el
                        .get_font_mut()
                        .get_color_mut()
                        .set_argb(COLOR_EVENT_FG);
                    rt.add_rich_text_elements(meta_el);
                    ws.get_cell_mut((xl_col, xl_row_start)).set_rich_text(rt);
                    if xl_row_end > xl_row_start {
                        ws.add_merge_cells(format!(
                            "{col}{r1}:{col}{r2}",
                            col = col_letter(xl_col),
                            r1 = xl_row_start,
                            r2 = xl_row_end,
                        ));
                        for r in xl_row_start..=xl_row_end {
                            covered.insert((r, xl_col));
                        }
                    }
                    // Style the event cell (top cell of merged range).
                    let addr = format!("{}{xl_row_start}", col_letter(xl_col));
                    let s = ws.get_style_mut(addr.as_str());
                    s.set_background_color_solid(COLOR_EVENT_BG);
                    s.get_font_mut().set_bold(true);
                    s.get_font_mut().set_size(13.0);
                    s.get_font_mut().get_color_mut().set_argb(COLOR_EVENT_FG);
                    s.get_alignment_mut().set_wrap_text(true);
                    s.get_alignment_mut()
                        .set_vertical(VerticalAlignmentValues::Top);
                    apply_thin_border(s);
                    // Thick left accent border on every row in the merged range.
                    if let Some(ref css_hex) = panel.type_color {
                        let argb = css_hex_to_argb(css_hex);
                        for accent_row in xl_row_start..=xl_row_end {
                            let accent_addr = format!("{}{accent_row}", col_letter(xl_col));
                            let as_ = ws.get_style_mut(accent_addr.as_str());
                            let b = as_.get_borders_mut();
                            b.get_left_mut().set_border_style(Border::BORDER_MEDIUM);
                            b.get_left_mut().get_color_mut().set_argb(argb.clone());
                        }
                    }
                }
            }
        }
    }
}

// ── Grid sheet tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_naive(day: u32, hour: u32, min: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 6, day)
            .unwrap()
            .and_hms_opt(hour, min, 0)
            .unwrap()
    }

    #[test]
    fn test_col_letter_basic() {
        assert_eq!(col_letter(1), "A");
        assert_eq!(col_letter(26), "Z");
        assert_eq!(col_letter(27), "AA");
        assert_eq!(col_letter(52), "AZ");
    }

    #[test]
    fn test_grid_time_label() {
        assert_eq!(grid_time_label(make_naive(27, 14, 0)), "2 PM");
        assert_eq!(grid_time_label(make_naive(27, 14, 30)), "2:30");
        assert_eq!(grid_time_label(make_naive(27, 0, 0)), "12 AM");
        assert_eq!(grid_time_label(make_naive(27, 12, 0)), "12 PM");
    }

    #[test]
    fn test_grid_sheet_name_truncates() {
        let long_label = "A very long day label that exceeds the limit";
        let name = grid_sheet_name(long_label);
        assert!(name.len() <= 31);
        assert!(name.starts_with("Grid - "));
    }

    #[test]
    fn test_format_day_label() {
        let dt = make_naive(27, 10, 0);
        let label = format_day_label(dt);
        assert!(
            label.contains("27"),
            "label should contain day number: {label}"
        );
    }

    fn make_simple_day_split(panels: Vec<(NaiveDateTime, NaiveDateTime)>) -> usize {
        let mut days: Vec<(String, Vec<(NaiveDateTime, NaiveDateTime)>)> = Vec::new();
        let mut current_max_end: Option<NaiveDateTime> = None;
        for (start, end) in panels {
            let is_new_day = match current_max_end {
                None => true,
                Some(prev_end) => {
                    let gap = (start - prev_end).num_minutes();
                    gap > OVERNIGHT_GAP_MINUTES && start.date() != prev_end.date()
                }
            };
            if is_new_day {
                let label = format_day_label(start);
                days.push((label, Vec::new()));
            }
            let e = end;
            days.last_mut().unwrap().1.push((start, end));
            current_max_end = Some(match current_max_end {
                Some(prev) if e > prev => e,
                Some(prev) => prev,
                None => e,
            });
        }
        days.len()
    }

    #[test]
    fn test_split_same_day_small_gap() {
        // 2 h gap, same date — same logical day.
        let count = make_simple_day_split(vec![
            (make_naive(27, 10, 0), make_naive(27, 11, 0)),
            (make_naive(27, 13, 0), make_naive(27, 14, 0)),
        ]);
        assert_eq!(count, 1, "2 h gap same date: same day");
    }

    #[test]
    fn test_split_large_gap_same_date_no_split() {
        // 5 h gap but same calendar date — no split (e.g. setup gap during the day).
        let count = make_simple_day_split(vec![
            (make_naive(27, 8, 0), make_naive(27, 9, 0)),
            (make_naive(27, 14, 1), make_naive(27, 15, 0)),
        ]);
        assert_eq!(count, 1, "gap > 4 h but same date: same logical day");
    }

    #[test]
    fn test_split_small_gap_across_midnight_no_split() {
        // 2 h gap crossing midnight — stays as one logical day.
        let count = make_simple_day_split(vec![
            (make_naive(27, 23, 0), make_naive(28, 0, 0)),
            (make_naive(28, 1, 0), make_naive(28, 2, 0)),
        ]);
        assert_eq!(count, 1, "2 h gap crossing midnight: same logical day");
    }
}
