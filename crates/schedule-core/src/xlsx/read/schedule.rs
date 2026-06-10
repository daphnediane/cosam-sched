/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the Schedule sheet → [`PanelEntityType`] entities + presenter edges.

use std::collections::HashMap;

use anyhow::Result;
use chrono::{Duration, NaiveDateTime};
use regex::Regex;

use crate::edit::builder::find_or_create_entity;
use crate::entity::{EntityType, EntityUuid};
use crate::field::set::FieldUpdate;
use crate::schedule::Schedule;
use crate::sidecar::{EntityOrigin, XlsxSourceInfo};
use crate::tables::event_room::EventRoomId;
use crate::tables::panel::{self, PanelEntityType, PanelId};
use crate::tables::panel_type::PanelTypeId;
use crate::tables::presenter::{
    find_or_create_tagged_presenter, PresenterEntityType, PresenterId, PresenterRank, RankSource,
};
use crate::tables::timeline::{self, TimelineEntityType, TimelineId};
use crate::value::time::{parse_datetime, parse_duration};
use crate::value::uniq_id::PanelUniqId;
use crate::xlsx::columns::schedule as sc;

use super::{
    build_column_map, find_data_range, get_cell_number, get_cell_str, is_truthy,
    known_field_key_set, parse_presenter_header, route_extra_columns, row_to_map, PresenterColumn,
    PresenterHeader, PresenterImportCache,
};

