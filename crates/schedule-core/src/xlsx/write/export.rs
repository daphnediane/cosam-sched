/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! XLSX export implementation (FEATURE-029).

use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use umya_spreadsheet::structs::Worksheet;

use crate::entity::{EntityType, EntityUuid};
use crate::schedule::Schedule;
use crate::tables::event_room::{self, EventRoomEntityType};
use crate::tables::hotel_room::HotelRoomEntityType;
use crate::tables::panel::{self, PanelEntityType, PanelInternalData};
use crate::tables::panel_type::PanelTypeEntityType;
use crate::tables::presenter::{self, PresenterEntityType, PresenterId, PresenterRank};
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

    for std_rank in PresenterRank::standard_ranks() {
        let prefix_char = std_rank.prefix_char();

        // Collect presenters for this rank tier who appear on at least one panel.
        let mut named: Vec<(PresenterId, String)> = Vec::new();
        let mut has_other = false;

        let mut rank_presenters: Vec<(PresenterId, &_)> = schedule
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
            let always_grouped = p.data.always_grouped;
            if count >= MIN_PANELS_FOR_NAMED_COLUMN || always_grouped {
                named.push((p_id, p.data.name.clone()));
            } else {
                has_other = true;
            }
        }

        // Sort named presenters by panel count desc, then name asc.
        named.sort_by(|(id_a, name_a), (id_b, name_b)| {
            let ca = panel_count.get(id_a).copied().unwrap_or(0);
            let cb = panel_count.get(id_b).copied().unwrap_or(0);
            cb.cmp(&ca).then_with(|| name_a.cmp(name_b))
        });

        for (p_id, name) in named {
            // Build header with optional group suffix.
            let group_ids =
                schedule.connected_entities::<PresenterEntityType>(p_id, presenter::EDGE_GROUPS);
            let group_name = group_ids
                .first()
                .and_then(|gid| schedule.get_internal::<PresenterEntityType>(*gid))
                .map(|g| g.data.name.as_str());

            let p_internal = schedule
                .get_internal::<PresenterEntityType>(p_id)
                .expect("presenter was in iter_entities");
            let always_grouped = p_internal.data.always_grouped;

            let header = match group_name {
                Some(group) if always_grouped => {
                    format!("{prefix_char}:{name}=={group}")
                }
                Some(group) => format!("{prefix_char}:{name}={group}"),
                None => format!("{prefix_char}:{name}"),
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
    let named_col_lookup: std::collections::HashMap<PresenterId, u32> = presenter_cols
        .iter()
        .enumerate()
        .filter_map(|(i, col)| {
            col.presenter_id
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
    let c_seats_sold = c(&sched_cols::SEATS_SOLD);
    let c_pre_reg_max = c(&sched_cols::PRE_REG_MAX);
    let c_capacity = c(&sched_cols::CAPACITY);
    let c_have_ticket_img = c(&sched_cols::HAVE_TICKET_IMAGE);
    let c_simpletix_event = c(&sched_cols::SIMPLE_TIX_EVENT);
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

        // Cost (with free/kids special cases).
        let cost_str = if panel.data.is_free {
            Some("Free".to_string())
        } else if panel.data.is_kids {
            Some("Kids".to_string())
        } else {
            panel.data.cost.clone()
        };
        set_opt(ws, c_cost, row, &cost_str);

        // Seat counts.
        let seats_sold = panel.data.seats_sold.map(|n| n.to_string());
        let pre_reg_max = panel.data.pre_reg_max.map(|n| n.to_string());
        let capacity = panel.data.capacity.map(|n| n.to_string());
        set_opt(ws, c_seats_sold, row, &seats_sold);
        set_opt(ws, c_pre_reg_max, row, &pre_reg_max);
        set_opt(ws, c_capacity, row, &capacity);

        // Ticketing.
        if panel.data.have_ticket_image {
            set_str(ws, c_have_ticket_img, row, "Yes");
        }
        set_opt(ws, c_simpletix_event, row, &panel.data.simpletix_event);
        set_opt(ws, c_ticket_sale, row, &panel.data.simpletix_link);
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
        if presenter.data.always_grouped {
            set_str(ws, c_always_grouped, row, "Yes");
        }
        if presenter.data.always_shown_in_group {
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
