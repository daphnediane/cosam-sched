/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use chrono::NaiveDateTime;
use indexmap::IndexMap;
use regex::Regex;
use umya_spreadsheet::Spreadsheet;

use crate::data::event::EventConflict;
use crate::data::panel::{ExtraValue, FormulaValue, Panel, apply_common_prefix};
use crate::data::panel_id::PanelId;
use crate::data::panel_type::PanelType;
use crate::data::presenter::{Presenter, PresenterGroup, PresenterMember, PresenterRank};
use crate::data::room::Room;
use crate::data::source_info::{ChangeState, SourceInfo};
use crate::data::timeline::TimelineEntry;

use super::find_data_range;
use super::headers::{PresenterColumn, PresenterHeader, parse_presenter_header};
use super::people::{PresenterInfo, parse_presenter_data};
use super::{build_column_map, get_cell_number, get_cell_str, get_field, is_truthy, row_to_map};

pub(super) fn read_panels(
    book: &Spreadsheet,
    preferred: &str,
    rooms: &[Room],
    panel_types: &IndexMap<String, PanelType>,
    file_path: &str,
    presenter_ranks: &HashMap<String, String>,
) -> Result<(IndexMap<String, Panel>, Vec<Presenter>, Vec<TimelineEntry>)> {
    let first_sheet_name = book
        .get_sheet_collection()
        .first()
        .map(|s| s.get_name().to_string());
    let first_sheet_ref: &str = first_sheet_name.as_deref().unwrap_or("");
    let range = match find_data_range(book, preferred, &["Schedule", first_sheet_ref]) {
        Some(r) => {
            // Check if the table range is smaller than the actual data
            let ws = book.get_sheet_by_name(&r.sheet_name).unwrap();
            let actual_end_row = ws.get_highest_row();
            let actual_end_col = ws.get_highest_column();

            // If there's more data beyond the table, extend the range
            if actual_end_row > r.end_row {
                super::DataRange {
                    sheet_name: r.sheet_name,
                    start_col: r.start_col,
                    header_row: r.header_row,
                    end_col: actual_end_col.max(r.end_col),
                    end_row: actual_end_row,
                }
            } else {
                r
            }
        }
        None => return Ok((IndexMap::new(), Vec::new(), Vec::new())),
    };

    let ws = book
        .get_sheet_by_name(&range.sheet_name)
        .ok_or_else(|| anyhow::anyhow!("Sheet '{}' not found", range.sheet_name))?;

    if !range.has_data() {
        return Ok((IndexMap::new(), Vec::new(), Vec::new()));
    }

    let (raw_headers, canonical_headers, col_map) = build_column_map(ws, &range);

    let ticket_cols: HashSet<u32> = raw_headers
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            let lower = h.to_lowercase();
            if lower == "ticket_sale"
                || lower == "ticketsale"
                || lower == "ticket sale"
                || lower == "simpletix_event"
                || lower == "simpletixevent"
                || lower == "simpletix event"
            {
                Some(range.start_col + i as u32)
            } else {
                None
            }
        })
        .collect();

    let presenter_cols: Vec<PresenterColumn> = raw_headers
        .iter()
        .enumerate()
        .filter_map(|(i, h)| parse_presenter_header(h, range.start_col + i as u32))
        .collect();

    // Known canonical header names for standard schedule columns
    let known_canonical_headers: HashSet<&str> = [
        "Uniq_ID",
        "UniqID",
        "ID",
        "Id",
        "Name",
        "Panel_Name",
        "PanelName",
        "Description",
        "Start_Time",
        "StartTime",
        "Start",
        "End_Time",
        "EndTime",
        "End",
        "Duration",
        "Room",
        "Room_Name",
        "RoomName",
        "Kind",
        "Panel_Kind",
        "PanelKind",
        "Cost",
        "Capacity",
        "Difficulty",
        "Note",
        "Prereq",
        "Ticket_Sale",
        "TicketSale",
        "Full",
        "Is_Full",
        "IsFull",
        "Hide_Panelist",
        "HidePanelist",
        "Alt_Panelist",
        "AltPanelist",
        "Presenter",
        "Presenters",
        "Presenter_s",
        "Seats_Sold",
        "SeatsSold",
        "PreReg_Max",
        "PreRegMax",
        "Notes_Non_Printing",
        "NotesNonPrinting",
        "Workshop_Notes",
        "WorkshopNotes",
        "Power_Needs",
        "PowerNeeds",
        "Sewing_Machines",
        "SewingMachines",
        "AV_Notes",
        "AVNotes",
        "Have_Ticket_Image",
        "HaveTicketImage",
        "SimpleTix_Event",
        "SimpleTixEvent",
        "Lstart",
        "Lend",
        "Old_Uniq_Id",
        "OldUniqId",
    ]
    .iter()
    .copied()
    .collect();

    // Identify non-standard columns as metadata candidates:
    // columns not in the known set, not a presenter column, and not a ticket column.
    let presenter_col_indices: HashSet<u32> = presenter_cols.iter().map(|pc| pc.col).collect();
    let metadata_cols: Vec<(String, u32)> = raw_headers
        .iter()
        .enumerate()
        .filter_map(|(i, raw_h)| {
            if raw_h.is_empty() {
                return None;
            }
            let col = range.start_col + i as u32;
            if presenter_col_indices.contains(&col) || ticket_cols.contains(&col) {
                return None;
            }
            let is_known = canonical_headers
                .get(i)
                .and_then(|c| c.as_ref())
                .map(|c| known_canonical_headers.contains(c.as_str()))
                .unwrap_or(false);
            if is_known {
                None
            } else {
                Some((raw_h.clone(), col))
            }
        })
        .collect();

    let room_lookup: HashMap<String, &Room> = rooms
        .iter()
        .flat_map(|r| {
            let mut entries = vec![(r.short_name.to_lowercase(), r)];
            entries.push((r.long_name.to_lowercase(), r));
            if !r.hotel_room.is_empty() {
                entries.push((r.hotel_room.to_lowercase(), r));
            }
            entries
        })
        .collect();

    let type_lookup: HashMap<String, &PanelType> = panel_types
        .iter()
        .map(|(prefix, pt)| (prefix.to_lowercase(), pt))
        .collect();

    let mut presenter_map: HashMap<String, PresenterInfo> = HashMap::new();
    let mut panels: IndexMap<String, Panel> = IndexMap::new();
    let mut timeline_entries: Vec<TimelineEntry> = Vec::new();

    let start_time_col = col_map
        .get("Start_Time")
        .or_else(|| col_map.get("StartTime"))
        .or_else(|| col_map.get("Start"))
        .copied();
    let end_time_col = col_map
        .get("End_Time")
        .or_else(|| col_map.get("EndTime"))
        .or_else(|| col_map.get("End"))
        .copied();
    let duration_col = col_map.get("Duration").copied();

    for row in (range.header_row + 1)..=range.end_row {
        let mut data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        for &col in &ticket_cols {
            if let Some(url) = extract_hyperlink_url(ws, col, row) {
                let header_idx = (col - range.start_col) as usize;
                if let Some(canon) = canonical_headers.get(header_idx).and_then(|c| c.as_ref()) {
                    data.insert(canon.clone(), url.clone());
                }
                if let Some(raw) = raw_headers.get(header_idx) {
                    if !raw.is_empty() {
                        data.insert(raw.clone(), url);
                    }
                }
            }
        }

        let raw_uniq_id = get_field(&data, &["Uniq_ID", "UniqID", "ID", "Id"]).cloned();
        // A leading * means this row was soft-deleted by xlsx_update; strip it and mark deleted.
        let (uniq_id, is_deleted_row) = match raw_uniq_id {
            Some(ref s) if s.starts_with('*') => {
                (Some(s.trim_start_matches('*').to_string()), true)
            }
            other => (other, false),
        };
        let raw_name = match get_field(&data, &["Name", "Panel_Name", "PanelName"]) {
            Some(n) => n.clone(),
            None => {
                continue;
            }
        };

        // Strip trailing part/session numbers from title
        let (name, title_part_num, title_session_num) = strip_title_suffix(&raw_name);

        let panel_id = match PanelId::parse(&uniq_id.as_deref().unwrap_or("")) {
            Some(pid) => pid,
            None => {
                // @TODO: Skip these records entirely, this was an old way to delete a panel from the schedule
                // Create a fake panel ID for rows without proper IDs
                // Use title-derived parts if available
                PanelId {
                    prefix: String::new(),
                    prefix_num: row,
                    part_num: title_part_num,
                    session_num: title_session_num,
                    suffix: None,
                }
            }
        };

        // @TODO: Check if panel id has already been used and if so try adding an alphabetical suffix starting with A until a unique id is found

        // Check for conflicts between title suffixes and Uniq ID parts
        let has_conflict = match (
            &panel_id.part_num,
            &panel_id.session_num,
            &title_part_num,
            &title_session_num,
        ) {
            (None, None, Some(_), Some(_)) => true, // ID has none, title has both
            (None, None, Some(_), None) => true,    // ID has none, title has part
            (None, None, None, Some(_)) => true,    // ID has none, title has session
            (Some(_id_part), None, None, Some(_)) => true, // ID has part, title has session
            (None, Some(_id_session), Some(_), None) => true, // ID has session, title has part
            (Some(id_part), Some(id_session), Some(title_part), Some(title_session)) => {
                id_part != title_part || id_session != title_session
            }
            (Some(id_part), None, Some(title_part), None) => id_part != title_part,
            (None, Some(id_session), None, Some(title_session)) => id_session != title_session,
            _ => false,
        };

        let start_time = parse_datetime_value(
            start_time_col.and_then(|c| get_cell_str(ws, c, row)),
            start_time_col.and_then(|c| get_cell_number(ws, c, row)),
        );

        // Allow panels without start times - they might be unscheduled
        let start_time = start_time.unwrap_or_else(|| {
            // Default to a placeholder time for unscheduled panels
            chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2026, 6, 26).unwrap(),
                chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            )
        });

        let end_time_from_cell = parse_datetime_value(
            end_time_col.and_then(|c| get_cell_str(ws, c, row)),
            end_time_col.and_then(|c| get_cell_number(ws, c, row)),
        );
        let duration_minutes = parse_duration_value(
            duration_col.and_then(|c| get_cell_str(ws, c, row)),
            duration_col.and_then(|c| get_cell_number(ws, c, row)),
        );

        let (end_time, duration) = match (end_time_from_cell, duration_minutes) {
            (Some(et), Some(_)) => {
                let diff = (et - start_time).num_minutes().max(0) as u32;
                (et, diff)
            }
            (Some(et), None) => {
                let diff = (et - start_time).num_minutes().max(0) as u32;
                (et, diff)
            }
            (None, Some(d)) => {
                let et = start_time + chrono::Duration::minutes(d as i64);
                (et, d)
            }
            (None, None) => {
                // Panel is unscheduled - no end time or duration
                // Use placeholder values but let is_scheduled() handle it
                let et = start_time + chrono::Duration::hours(1);
                (et, 60)
            }
        };

        let room_name = get_field(&data, &["Room", "Room_Name", "RoomName"]).cloned();
        let room_ids: Vec<u32> = if let Some(ref room_name) = room_name {
            room_name
                .split(',')
                .filter_map(|name| {
                    let trimmed = name.trim();
                    room_lookup.get(&trimmed.to_lowercase()).map(|r| r.uid)
                })
                .collect()
        } else {
            Vec::new()
        };

        let kind_raw = get_field(&data, &["Kind", "Panel_Kind", "PanelKind"]).cloned();
        let panel_type = if !panel_id.prefix.is_empty() {
            type_lookup.get(&panel_id.prefix.to_lowercase()).copied()
        } else {
            None
        };

        let panel_type = panel_type.or_else(|| {
            kind_raw.as_ref().and_then(|kr| {
                panel_types
                    .values()
                    .find(|pt| pt.kind.to_lowercase() == kr.to_lowercase())
            })
        });

        let cost_raw = get_field(&data, &["Cost"]).cloned();
        let (cost, is_free, is_kids) = normalize_cost(cost_raw.as_ref());
        let is_full = get_field(&data, &["Full"])
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let mut credited_presenters: Vec<String> = Vec::new();
        let mut uncredited_presenters: Vec<String> = Vec::new();
        for pc in &presenter_cols {
            let cell_str = match get_cell_str(ws, pc.col, row) {
                Some(s) => s,
                None => continue,
            };

            let rank = pc.rank.as_str();

            // For Other columns, split by commas; for Named, each chunk is the whole cell
            let chunks: Vec<String> = match &pc.header {
                PresenterHeader::Other => split_presenter_names(&cell_str),
                PresenterHeader::Named(_) => vec![cell_str],
            };

            for chunk in chunks {
                let (uid, is_credited) =
                    match parse_presenter_data(&pc.header, rank, &chunk, &mut presenter_map) {
                        Some(r) => r,
                        None => continue,
                    };

                if is_credited {
                    if !credited_presenters.contains(&uid) {
                        credited_presenters.push(uid);
                    }
                } else if !uncredited_presenters.contains(&uid) {
                    uncredited_presenters.push(uid);
                }
            }
        }

        // Fallback: Presenter/Presenters column
        if credited_presenters.is_empty() && uncredited_presenters.is_empty() {
            if let Some(presenter_raw) =
                get_field(&data, &["Presenter", "Presenters", "Presenter_s"])
            {
                for part in split_presenter_names(presenter_raw) {
                    presenter_map
                        .entry(part.clone())
                        .or_insert_with(|| PresenterInfo {
                            rank: PresenterRank::from_str("fan_panelist"),
                            is_member: PresenterMember::NotMember,
                            is_grouped: PresenterGroup::NotGroup,
                        });
                    credited_presenters.push(part);
                }
            }
        }

        let panel_type_uid = panel_type.map(|pt| pt.prefix.clone()).or_else(|| {
            if !panel_id.prefix.is_empty() {
                Some(panel_id.prefix.clone())
            } else {
                None
            }
        });

        // Check if this is a timeline entry
        if let Some(pt) = panel_type {
            if pt.is_timeline {
                // Get the note field for timeline entries
                let note = get_field(&data, &["Note"]).cloned();

                // Create a TimelineEntry instead of a regular Panel
                let timeline_entry = TimelineEntry {
                    id: uniq_id.unwrap_or_else(|| format!("TL{}", row)).to_string(),
                    start_time: start_time.format("%Y-%m-%dT%H:%M:%S").to_string(),
                    description: name.clone(),
                    panel_type: panel_type_uid.clone(),
                    note,
                    metadata: None,
                    source: Some(SourceInfo {
                        file_path: Some(file_path.to_string()),
                        sheet_name: Some(range.sheet_name.clone()),
                        row_index: Some(row as u32),
                    }),
                    change_state: ChangeState::Unchanged,
                };
                timeline_entries.push(timeline_entry);
                continue; // Skip regular panel processing for timeline entries
            }
        }

        // Get other fields
        let description = get_field(&data, &["Description"]).cloned();
        let note = get_field(&data, &["Note"]).cloned();
        let prereq = get_field(&data, &["Prereq"]).cloned();
        let alt_panelist = get_field(&data, &["Alt_Panelist", "AltPanelist"]).cloned();
        let capacity = get_field(&data, &["Capacity"]).cloned();
        let difficulty = get_field(&data, &["Difficulty"]).cloned();
        let ticket_url = get_field(&data, &["Ticket_Sale", "TicketSale"]).cloned();
        let simple_tix_event = get_field(&data, &["SimpleTix_Event", "SimpleTixEvent"]).cloned();
        let hide_panelist = get_field(&data, &["Hide_Panelist", "HidePanelist"])
            .map(|s| is_truthy(s))
            .unwrap_or(false);
        let seats_sold =
            get_field(&data, &["Seats_Sold", "SeatsSold"]).and_then(|s| s.parse::<u32>().ok());
        let pre_reg_max = get_field(&data, &["PreReg_Max", "PreRegMax"]).cloned();
        let notes_non_printing =
            get_field(&data, &["Notes_Non_Printing", "NotesNonPrinting"]).cloned();
        let workshop_notes = get_field(&data, &["Workshop_Notes", "WorkshopNotes"]).cloned();
        let power_needs = get_field(&data, &["Power_Needs", "PowerNeeds"]).cloned();
        let sewing_machines = get_field(&data, &["Sewing_Machines", "SewingMachines"])
            .map(|s| is_truthy(s))
            .unwrap_or(false);
        let av_notes = get_field(&data, &["AV_Notes", "AVNotes"]).cloned();
        let have_ticket_image =
            get_field(&data, &["Have_Ticket_Image", "HaveTicketImage"]).map(|s| is_truthy(s));

        // Find or create the base panel, handling duplicates
        let is_duplicate = panels.contains_key(&panel_id.base_id());
        let panel = panels.entry(panel_id.base_id()).or_insert_with(|| {
            let mut p = Panel::new(panel_id.base_id());
            p.name = name.clone();
            p.panel_type = panel_type_uid.clone();
            p.cost = cost.clone();
            p.capacity = capacity.clone();
            p.difficulty = difficulty.clone();
            p.ticket_url = ticket_url.clone();
            p.is_free = is_free;
            p.is_kids = is_kids;
            p.simple_tix_event = simple_tix_event.clone();
            p.have_ticket_image = have_ticket_image;
            // Store first description/note/prereq at base level
            p.description = description.clone();
            p.note = note.clone();
            p.prereq = prereq.clone();
            p.credited_presenters = credited_presenters.clone();
            p.uncredited_presenters = uncredited_presenters.clone();
            p
        });

        // Handle duplicate Uniq ID cases
        if is_duplicate {
            if panel.name == name {
                // Same Uniq ID + Same Name → Different sessions with alpha suffixes
            } else {
                // Same Uniq ID + Different Name → Update to new unused ID of same panel type
                // TODO: Generate new unused ID of same panel type
            }
        }

        // Apply common-prefix algorithm at base level.
        // Each call returns (new_entry_suffix, old_prefix_suffix_if_narrowed).
        // Neither value includes the separator space; join_parts adds it back.
        let (base_desc_suffix, narrowed_base_desc) = match description.as_deref() {
            Some(v) => apply_common_prefix(&mut panel.description, v),
            None => (String::new(), None),
        };
        let (base_note_suffix, narrowed_base_note) = match note.as_deref() {
            Some(v) => apply_common_prefix(&mut panel.note, v),
            None => (String::new(), None),
        };
        let (base_prereq_suffix, narrowed_base_prereq) = match prereq.as_deref() {
            Some(v) => apply_common_prefix(&mut panel.prereq, v),
            None => (String::new(), None),
        };

        // When the base prefix narrowed, push the old base tail to all existing parts.
        if narrowed_base_desc.is_some()
            || narrowed_base_note.is_some()
            || narrowed_base_prereq.is_some()
        {
            for ep in &mut panel.parts {
                if let Some(ref tail) = narrowed_base_desc {
                    ep.description = prepend_suffix(tail, ep.description.as_deref());
                }
                if let Some(ref tail) = narrowed_base_note {
                    ep.note = prepend_suffix(tail, ep.note.as_deref());
                }
                if let Some(ref tail) = narrowed_base_prereq {
                    ep.prereq = prepend_suffix(tail, ep.prereq.as_deref());
                }
            }
        }

        let panel_id_str = panel.id.clone();

        // Detect whether this part already exists before find_or_create_part.
        let part_already_exists = panel_id
            .part_num
            .map(|n| panel.parts.iter().any(|p| p.part_num == Some(n)))
            .unwrap_or(!panel.parts.is_empty());

        // Find or create the part
        let part = panel.find_or_create_part(panel_id.part_num);

        // Apply common-prefix at the part level using the base-level suffixes.
        let (part_desc_suffix, part_note_suffix, part_prereq_suffix) = if part_already_exists {
            let (s_desc, n_desc) = apply_common_prefix(&mut part.description, &base_desc_suffix);
            let (s_note, n_note) = apply_common_prefix(&mut part.note, &base_note_suffix);
            let (s_prereq, n_prereq) = apply_common_prefix(&mut part.prereq, &base_prereq_suffix);
            // When part-level fields narrowed, push old tails to all existing sessions.
            if n_desc.is_some() || n_note.is_some() || n_prereq.is_some() {
                for es in &mut part.sessions {
                    if let Some(ref tail) = n_desc {
                        es.description = prepend_suffix(tail, es.description.as_deref());
                    }
                    if let Some(ref tail) = n_note {
                        es.note = prepend_suffix(tail, es.note.as_deref());
                    }
                    if let Some(ref tail) = n_prereq {
                        es.prereq = prepend_suffix(tail, es.prereq.as_deref());
                    }
                }
            }
            (s_desc, s_note, s_prereq)
        } else {
            if !base_desc_suffix.is_empty() {
                part.description = Some(base_desc_suffix.clone());
            }
            if !base_note_suffix.is_empty() {
                part.note = Some(base_note_suffix.clone());
            }
            if !base_prereq_suffix.is_empty() {
                part.prereq = Some(base_prereq_suffix.clone());
            }
            (String::new(), String::new(), String::new())
        };

        // Add presenters to part
        for presenter in &credited_presenters {
            if !part.credited_presenters.contains(presenter) {
                part.credited_presenters.push(presenter.clone());
            }
        }
        for presenter in &uncredited_presenters {
            if !part.uncredited_presenters.contains(presenter) {
                part.uncredited_presenters.push(presenter.clone());
            }
        }

        // Clone values before creating session
        let part_sessions_count = part.sessions.len();

        // Create the session - always add new during import, handle conflicts in post-processing
        let session_id = if let Some(ref id) = uniq_id {
            id.clone()
        } else {
            format!("{}-session-{}", panel_id_str, part_sessions_count)
        };

        let session = part.create_new_session(panel_id.session_num, session_id);

        // Set session fields
        session.room_ids = room_ids;
        session.start_time = Some(start_time.format("%Y-%m-%dT%H:%M:%S").to_string());
        session.end_time = Some(end_time.format("%Y-%m-%dT%H:%M:%S").to_string());
        session.duration = duration;
        session.is_full = is_full;
        session.capacity = capacity;
        session.seats_sold = seats_sold;
        session.pre_reg_max = pre_reg_max;
        session.ticket_url = ticket_url;
        session.simple_tix_event = simple_tix_event;
        session.hide_panelist = hide_panelist;
        session.notes_non_printing = notes_non_printing;
        session.workshop_notes = workshop_notes;
        session.power_needs = power_needs;
        session.sewing_machines = sewing_machines;
        session.av_notes = av_notes;
        session.source = Some(SourceInfo {
            file_path: Some(file_path.to_string()),
            sheet_name: Some(range.sheet_name.clone()),
            row_index: Some(row),
        });
        if is_deleted_row {
            session.change_state = ChangeState::Deleted;
        }

        // Store the part-level suffixes as the session's unique fields.
        if !part_desc_suffix.is_empty() {
            session.description = Some(part_desc_suffix);
        }
        if !part_note_suffix.is_empty() {
            session.note = Some(part_note_suffix);
        }
        if !part_prereq_suffix.is_empty() {
            session.prereq = Some(part_prereq_suffix);
        }

        // Add conflict if detected
        if has_conflict {
            let conflict_details = format!(
                "Title suffix (Part:{}, Session:{}) doesn't match Uniq ID (Part:{}, Session:{})",
                title_part_num.unwrap_or(0),
                title_session_num.unwrap_or(0),
                panel_id.part_num.unwrap_or(0),
                panel_id.session_num.unwrap_or(0)
            );
            session.conflicts.push(EventConflict {
                conflict_type: "title_id_mismatch".to_string(),
                details: Some(conflict_details),
                conflict_event_id: None,
            });
        }

        // Store alt_panelist at session level; post-processing promotes uniform values upward.
        session.alt_panelist = alt_panelist;

        // Add presenters to session
        session.credited_presenters = credited_presenters;
        session.uncredited_presenters = uncredited_presenters;

        // Collect metadata from non-standard columns
        if !metadata_cols.is_empty() {
            let metadata: IndexMap<String, ExtraValue> = metadata_cols
                .iter()
                .filter_map(|(raw_h, col)| {
                    let cell = ws.get_cell((*col, row))?;
                    let formula = cell.get_formula().to_string();
                    let str_val = ws.get_value((*col, row)).trim().to_string();
                    if str_val.is_empty() && formula.is_empty() {
                        return None;
                    }
                    let value = if !formula.is_empty() {
                        ExtraValue::Formula(FormulaValue {
                            formula,
                            value: str_val,
                        })
                    } else {
                        ExtraValue::String(str_val)
                    };
                    Some((raw_h.clone(), value))
                })
                .collect();
            if !metadata.is_empty() {
                session.metadata = metadata;
            }
        }
    }

    // Post-processing: promote uniform alt_panelist values up the hierarchy.
    // If all sessions within a part share the same value, move it to the part level.
    // If all parts then share the same value, move it to the base level.
    for panel in panels.values_mut() {
        for part in &mut panel.parts {
            if part.sessions.is_empty() {
                continue;
            }
            let first = part.sessions[0].alt_panelist.clone();
            if part.sessions.iter().all(|s| s.alt_panelist == first) {
                part.alt_panelist = first;
                for session in &mut part.sessions {
                    session.alt_panelist = None;
                }
            }
        }

        if panel.parts.is_empty() {
            continue;
        }
        let first = panel.parts[0].alt_panelist.clone();
        if panel.parts.iter().all(|p| p.alt_panelist == first) {
            panel.alt_panelist = first;
            for part in &mut panel.parts {
                part.alt_panelist = None;
            }
        }
    }

    let mut presenters: Vec<Presenter> = presenter_map
        .into_iter()
        .map(|(name, info)| {
            // Use preserved rank from People sheet if available, otherwise use inferred rank
            let rank = if let Some(preserved_rank) = presenter_ranks.get(&name) {
                PresenterRank::from_str(preserved_rank)
            } else {
                info.rank
            };

            Presenter {
                id: None,
                name,
                rank,
                is_member: info.is_member.clone(),
                is_grouped: info.is_grouped.clone(),
                metadata: None,
                source: None,
                change_state: ChangeState::Converted,
            }
        })
        .collect();

    presenters.sort_by(|a, b| a.name.cmp(&b.name));

    Ok((panels, presenters, timeline_entries))
}