impl super::ImportContext<'_> {
    /// Read the Schedule sheet and create Panel entities with all relationships.
    ///
    /// Accumulates seen Panel/Timeline UUIDs into `self.seen_panels` and seen Presenter
    /// UUIDs (from presenter columns) into `self.seen_presenters`.
    pub(super) fn read_schedule(&mut self) -> Result<()> {
        let mode = self.options.schedule.clone();
        let first_sheet_name = self
            .book
            .get_sheet_collection()
            .first()
            .map(|s| s.get_name().to_string());
        let first_ref = first_sheet_name.as_deref().unwrap_or("");

        let range = match find_data_range(self.book, self.csv_map, &mode, &["Schedule", first_ref])
        {
            Some(r) => {
                // If actual data extends beyond the named table, expand the range.
                let ws = self.book.get_sheet_by_name(&r.sheet_name).unwrap();
                let actual_end_row = ws.get_highest_row();
                let actual_end_col = ws.get_highest_column();
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
            None => return Ok(()),
        };

        let ws = match self.book.get_sheet_by_name(&range.sheet_name) {
            Some(ws) => ws,
            None => return Ok(()),
        };

        if !range.has_data() {
            return Ok(());
        }

        let (raw_headers, canonical_headers, col_map) = build_column_map(ws, &range);
        let known_keys = known_field_key_set(sc::ALL, &[sc::OLD_UNIQ_ID]);

        // Identify presenter columns.
        let presenter_cols: Vec<PresenterColumn> = raw_headers
            .iter()
            .enumerate()
            .filter_map(|(i, h)| parse_presenter_header(h, range.start_col + i as u32))
            .collect();

        // Identify ticket columns for hyperlink extraction.
        let ticket_keys: std::collections::HashSet<String> = sc::TICKET_SALE
            .keys()
            .chain(sc::SIMPLE_TIX_EVENT.keys())
            .filter_map(super::canonical_header)
            .collect();
        let ticket_cols: std::collections::HashSet<u32> = canonical_headers
            .iter()
            .enumerate()
            .filter_map(|(i, canon)| {
                let key = canon.as_deref()?;
                if ticket_keys.contains(key) {
                    Some(range.start_col + i as u32)
                } else {
                    None
                }
            })
            .collect();

        let start_time_col = col_map.get(sc::START_TIME.canonical).copied();
        let end_time_col = col_map.get(sc::END_TIME.canonical).copied();
        let duration_col = col_map.get(sc::DURATION.canonical).copied();

        for row in (range.header_row + 1)..=range.end_row {
            let mut data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

            // Extract hyperlink URLs from ticket/SimpleTix columns.
            for &col in &ticket_cols {
                if let Some(url) = extract_hyperlink_url(ws, col, row) {
                    let idx = (col - range.start_col) as usize;
                    if let Some(canon) = canonical_headers.get(idx).and_then(|c| c.as_ref()) {
                        data.insert(canon.clone(), url.clone());
                    }
                    if let Some(raw) = raw_headers.get(idx) {
                        if !raw.is_empty() {
                            data.insert(raw.clone(), url);
                        }
                    }
                }
            }

            // Require a Name field.
            let name = match get_field_def(&data, &sc::NAME) {
                Some(n) => n.clone(),
                None => continue,
            };

            // A `*` anywhere on the Uniq ID marks the panel as *unscheduled*: it
            // still imports (the code is not required), but with no start time.
            // Deleting a panel is done by removing its row from the sheet, which
            // soft-deletes it as "not seen on re-import".
            let raw_uniq_id = get_field_def(&data, &sc::UNIQ_ID).cloned();
            let force_unscheduled = raw_uniq_id.as_deref().is_some_and(|s| s.contains('*'));
            let uniq_id_str = raw_uniq_id
                .map(|s| s.replace('*', "").trim().to_string())
                .filter(|s| !s.is_empty());

            // Parse timing.
            let start_time = parse_cell_datetime(
                start_time_col.and_then(|c| get_cell_str(ws, c, row)),
                start_time_col.and_then(|c| get_cell_number(ws, c, row)),
            );
            let end_time_raw = parse_cell_datetime(
                end_time_col.and_then(|c| get_cell_str(ws, c, row)),
                end_time_col.and_then(|c| get_cell_number(ws, c, row)),
            );
            let duration_minutes = parse_cell_duration(
                duration_col.and_then(|c| get_cell_str(ws, c, row)),
                duration_col.and_then(|c| get_cell_number(ws, c, row)),
            );

            // Resolve effective duration from whatever combination we have.
            let (effective_start, effective_duration) =
                resolve_timing(start_time, end_time_raw, duration_minutes);
            // A `*`-marked panel is unscheduled: drop any start time (keeping the
            // duration as metadata). With no start it sorts last in the export.
            let effective_start = if force_unscheduled {
                None
            } else {
                effective_start
            };

            // Look up rooms (comma-separated).
            let room_ids: Vec<EventRoomId> = get_field_def(&data, &sc::ROOM)
                .map(|r| {
                    r.split(',')
                        .filter_map(|name| {
                            self.room_lookup
                                .get(name.trim().to_lowercase().as_str())
                                .copied()
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Determine panel type from prefix or Kind column.
            // Use local re-borrows so the immutable borrows don't conflict with
            // the mutable schedule borrow held by the for-loop body.
            let parsed_code = uniq_id_str.as_deref().and_then(PanelUniqId::parse);
            let panel_type_lookup = &self.panel_type_lookup;
            let schedule_ref = &*self.schedule;
            let panel_type_id: Option<PanelTypeId> = parsed_code
                .as_ref()
                .and_then(|c| panel_type_lookup.get(c.type_prefix()))
                .copied()
                .or_else(|| {
                    get_field_def(&data, &sc::KIND).and_then(|kind| {
                        panel_type_lookup
                            .values()
                            .find(|&&pt_id| {
                                schedule_ref
                                    .get_internal::<crate::tables::panel_type::PanelTypeEntityType>(
                                        pt_id,
                                    )
                                    .map(|d| {
                                        d.data.panel_kind.to_lowercase() == kind.to_lowercase()
                                    })
                                    .unwrap_or(false)
                            })
                            .copied()
                    })
                });

            // Parse cost string into typed fields.
            // Blank on a workshop means the cost hasn't been set yet (TBD).
            let (additional_cost, cost_for_kids) = parse_cost_fields(
                get_field_def(&data, &sc::COST).map(|s| s.as_str()),
                panel_type_id,
                self.schedule,
            );

            let is_full = get_field_def(&data, &sc::FULL)
                .map(|s| is_truthy(s))
                .unwrap_or(false);
            let hide_panelist = get_field_def(&data, &sc::HIDE_PANELIST)
                .map(|s| is_truthy(s))
                .unwrap_or(false);
            let sewing_machines = get_field_def(&data, &sc::SEWING_MACHINES)
                .map(|s| is_truthy(s))
                .unwrap_or(false);

            let capacity = get_field_def(&data, &sc::CAPACITY).and_then(|s| s.parse::<i64>().ok());
            let pre_reg_max =
                get_field_def(&data, &sc::PRE_REG_MAX).and_then(|s| s.parse::<i64>().ok());

            // Determine Uniq ID string (synthesize row-based ID if missing).
            // Normalize via PanelUniqId so the upsert key matches what find_by_code
            // will read back from stored full_id() after parse+normalize.
            let code_str = uniq_id_str.unwrap_or_else(|| format!("XX{row:03}"));
            let upsert_name = parsed_code
                .as_ref()
                .map(|c| c.full_id())
                .unwrap_or_else(|| code_str.to_uppercase());

            // Upsert Panel entity via field system.
            let mut updates: Vec<FieldUpdate<PanelEntityType>> = vec![
                FieldUpdate::set(&crate::tables::panel::FIELD_CODE, code_str.as_str()),
                FieldUpdate::set(&crate::tables::panel::FIELD_NAME, name.as_str()),
                FieldUpdate::set(&crate::tables::panel::FIELD_IS_FULL, is_full),
                FieldUpdate::set(&crate::tables::panel::FIELD_HIDE_PANELIST, hide_panelist),
                FieldUpdate::set(
                    &crate::tables::panel::FIELD_SEWING_MACHINES,
                    sewing_machines,
                ),
            ];

            updates.push(FieldUpdate::set(
                &crate::tables::panel::FIELD_ADDITIONAL_COST,
                additional_cost,
            ));
            if cost_for_kids {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_FOR_KIDS,
                    true,
                ));
            }
            if let Some(ref d) = get_field_def(&data, &sc::DESCRIPTION).cloned() {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_DESCRIPTION,
                    d.as_str(),
                ));
            }
            if let Some(ref n) = get_field_def(&data, &sc::NOTE).cloned() {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_NOTE,
                    n.as_str(),
                ));
            }
            if let Some(ref n) = get_field_def(&data, &sc::NOTES_NON_PRINTING).cloned() {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_NOTES_NON_PRINTING,
                    n.as_str(),
                ));
            }
            if let Some(ref n) = get_field_def(&data, &sc::WORKSHOP_NOTES).cloned() {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_WORKSHOP_NOTES,
                    n.as_str(),
                ));
            }
            if let Some(ref n) = get_field_def(&data, &sc::POWER_NEEDS).cloned() {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_POWER_NEEDS,
                    n.as_str(),
                ));
            }
            if let Some(ref n) = get_field_def(&data, &sc::AV_NOTES).cloned() {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_AV_NOTES,
                    n.as_str(),
                ));
            }
            if let Some(ref n) = get_field_def(&data, &sc::DIFFICULTY).cloned() {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_DIFFICULTY,
                    n.as_str(),
                ));
            }
            if let Some(ref n) = get_field_def(&data, &sc::PREREQ).cloned() {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_PREREQ,
                    n.as_str(),
                ));
            }
            if let Some(ref n) = get_field_def(&data, &sc::TICKET_SALE).cloned() {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_TICKET_URL,
                    n.as_str(),
                ));
            }
            if let Some(ref n) = get_field_def(&data, &sc::ALT_PANELIST).cloned() {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_ALT_PANELIST,
                    n.as_str(),
                ));
            }
            if let Some(cap) = capacity {
                updates.push(FieldUpdate::set(&crate::tables::panel::FIELD_CAPACITY, cap));
            }
            if let Some(prm) = pre_reg_max {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_PRE_REG_MAX,
                    prm,
                ));
            }
            if let Some(st) = effective_start {
                updates.push(FieldUpdate::set(
                    &crate::tables::panel::FIELD_START_TIME,
                    st,
                ));
            }
            if let Some(dur) = effective_duration {
                updates.push(FieldUpdate::set(&crate::tables::panel::FIELD_DURATION, dur));
            }

            // Check if this is a timeline panel (has is_timeline panel type)
            let is_timeline = panel_type_id
                .and_then(|pt_id| {
                    self.schedule
                        .get_internal::<crate::tables::panel_type::PanelTypeEntityType>(pt_id)
                })
                .map(|pt| pt.data.is_timeline)
                .unwrap_or(false);

            if is_timeline {
                // Upsert Timeline entity instead of Panel entity.
                // Use the same normalized upsert_name computed above (parsed_code.full_id())
                // so that long raw prefixes like "SPLIT001" → "SP001" match what is stored.
                let mut tl_updates: Vec<FieldUpdate<TimelineEntityType>> = vec![
                    FieldUpdate::set(&timeline::FIELD_CODE, code_str.as_str()),
                    FieldUpdate::set(&timeline::FIELD_NAME, name.as_str()),
                ];
                if let Some(ref d) = get_field_def(&data, &sc::DESCRIPTION).cloned() {
                    tl_updates.push(FieldUpdate::set(&timeline::FIELD_DESCRIPTION, d.as_str()));
                }
                if let Some(ref n) = get_field_def(&data, &sc::NOTE).cloned() {
                    tl_updates.push(FieldUpdate::set(&timeline::FIELD_NOTE, n.as_str()));
                }
                if let Some(st) = effective_start {
                    tl_updates.push(FieldUpdate::set(&timeline::FIELD_TIME, st));
                }

                let timeline_id: TimelineId = match find_or_create_entity::<TimelineEntityType>(
                    self.schedule,
                    &upsert_name,
                    &self.seen_timelines,
                    false,
                    tl_updates,
                ) {
                    Ok(id) => id,
                    Err(e) => {
                        eprintln!("xlsx import: skipping timeline {code_str:?}: {e}");
                        continue;
                    }
                };
                self.seen_timelines.insert(timeline_id.entity_uuid());
                self.schedule.sidecar_mut().set_origin(
                    timeline_id.entity_uuid(),
                    EntityOrigin::Xlsx(XlsxSourceInfo {
                        file_path: self.file_path.map(str::to_owned),
                        sheet_name: range.sheet_name.clone(),
                        row_index: row,
                        import_time: self.import_time,
                    }),
                );

                // Replace panel type edge (set, not add, to handle changed type).
                if let Some(pt_id) = panel_type_id {
                    let _ =
                        self.schedule
                            .edge_set(timeline_id, timeline::EDGE_PANEL_TYPES, [pt_id]);
                } else {
                    let _ = self.schedule.edge_set(
                        timeline_id,
                        timeline::EDGE_PANEL_TYPES,
                        std::iter::empty::<TimelineId>(),
                    );
                }

                // Skip the rest of panel-specific processing for timelines
                continue;
            }

            let panel_id: PanelId = match find_or_create_entity::<PanelEntityType>(
                self.schedule,
                &upsert_name,
                &self.seen_panels,
                false,
                updates,
            ) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("xlsx import: skipping panel {code_str:?}: {e}");
                    continue;
                }
            };
            let panel_uuid = panel_id.entity_uuid();
            self.seen_panels.insert(panel_uuid);
            self.schedule.sidecar_mut().set_origin(
                panel_uuid,
                EntityOrigin::Xlsx(XlsxSourceInfo {
                    file_path: self.file_path.map(str::to_owned),
                    sheet_name: range.sheet_name.clone(),
                    row_index: row,
                    import_time: self.import_time,
                }),
            );
            // Build set of presenter column headers to skip from extra fields.
            let presenter_headers: std::collections::HashSet<String> = presenter_cols
                .iter()
                .map(|pc| {
                    let idx = (pc.col - range.start_col) as usize;
                    raw_headers[idx].clone()
                })
                .collect();

            route_extra_columns(
                ws,
                row,
                &range,
                &raw_headers,
                &canonical_headers,
                &known_keys,
                sc::FORMULA_COLUMNS,
                &presenter_headers,
                panel_uuid,
                PanelEntityType::TYPE_NAME,
                self.schedule,
            );

            // Replace room and panel-type edges (set, not add, to reflect import authority).
            let _ = self
                .schedule
                .edge_set(panel_id, panel::EDGE_EVENT_ROOMS, room_ids);
            if let Some(pt_id) = panel_type_id {
                let _ = self
                    .schedule
                    .edge_set(panel_id, panel::EDGE_PANEL_TYPE, [pt_id]);
            } else {
                let _ = self.schedule.edge_set(
                    panel_id,
                    panel::EDGE_PANEL_TYPE,
                    std::iter::empty::<PanelId>(),
                );
            }

            // Parse presenter columns for this row.
            // collect_presenters takes schedule + cache as separate field refs because
            // ws (borrowed from self.book) must remain alive during the call.
            let (credited, uncredited, groups) = collect_presenters(
                ws,
                row,
                &presenter_cols,
                self.schedule,
                &mut self.presenter_cache,
            );

            // Track all seen presenter and group IDs.
            for id in credited
                .iter()
                .chain(uncredited.iter())
                .chain(groups.iter())
            {
                self.seen_presenters.insert(id.entity_uuid());
            }

            // Replace all presenter edges — XLSX is authoritative for both credited
            // and uncredited presenters.
            if hide_panelist {
                let all: Vec<PresenterId> =
                    credited.iter().chain(uncredited.iter()).copied().collect();
                let _ = self.schedule.edge_set(
                    panel_id,
                    panel::EDGE_CREDITED_PRESENTERS,
                    std::iter::empty::<PresenterId>(),
                );
                let _ = self
                    .schedule
                    .edge_set(panel_id, panel::EDGE_UNCREDITED_PRESENTERS, all);
            } else {
                let _ = self
                    .schedule
                    .edge_set(panel_id, panel::EDGE_CREDITED_PRESENTERS, credited);
                let _ =
                    self.schedule
                        .edge_set(panel_id, panel::EDGE_UNCREDITED_PRESENTERS, uncredited);
            }
        }

        Ok(())
    }
}

