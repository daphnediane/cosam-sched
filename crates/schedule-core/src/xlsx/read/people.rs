/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the People/Presenters sheet → [`PresenterEntityType`] entities.
//!
//! The People sheet is read before the Schedule sheet so that presenters
//! encountered on the Schedule can be matched by name and their rank upgraded
//! if the People sheet carries a higher-priority classification.
//!
//! The People sheet carries explicit `Classification`, `Is Group`,
//! `Always Grouped`, and `Always Shown` columns.  These are set directly via
//! the field system so that all writes flow through the CRDT mirror.
//!
//! Group membership (Members / Groups columns) is intentionally not processed
//! here; those relationships are established by the `=Group` / `==Group`
//! syntax on the Schedule sheet's presenter columns.

use anyhow::Result;
use chrono::{DateTime, Utc};
use umya_spreadsheet::Spreadsheet;

use crate::edit::builder::build_entity;
use crate::entity::{EntityType, EntityUuid, UuidPreference};
use crate::field::set::FieldUpdate;
use crate::schedule::Schedule;
use crate::sidecar::{EntityOrigin, XlsxSourceInfo};
use crate::tables::presenter::{self, PresenterEntityType, PresenterRank};
use crate::xlsx::columns::people as pc;

use super::{
    build_column_map, find_data_range, get_field_def, is_truthy, known_field_key_set,
    route_extra_columns, row_to_map,
};

/// Read the People sheet and populate the schedule with Presenter entities.
///
/// Presenters created here get their rank, `is_explicit_group`,
/// `always_grouped`, and `always_shown_in_group` flags set from the explicit
/// People sheet columns.  Any presenter already in the schedule (created by a
/// prior pass or earlier in the sheet) is updated if the People sheet carries
/// a higher-priority rank.
pub(super) fn read_people_into(
    book: &Spreadsheet,
    preferred: &str,
    schedule: &mut Schedule,
    file_path: Option<&str>,
    import_time: DateTime<Utc>,
) -> Result<()> {
    let range = match find_data_range(
        book,
        preferred,
        &["Presenters", "Presenter", "People", "Person"],
    ) {
        Some(r) => r,
        None => return Ok(()),
    };

    let ws = match book.get_sheet_by_name(&range.sheet_name) {
        Some(ws) => ws,
        None => return Ok(()),
    };

    if !range.has_data() {
        return Ok(());
    }

    let (raw_headers, canonical_headers, _col_map) = build_column_map(ws, &range);
    let known_keys = known_field_key_set(pc::ALL, &[]);

    for row in (range.header_row + 1)..=range.end_row {
        let data = row_to_map(ws, row, &range, &raw_headers, &canonical_headers);

        let name = match get_field_def(&data, &pc::NAME) {
            Some(n) if !n.trim().is_empty() => n.trim().to_string(),
            _ => continue,
        };

        let rank = get_field_def(&data, &pc::CLASSIFICATION)
            .map(|s| parse_classification(s))
            .unwrap_or_default();

        let is_explicit_group = get_field_def(&data, &pc::IS_GROUP)
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let always_grouped = get_field_def(&data, &pc::ALWAYS_GROUPED)
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        let always_shown = get_field_def(&data, &pc::ALWAYS_SHOWN)
            .map(|s| is_truthy(s))
            .unwrap_or(false);

        if let Some(existing_id) = PresenterEntityType::find_by_name(schedule, &name) {
            // Update flags and rank if People table has higher priority.
            let existing_rank = schedule
                .get_internal::<PresenterEntityType>(existing_id)
                .map(|d| d.data.rank.clone())
                .unwrap_or_default();

            let mut updates: Vec<FieldUpdate<PresenterEntityType>> = vec![
                FieldUpdate::set(&presenter::FIELD_IS_EXPLICIT_GROUP, is_explicit_group),
                FieldUpdate::set(&presenter::FIELD_ALWAYS_GROUPED, always_grouped),
                FieldUpdate::set(&presenter::FIELD_ALWAYS_SHOWN_IN_GROUP, always_shown),
            ];
            // People table is authoritative: upgrade rank if it has higher priority.
            if rank.priority() < existing_rank.priority() {
                updates.push(FieldUpdate::set(&presenter::FIELD_RANK, rank.as_str()));
            }
            let _ =
                PresenterEntityType::field_set().write_multiple(existing_id, schedule, &updates);
        } else {
            // Create new presenter entity.
            let uuid_pref = UuidPreference::PreferFromV5 {
                name: name.to_lowercase(),
            };
            let updates = vec![
                FieldUpdate::set(&presenter::FIELD_NAME, name.as_str()),
                FieldUpdate::set(&presenter::FIELD_RANK, rank.as_str()),
                FieldUpdate::set(&presenter::FIELD_IS_EXPLICIT_GROUP, is_explicit_group),
                FieldUpdate::set(&presenter::FIELD_ALWAYS_GROUPED, always_grouped),
                FieldUpdate::set(&presenter::FIELD_ALWAYS_SHOWN_IN_GROUP, always_shown),
            ];
            match build_entity::<PresenterEntityType>(schedule, uuid_pref, updates) {
                Ok(id) => {
                    let uuid = id.entity_uuid();
                    schedule.sidecar_mut().set_origin(
                        uuid,
                        EntityOrigin::Xlsx(XlsxSourceInfo {
                            file_path: file_path.map(str::to_owned),
                            sheet_name: range.sheet_name.clone(),
                            row_index: row,
                            import_time,
                        }),
                    );
                    // Column 0 = People sheet; row gives relative order.
                    schedule.sidecar_mut().get_or_insert(uuid).xlsx_sort_key = Some((0, row));
                    route_extra_columns(
                        ws,
                        row,
                        &range,
                        &raw_headers,
                        &canonical_headers,
                        &known_keys,
                        &[],
                        id.entity_uuid(),
                        PresenterEntityType::TYPE_NAME,
                        schedule,
                    );
                }
                Err(e) => eprintln!("xlsx import: skipping presenter {name:?}: {e}"),
            }
        }
    }

    Ok(())
}