fn extract_hyperlink_url(ws: &umya_spreadsheet::Worksheet, col: u32, row: u32) -> Option<String> {
    let cell = ws.get_cell((col, row))?;

    if let Some(hyperlink) = cell.get_hyperlink() {
        let url = hyperlink.get_url();
        if !url.is_empty() {
            return Some(url.to_string());
        }
    }

    let formula = cell.get_formula();
    if !formula.is_empty() {
        if let Some(url) = parse_hyperlink_formula(formula) {
            return Some(url);
        }
    }

    None
}

/// Parse `HYPERLINK("url","text")` (without leading `=`) and return the URL.
fn parse_hyperlink_formula(formula: &str) -> Option<String> {
    let upper = formula.to_uppercase();
    if !upper.starts_with("HYPERLINK(") {
        return None;
    }
    let re = Regex::new(r#"(?i)^HYPERLINK\s*\(\s*"([^"]+)""#).ok()?;
    re.captures(formula).map(|caps| caps[1].to_string())
}

fn excel_serial_to_naive_datetime(serial: f64) -> Option<NaiveDateTime> {
    let epoch = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let days = serial.floor() as i64;
    let fraction = serial - serial.floor();
    let seconds_in_day = (fraction * 86400.0).round() as i64;

    let date = epoch + chrono::Duration::days(days);
    let time = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
        seconds_in_day.clamp(0, 86399) as u32,
        0,
    )?;
    Some(NaiveDateTime::new(date, time))
}