// ── Presenter collection ──────────────────────────────────────────────────────

fn collect_presenters(
    ws: &umya_spreadsheet::structs::Worksheet,
    row: u32,
    presenter_cols: &[PresenterColumn],
    schedule: &mut Schedule,
    cache: &mut PresenterImportCache,
) -> (
    Vec<crate::tables::presenter::PresenterId>,
    Vec<crate::tables::presenter::PresenterId>,
    Vec<crate::tables::presenter::PresenterId>,
) {
    let mut credited = Vec::new();
    let mut uncredited = Vec::new();
    let mut groups: Vec<PresenterId> = Vec::new();

    for pc in presenter_cols {
        let cell_str = match get_cell_str(ws, pc.col, row) {
            Some(s) => s,
            None => continue,
        };

        let chunks: Vec<String> = match &pc.header {
            PresenterHeader::Other => split_presenter_names(&cell_str),
            PresenterHeader::Named(_) => vec![cell_str],
        };

        for chunk in &chunks {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                continue;
            }

            // Leading `*` marks an uncredited presenter.
            let (name_part, is_uncredited) = if let Some(rest) = chunk.strip_prefix('*') {
                (rest.trim(), true)
            } else {
                (chunk, false)
            };

            // For Named columns the cell is a presence flag; look up by header name.
            // For Other columns the cell value is the tagged name itself.
            let (lookup, force_uncredited) = match &pc.header {
                PresenterHeader::Named(header_name) => {
                    let is_unlisted = chunk.eq_ignore_ascii_case("unlisted");
                    let is_uncredited_flag = chunk.eq_ignore_ascii_case("*");
                    let mark_uncredited = is_unlisted || is_uncredited_flag || is_uncredited;
                    if !mark_uncredited && name_part.is_empty() {
                        continue;
                    }
                    (header_name.as_str(), mark_uncredited)
                }
                PresenterHeader::Other => (name_part, is_uncredited),
            };

            let matched = match find_or_create_tagged_presenter(schedule, lookup) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("xlsx import: skipping presenter {lookup:?}: {e}");
                    continue;
                }
            };

            let id = matched.as_presenter();

            // Build the rank claim for this encounter.  A Named column declares
            // its header rank; an `Other` cell declares a rank only when the cell
            // value carries a tag prefix ("G:Alice"), otherwise it makes no claim.
            // The column rank applies to the named presenter; a group named via a
            // `=Group` suffix only inherits it (Implied).  The cache reconciles
            // these claims with the stored rank at flush — see PresenterImportCache.
            let member_source = match &pc.header {
                PresenterHeader::Named(_) => RankSource::Declared(pc.rank.clone()),
                PresenterHeader::Other => {
                    tag_rank_prefix(lookup).map_or(RankSource::None, RankSource::Declared)
                }
            };
            let group_source = member_source
                .rank()
                .map_or(RankSource::None, |r| RankSource::Implied(r.clone()));

            // The cache also pins the canonical name (People-sheet name wins if
            // recorded there first).
            let presenter_name = schedule
                .get_internal::<PresenterEntityType>(id)
                .map(|d| d.data.name.clone())
                .unwrap_or_else(|| lookup.to_string());
            cache.record(id, &presenter_name, member_source);
            // Also record the group so it participates in cache flush.
            if let Some(gid) = matched.group_id() {
                let group_name = schedule
                    .get_internal::<PresenterEntityType>(gid)
                    .map(|d| d.data.name.clone())
                    .unwrap_or_default();
                cache.record(gid, &group_name, group_source);
            }

            if force_uncredited {
                if !uncredited.contains(&id) {
                    uncredited.push(id);
                }
            } else if !credited.contains(&id) {
                credited.push(id);
            }

            if let Some(gid) = matched.group_id() {
                if !groups.contains(&gid) {
                    groups.push(gid);
                }
            }
        }
    }

    (credited, uncredited, groups)
}