/// Map the People sheet's `Classification` column value to a `PresenterRank`.
///
/// Common values from actual Cosplay America spreadsheets:
/// - "Guest", "GOH"                → Guest
/// - "Judge"                       → Judge
/// - "Staff"                       → Staff
/// - "Invited", "Invited Panelist", "Industry Panelist" → InvitedGuest(None)
/// - "Fan", "Fan Panelist"         → FanPanelist
/// - "Panelist"                    → Panelist
/// - anything else (e.g. "Sponsor") → InvitedGuest(Some(label))
fn parse_classification(s: &str) -> PresenterRank {
    match s.trim().to_lowercase().as_str() {
        "guest" | "goh" | "guest of honor" => PresenterRank::Guest,
        "judge" => PresenterRank::Judge,
        "staff" => PresenterRank::Staff,
        "invited" | "invited panelist" | "invited_panelist" | "industry panelist"
        | "industry_panelist" | "invitedpanelist" => PresenterRank::InvitedGuest(None),
        "panelist" => PresenterRank::Panelist,
        "fan" | "fan panelist" | "fan_panelist" | "fanpanelist" => PresenterRank::FanPanelist,
        _ => PresenterRank::parse(s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_classification_standard() {
        assert_eq!(parse_classification("Guest"), PresenterRank::Guest);
        assert_eq!(parse_classification("GOH"), PresenterRank::Guest);
        assert_eq!(parse_classification("Judge"), PresenterRank::Judge);
        assert_eq!(parse_classification("Staff"), PresenterRank::Staff);
        assert_eq!(
            parse_classification("Invited"),
            PresenterRank::InvitedGuest(None)
        );
        assert_eq!(
            parse_classification("Industry Panelist"),
            PresenterRank::InvitedGuest(None)
        );
        assert_eq!(parse_classification("Panelist"), PresenterRank::Panelist);
        assert_eq!(parse_classification("Fan"), PresenterRank::FanPanelist);
        assert_eq!(
            parse_classification("Fan Panelist"),
            PresenterRank::FanPanelist
        );
    }

    #[test]
    fn test_parse_classification_custom() {
        // Unknown values become custom InvitedGuest labels.
        assert_eq!(
            parse_classification("Sponsor"),
            PresenterRank::InvitedGuest(Some("Sponsor".into()))
        );
    }
}