fn parse_datetime_value(str_val: Option<String>, num_val: Option<f64>) -> Option<NaiveDateTime> {
    if let Some(s) = str_val {
        if let Some(dt) = parse_datetime_string(&s) {
            return Some(dt);
        }
    }
    if let Some(f) = num_val {
        return excel_serial_to_naive_datetime(f);
    }
    None
}

fn parse_datetime_string(text: &str) -> Option<NaiveDateTime> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // ISO format
    if let Ok(dt) = NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S") {
        return Some(dt);
    }

    // M-DD-YY HH:MM format (e.g., "6-27-26 18:00")
    let re_short = Regex::new(r"^(\d{1,2})-(\d{1,2})-(\d{2})\s+(\d{1,2}):(\d{2})$").ok()?;
    if let Some(caps) = re_short.captures(text) {
        let month: u32 = caps[1].parse().ok()?;
        let day: u32 = caps[2].parse().ok()?;
        let year_short: u32 = caps[3].parse().ok()?;
        let hour: u32 = caps[4].parse().ok()?;
        let minute: u32 = caps[5].parse().ok()?;

        // Convert 2-digit year to 4-digit year (assuming 2000s for 00-99)
        let year = if year_short >= 70 {
            1900 + year_short as i32
        } else {
            2000 + year_short as i32
        };

        let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
        let time = chrono::NaiveTime::from_hms_opt(hour, minute, 0)?;
        return Some(NaiveDateTime::new(date, time));
    }

    // M/DD/YYYY H:MM AM/PM
    let re_us =
        Regex::new(r"^(\d{1,2})/(\d{1,2})/(\d{4})\s+(\d{1,2}):(\d{2})(?::(\d{2}))?\s*(AM|PM)?$")
            .ok()?;

    if let Some(caps) = re_us.captures(text) {
        let month: u32 = caps[1].parse().ok()?;
        let day: u32 = caps[2].parse().ok()?;
        let year: i32 = caps[3].parse().ok()?;
        let mut hour: u32 = caps[4].parse().ok()?;
        let minute: u32 = caps[5].parse().ok()?;
        let second: u32 = caps
            .get(6)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);

        if let Some(ampm) = caps.get(7) {
            match ampm.as_str() {
                "PM" if hour < 12 => hour += 12,
                "AM" if hour == 12 => hour = 0,
                _ => {}
            }
        }

        let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
        let time = chrono::NaiveTime::from_hms_opt(hour, minute, second)?;
        return Some(NaiveDateTime::new(date, time));
    }

    None
}