// ── Timing helpers ────────────────────────────────────────────────────────────

fn resolve_timing(
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
    dur_mins: Option<u32>,
) -> (Option<NaiveDateTime>, Option<Duration>) {
    match (start, end, dur_mins) {
        (Some(st), Some(et), _) => {
            let dur = Duration::minutes((et - st).num_minutes().max(0));
            (Some(st), Some(dur))
        }
        (Some(st), None, Some(d)) => (Some(st), Some(Duration::minutes(d as i64))),
        (Some(st), None, None) => (Some(st), None),
        (None, _, Some(d)) => (None, Some(Duration::minutes(d as i64))),
        (None, _, None) => (None, None),
    }
}

fn parse_cell_datetime(str_val: Option<String>, num_val: Option<f64>) -> Option<NaiveDateTime> {
    if let Some(s) = str_val {
        if let Some(dt) = parse_datetime(&s) {
            return Some(dt);
        }
    }
    num_val.and_then(excel_serial_to_naive_datetime)
}

fn parse_cell_duration(str_val: Option<String>, num_val: Option<f64>) -> Option<u32> {
    if let Some(s) = str_val {
        if let Some(d) = parse_duration(&s) {
            return Some(d.num_minutes().max(0) as u32);
        }
    }
    if let Some(f) = num_val {
        // Excel stores time-of-day fractions (< 1.0) and plain integers.
        if f > 0.0 && f < 1.0 {
            return Some((f * 24.0 * 60.0).round() as u32);
        }
        if f >= 1.0 {
            return Some(f as u32);
        }
    }
    None
}