fn parse_duration_value(str_val: Option<String>, num_val: Option<f64>) -> Option<u32> {
    if let Some(s) = str_val {
        if let Some(d) = parse_duration_string(&s) {
            return Some(d);
        }
    }
    if let Some(f) = num_val {
        if f > 0.0 && f < 1.0 {
            return Some((f * 24.0 * 60.0).round() as u32);
        }
        if f >= 1.0 {
            return Some(f as u32);
        }
    }
    None
}

fn parse_duration_string(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // H:MM or HH:MM
    let re_hm = Regex::new(r"^(\d+):(\d{1,2})$").ok()?;
    if let Some(caps) = re_hm.captures(text) {
        let hours: u32 = caps[1].parse().ok()?;
        let minutes: u32 = caps[2].parse().ok()?;
        return Some(hours * 60 + minutes);
    }

    // Plain number = minutes (only integers, not decimals)
    if let Ok(minutes) = text.parse::<u32>() {
        return Some(minutes);
    }

    None
}

fn normalize_cost(text: Option<&String>) -> (Option<String>, bool, bool) {
    let text = match text {
        Some(t) => t.trim(),
        None => return (None, true, false),
    };

    if text.is_empty() || text == "*" {
        return (None, true, false);
    }

    let lower = text.to_lowercase();
    if lower == "free" || lower == "n/a" || lower == "nothing" || lower == "$0" || lower == "$0.00"
    {
        return (None, true, false);
    }
    if lower == "kids" {
        return (None, true, true);
    }

    (Some(text.to_string()), false, false)
}