fn excel_serial_to_naive_datetime(serial: f64) -> Option<NaiveDateTime> {
    let epoch = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let days = serial.floor() as i64;
    let fraction = serial - serial.floor();
    let seconds_in_day = (fraction * 86400.0).round() as i64;
    let date = epoch + Duration::days(days);
    let time = chrono::NaiveTime::from_num_seconds_from_midnight_opt(
        seconds_in_day.clamp(0, 86399) as u32,
        0,
    )?;
    Some(NaiveDateTime::new(date, time))
}

// ── Cost parsing ──────────────────────────────────────────────────────────

/// Parses a raw cost cell value from the XLSX into a typed
/// ([`AdditionalCost`], for_kids) pair.
///
/// - `None` / `""` + workshop → (`TBD`, false) — cost not yet entered.
/// - `None` / `""` + non-workshop → (`Included`, false).
/// - `"*"` → (`Included`, false) always (explicit wildcard/placeholder).
/// - `"Kids"` / `"Kid"` → (`Included`, true).
/// - `"Free"`, `"$0"`, etc. → (`Included`, false).
/// - `"TBD"` → (`TBD`, false).
/// - `"$35"`, etc. → (`Premium(cents)`, false).
/// - Unrecognized → (`Included`, false) (safe default).
fn parse_cost_fields(
    text: Option<&str>,
    panel_type_id: Option<crate::tables::panel_type::PanelTypeId>,
    schedule: &crate::schedule::Schedule,
) -> (crate::value::AdditionalCost, bool) {
    use crate::value::cost::{cost_string_is_kid_panel, parse_additional_cost};
    use crate::value::AdditionalCost;
    let text = match text {
        Some(t) => t.trim(),
        None => {
            let is_ws = is_workshop(panel_type_id, schedule);
            return (
                if is_ws {
                    AdditionalCost::TBD
                } else {
                    AdditionalCost::Included
                },
                false,
            );
        }
    };
    if text.is_empty() {
        let is_ws = is_workshop(panel_type_id, schedule);
        return (
            if is_ws {
                AdditionalCost::TBD
            } else {
                AdditionalCost::Included
            },
            false,
        );
    }
    if text == "*" {
        return (AdditionalCost::Included, false);
    }
    let for_kids = cost_string_is_kid_panel(text);
    let cost = parse_additional_cost(text).unwrap_or(AdditionalCost::Included);
    (cost, for_kids)
}

fn is_workshop(
    panel_type_id: Option<crate::tables::panel_type::PanelTypeId>,
    schedule: &crate::schedule::Schedule,
) -> bool {
    panel_type_id
        .and_then(|pt_id| {
            schedule.get_internal::<crate::tables::panel_type::PanelTypeEntityType>(pt_id)
        })
        .map(|d| d.data.is_workshop)
        .unwrap_or(false)
}

// ── Name splitting ────────────────────────────────────────────────────────────

fn split_presenter_names(text: &str) -> Vec<String> {
    let mut results: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut padding = String::new();
    let mut chars = text.chars().peekable();
    let mut quote_char: Option<char> = None;
    let mut paren_depth: u32 = 0;

    while let Some(ch) = chars.next() {
        match ch {
            // Parentheses handling - track depth
            '(' if quote_char.is_none() => {
                paren_depth += 1;
                current.push_str(&padding);
                padding.clear();
                current.push(ch);
            }
            ')' if quote_char.is_none() => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push_str(&padding);
                padding.clear();
                current.push(ch);
            }
            // Quote handling - track state but don't include quotes in output
            '\'' | '"' => {
                if quote_char == Some(ch) {
                    // Ending quote - exit quote mode (don't add to output)
                    quote_char = None;
                } else if quote_char.is_some() {
                    // Different quote type inside quoted string - add it
                    current.push_str(&padding);
                    padding.clear();
                    current.push(ch);
                } else {
                    // Check if this quote should start quote mode:
                    // - At start of chunk (current empty)
                    // - After whitespace (padding not empty)
                    // - After assignment operators = or :
                    let can_start_quote = current.is_empty()
                        || !padding.is_empty()
                        || current
                            .chars()
                            .last()
                            .map(|c| c == '=' || c == ':')
                            .unwrap_or(false);
                    if can_start_quote {
                        // Starting quote - enter quote mode (don't add to output)
                        quote_char = Some(ch);
                    } else {
                        // Quote in middle of name (e.g., possessive "Same's") - add it
                        current.push_str(&padding);
                        padding.clear();
                        current.push(ch);
                    }
                }
            }
            // Comma delimiter (only when not in quotes or parens)
            ',' if quote_char.is_none() && paren_depth == 0 => {
                if !current.is_empty() {
                    results.push(current.to_string());
                }
                current.clear();
                padding.clear();
                // Skip whitespace after comma
                while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                    chars.next();
                }
            }
            // 'a' might start "and " delimiter (only when not in quotes or parens)
            // Requires word boundary: "Alice and Bob" splits, "Pros and Cons" doesn't
            'a' | 'A' if quote_char.is_none() && paren_depth == 0 => {
                let next_three: String = chars.clone().take(3).collect();
                if format!("{ch}{next_three}").to_lowercase() == "and " {
                    // Check word boundary before 'a':
                    // - If padding is not empty, previous char was whitespace (boundary)
                    // - If current is empty, we're at start of string (boundary)
                    let has_boundary_before = !padding.is_empty() || current.is_empty();
                    if has_boundary_before {
                        for _ in 0..3 {
                            chars.next();
                        }
                        if !current.is_empty() {
                            results.push(current.to_string());
                        }
                        current.clear();
                        padding.clear();
                        // Skip whitespace after "and"
                        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
                            chars.next();
                        }
                    } else {
                        // "and" not at word boundary - add as normal text
                        current.push_str(&padding);
                        padding.clear();
                        current.push(ch);
                    }
                } else {
                    current.push_str(&padding);
                    padding.clear();
                    current.push(ch);
                }
            }
            // Any other character
            _ => {
                if !ch.is_whitespace() {
                    current.push_str(&padding);
                    padding.clear();
                    current.push(ch);
                } else if !current.is_empty() {
                    padding.push(ch);
                }
            }
        }
    }

    // Flush final chunk
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        results.push(trimmed.to_string());
    }

    results
}