fn split_presenter_names(text: &str) -> Vec<String> {
    let re = Regex::new(r"\s*(?:,\s*(?:and\s+)?|\band\s+)").expect("valid regex");
    re.split(text)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Strip trailing part/session numbers from a panel title
///
/// Removes patterns like:
/// - " (Session #)"
/// - " (Part #)"  
/// - " (Part #, Session #)"
///
/// Returns the cleaned title and a tuple of (part_num, session_num) if found
fn strip_title_suffix(title: &str) -> (String, Option<u32>, Option<u32>) {
    let re = Regex::new(r"(?i)\s*\((?:Part\s+(\d+)(?:,\s*Session\s+(\d+))?|Session\s+(\d+))\)\s*$")
        .expect("valid regex");

    if let Some(caps) = re.captures(title) {
        let base_title = title[..caps.get(0).unwrap().start()].trim().to_string();

        let part_num = caps.get(1).and_then(|m| m.as_str().parse().ok());
        let session_num = caps
            .get(2)
            .or_else(|| caps.get(3))
            .and_then(|m| m.as_str().parse().ok());

        (base_title, part_num, session_num)
    } else {
        (title.to_string(), None, None)
    }
}

/// Prepend a narrowed base/part prefix tail to an existing sibling field value.
fn prepend_suffix(prefix: &str, existing: Option<&str>) -> Option<String> {
    if prefix.is_empty() {
        return existing.map(|s| s.to_string());
    }
    match existing.filter(|s| !s.is_empty()) {
        None => Some(prefix.to_string()),
        Some(val) => Some(format!("{} {}", prefix, val)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_title_suffix() {
        // Test basic suffix removal
        let (title, part, session) = strip_title_suffix("My Panel (Part 1)");
        assert_eq!(title, "My Panel");
        assert_eq!(part, Some(1));
        assert_eq!(session, None);

        let (title, part, session) = strip_title_suffix("My Panel (Session 2)");
        assert_eq!(title, "My Panel");
        assert_eq!(part, None);
        assert_eq!(session, Some(2));

        let (title, part, session) = strip_title_suffix("My Panel (Part 3, Session 2)");
        assert_eq!(title, "My Panel");
        assert_eq!(part, Some(3));
        assert_eq!(session, Some(2));

        // Test no suffix
        let (title, part, session) = strip_title_suffix("My Panel");
        assert_eq!(title, "My Panel");
        assert_eq!(part, None);
        assert_eq!(session, None);

        // Test with extra spaces
        let (title, part, session) = strip_title_suffix("My Panel   (Part 1)   ");
        assert_eq!(title, "My Panel");
        assert_eq!(part, Some(1));
        assert_eq!(session, None);

        // Test case insensitive
        let (title, part, session) = strip_title_suffix("My Panel (part 1, session 2)");
        assert_eq!(title, "My Panel");
        assert_eq!(part, Some(1));
        assert_eq!(session, Some(2));
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration_string("1:00"), Some(60));
        assert_eq!(parse_duration_string("1:30"), Some(90));
        assert_eq!(parse_duration_string("2:00"), Some(120));
        assert_eq!(parse_duration_string("90"), Some(90));
        assert_eq!(parse_duration_string(""), None);
    }

    #[test]
    fn test_normalize_cost() {
        assert_eq!(normalize_cost(None), (None, true, false));
        assert_eq!(
            normalize_cost(Some(&"Free".to_string())),
            (None, true, false)
        );
        assert_eq!(
            normalize_cost(Some(&"Kids".to_string())),
            (None, true, true)
        );
        assert_eq!(
            normalize_cost(Some(&"$20.00".to_string())),
            (Some("$20.00".to_string()), false, false)
        );
    }

    #[test]
    fn test_split_presenter_names() {
        let names = split_presenter_names("Alice, Bob and Charlie");
        assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);

        let names = split_presenter_names("Alice, and Bob");
        assert_eq!(names, vec!["Alice", "Bob"]);

        let names = split_presenter_names("Single Name");
        assert_eq!(names, vec!["Single Name"]);
    }

    #[test]
    fn test_parse_hyperlink_formula() {
        let url = parse_hyperlink_formula(
            r#"HYPERLINK("https://www.simpletix.com/e/fw001-tickets-219590","purchase")"#,
        );
        assert_eq!(
            url.as_deref(),
            Some("https://www.simpletix.com/e/fw001-tickets-219590")
        );
        assert!(parse_hyperlink_formula("SUM(A1:A2)").is_none());
        assert!(parse_hyperlink_formula("").is_none());
    }

    #[test]
    fn test_parse_datetime_string() {
        let dt = parse_datetime_string("2026-06-26T14:00:00").expect("should parse ISO");
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-06-26 14:00");

        let dt = parse_datetime_string("6/26/2026 2:00 PM").expect("should parse US date");
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-06-26 14:00");
    }
}