// ── Tag rank extraction ───────────────────────────────────────────────────────

/// If `tag` starts with a recognised single-character rank prefix followed by
/// `':'` (e.g. `"G:Alice"`, `"F:Alice"`), return the corresponding rank.
/// Returns `None` for untagged values like `"Alice"`.
fn tag_rank_prefix(tag: &str) -> Option<PresenterRank> {
    let tag = tag.trim();
    let mut chars = tag.chars();
    let prefix = chars.next()?;
    if chars.next()? == ':' {
        PresenterRank::from_prefix_char(prefix)
    } else {
        None
    }
}

// ── Hyperlink extraction ──────────────────────────────────────────────────────

fn extract_hyperlink_url(
    ws: &umya_spreadsheet::structs::Worksheet,
    col: u32,
    row: u32,
) -> Option<String> {
    let cell = ws.get_cell((col, row))?;
    if let Some(hyperlink) = cell.get_hyperlink() {
        let url = hyperlink.get_url();
        if !url.is_empty() {
            return Some(url.to_string());
        }
    }
    let formula = cell.get_formula();
    if !formula.is_empty() {
        return parse_hyperlink_formula(formula);
    }
    None
}

fn parse_hyperlink_formula(formula: &str) -> Option<String> {
    let re = Regex::new(r#"(?i)^HYPERLINK\s*\(\s*"([^"]+)""#).ok()?;
    re.captures(formula).map(|c| c[1].to_string())
}

// ── field helpers ─────────────────────────────────────────────────────────────

fn get_field_def<'a>(
    row_data: &'a HashMap<String, String>,
    field: &crate::xlsx::columns::FieldDef,
) -> Option<&'a String> {
    super::get_field_def(row_data, field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cost_fields() {
        use crate::value::AdditionalCost;
        // Build a minimal schedule with a workshop and non-workshop panel type.
        let mut sched = crate::schedule::Schedule::default();
        let ws_id = crate::tables::panel_type::PanelTypeBuilder::new()
            .with_prefix("GW".to_string())
            .with_panel_kind("Workshop".to_string())
            .with_is_workshop(true)
            .build(&mut sched)
            .unwrap();
        let gp_id = crate::tables::panel_type::PanelTypeBuilder::new()
            .with_prefix("GP".to_string())
            .with_panel_kind("General".to_string())
            .with_is_workshop(false)
            .build(&mut sched)
            .unwrap();

        // Non-workshop: blank/None → Included; * → Included.
        assert_eq!(
            parse_cost_fields(None, Some(gp_id), &sched),
            (AdditionalCost::Included, false)
        );
        assert_eq!(
            parse_cost_fields(Some(""), Some(gp_id), &sched),
            (AdditionalCost::Included, false)
        );
        assert_eq!(
            parse_cost_fields(Some("*"), Some(gp_id), &sched),
            (AdditionalCost::Included, false)
        );
        // Workshop: blank/None → TBD; * → Included (explicit wildcard).
        assert_eq!(
            parse_cost_fields(None, Some(ws_id), &sched),
            (AdditionalCost::TBD, false)
        );
        assert_eq!(
            parse_cost_fields(Some(""), Some(ws_id), &sched),
            (AdditionalCost::TBD, false)
        );
        assert_eq!(
            parse_cost_fields(Some("*"), Some(ws_id), &sched),
            (AdditionalCost::Included, false)
        );
        // Explicit values unaffected by workshop flag.
        assert_eq!(
            parse_cost_fields(Some("Free"), Some(ws_id), &sched),
            (AdditionalCost::Included, false)
        );
        assert_eq!(
            parse_cost_fields(Some("$0"), Some(gp_id), &sched),
            (AdditionalCost::Included, false)
        );
        assert_eq!(
            parse_cost_fields(Some("N/A"), Some(gp_id), &sched),
            (AdditionalCost::Included, false)
        );
        // Kids flag is set separately; cost is still Included.
        assert_eq!(
            parse_cost_fields(Some("Kids"), Some(gp_id), &sched),
            (AdditionalCost::Included, true)
        );
        assert_eq!(
            parse_cost_fields(Some("Kid"), Some(gp_id), &sched),
            (AdditionalCost::Included, true)
        );
        assert_eq!(
            parse_cost_fields(Some("TBD"), Some(gp_id), &sched),
            (AdditionalCost::TBD, false)
        );
        assert_eq!(
            parse_cost_fields(Some("TBD"), Some(ws_id), &sched),
            (AdditionalCost::TBD, false)
        );
        assert_eq!(
            parse_cost_fields(Some("$35"), Some(gp_id), &sched),
            (AdditionalCost::Premium(3500), false)
        );
        assert_eq!(
            parse_cost_fields(Some("$35.50"), Some(gp_id), &sched),
            (AdditionalCost::Premium(3550), false)
        );
    }

    #[test]
    fn test_split_presenter_names() {
        assert_eq!(
            split_presenter_names("Alice, Bob and Charlie"),
            vec!["Alice", "Bob", "Charlie"]
        );
        assert_eq!(split_presenter_names("Single Name"), vec!["Single Name"]);
    }

    #[test]
    fn test_split_presenter_names_quoted() {
        // Quoted strings prevent splitting on "and"
        assert_eq!(
            split_presenter_names("\"Ari and Bee Cosplay\""),
            vec!["Ari and Bee Cosplay"]
        );
        assert_eq!(
            split_presenter_names("'Ari and Bee Cosplay'"),
            vec!["Ari and Bee Cosplay"]
        );
        // Mixed with other names
        assert_eq!(
            split_presenter_names("Alice and \"Ari and Bee Cosplay\""),
            vec!["Alice", "Ari and Bee Cosplay"]
        );
        assert_eq!(
            split_presenter_names("\"Ari and Bee Cosplay\" and Bob"),
            vec!["Ari and Bee Cosplay", "Bob"]
        );
        // Multiple quoted groups
        assert_eq!(
            split_presenter_names("\"Group A and B\" and \"Group C and D\""),
            vec!["Group A and B", "Group C and D"]
        );
        // Quoted with comma delimiter
        assert_eq!(
            split_presenter_names("\"Ari and Bee Cosplay\", Alice"),
            vec!["Ari and Bee Cosplay", "Alice"]
        );
    }

    #[test]
    fn test_split_presenter_names_quoted_edge_cases() {
        // Adjacent quoted strings with space between - space should be trimmed
        assert_eq!(
            split_presenter_names("   \"First1\" ' Second1'   , \"Third1\""),
            vec!["First1  Second1", "Third1"]
        );
        // Mixed quote types with space between
        assert_eq!(
            split_presenter_names("'First2 ' \"Second2\""),
            vec!["First2  Second2"]
        );
        // Leading and trailing whitespace should be trimmed
        assert_eq!(
            split_presenter_names("  \"Ari and Bee Cosplay\"  "),
            vec!["Ari and Bee Cosplay"]
        );
        // Internal spaces preserved, outer trimmed
        assert_eq!(
            split_presenter_names("  Alice   and   Bob  "),
            vec!["Alice", "Bob"]
        );
        // 2016 data examples - verify comma splits correctly
        assert_eq!(
            split_presenter_names("Darth Claire Cosplay, G.C. Kinsey"),
            vec!["Darth Claire Cosplay", "G.C. Kinsey"]
        );
        // Test "and" delimiter alone (no comma)
        assert_eq!(split_presenter_names("Alice and Bob"), vec!["Alice", "Bob"]);
        // Test comma with "and" (Oxford comma style)
        assert_eq!(
            split_presenter_names("Alice, Bob, and Charlie"),
            vec!["Alice", "Bob", "Charlie"]
        );
        // Parentheses protect internal 'and'
        assert_eq!(
            split_presenter_names(
                "Ayla Craft Cosplay (Ayla & Jacob), That's So Cosplay (Abbi & Ace)"
            ),
            vec![
                "Ayla Craft Cosplay (Ayla & Jacob)",
                "That's So Cosplay (Abbi & Ace)"
            ]
        );
        // Possessive apostrophe in middle of name (not treated as quote)
        assert_eq!(
            split_presenter_names("Sam's Cosplay and Alice's Workshop"),
            vec!["Sam's Cosplay", "Alice's Workshop"]
        );
        // "and" inside a word shouldn't split - requires word boundary
        assert_eq!(
            split_presenter_names("Band Ampersand And Andrew"),
            vec!["Band Ampersand", "Andrew"]
        );
    }

    #[test]
    fn test_parse_hyperlink_formula() {
        let url = parse_hyperlink_formula(r#"HYPERLINK("https://example.com/tickets","Buy")"#);
        assert_eq!(url.as_deref(), Some("https://example.com/tickets"));
        assert!(parse_hyperlink_formula("SUM(A1:A2)").is_none());
    }

    #[test]
    fn test_resolve_timing_start_end() {
        use chrono::NaiveDate;
        let base = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        let st = base.and_hms_opt(10, 0, 0).unwrap();
        let et = base.and_hms_opt(11, 0, 0).unwrap();
        let (s, d) = resolve_timing(Some(st), Some(et), None);
        assert_eq!(s, Some(st));
        assert_eq!(d, Some(Duration::minutes(60)));
    }

    #[test]
    fn test_resolve_timing_start_duration() {
        use chrono::NaiveDate;
        let base = NaiveDate::from_ymd_opt(2026, 6, 27).unwrap();
        let st = base.and_hms_opt(10, 0, 0).unwrap();
        let (s, d) = resolve_timing(Some(st), None, Some(90));
        assert_eq!(s, Some(st));
        assert_eq!(d, Some(Duration::minutes(90)));
    }
}
