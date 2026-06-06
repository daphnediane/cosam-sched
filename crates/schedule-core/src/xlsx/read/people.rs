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
//! Every presenter — the row's own entity and any entity named in a `Members`
//! or `Groups` cell — is created through the single public
//! [`find_or_create_tagged_presenter`] API, which assigns a deterministic v5
//! UUID to new entities so re-imports and merges stay stable.  The `Members`
//! (group row) and `Groups` (member row) columns each contribute one side of a
//! membership edge; edges are deduplicated so declaring membership in both
//! places is safe.
//!
//! Rank is contributed as a [`RankSource`] claim: the `Classification` column
//! and a membership entry's own tag prefix are [`RankSource::Declared`], while
//! an untagged entry inherits the declaring row's classification as
//! [`RankSource::Implied`].  The import cache reconciles these claims with the
//! stored rank at flush time.

use anyhow::Result;

use crate::entity::{EntityType, EntityUuid};
use crate::field::set::FieldUpdate;
use crate::sidecar::{EntityOrigin, XlsxSourceInfo};
use crate::tables::presenter::{
    self, find_or_create_tagged_presenter, tag_prefix_rank, PresenterEntityType, PresenterId,
    PresenterRank, RankSource, EDGE_GROUPS,
};
use crate::xlsx::columns::people as pc;

use super::{
    build_column_map, find_data_range, get_field_def, is_truthy, known_field_key_set,
    route_extra_columns, row_to_map,
};

impl super::ImportContext<'_> {
    /// Read the People sheet and populate the schedule with Presenter entities.
    ///
    /// Each row resolves its presenter through [`find_or_create_tagged_presenter`]
    /// (so new entities get deterministic v5 UUIDs), sets the explicit
    /// `is_explicit_group` / `subsumes_members` / `show_individually` flags from
    /// the People-sheet columns, and records a [`RankSource`] claim for the row's
    /// `Classification`.  `Members` and `Groups` cells then link membership edges,
    /// each listed entry contributing its own rank claim.
    ///
    /// Accumulates seen presenter UUIDs into `self.seen_presenters`.
    pub(super) fn read_people(&mut self) -> Result<()> {
        let mode = self.options.people.clone();

        let range = match find_data_range(
            self.book,
            self.csv_map,
            &mode,
            &["Presenters", "Presenter", "People", "Person"],
        ) {
            Some(r) => r,
            None => return Ok(()),
        };

        if self.book.get_sheet_by_name(&range.sheet_name).is_none() || !range.has_data() {
            return Ok(());
        }

        // `ws` borrows `self.book`, so it is fetched in tight scopes that never
        // overlap the `&mut self` record/membership helpers below.
        let (raw_headers, canonical_headers, _col_map) = {
            let ws = self.sheet(&range.sheet_name);
            build_column_map(ws, &range)
        };
        let known_keys = known_field_key_set(pc::ALL, &[]);

        for row in (range.header_row + 1)..=range.end_row {
            let data = {
                let ws = self.sheet(&range.sheet_name);
                row_to_map(ws, row, &range, &raw_headers, &canonical_headers)
            };

            let name = match get_field_def(&data, &pc::NAME) {
                Some(n) if !n.trim().is_empty() => n.trim().to_string(),
                _ => continue,
            };

            // Classification column → a Declared rank claim for this row's
            // presenter; absent means no rank information from this row.
            let classification =
                get_field_def(&data, &pc::CLASSIFICATION).map(|s| parse_classification(s));
            let row_source = classification
                .clone()
                .map_or(RankSource::None, RankSource::Declared);

            let members = get_field_def(&data, &pc::MEMBERS).filter(|s| !s.trim().is_empty());
            let groups = get_field_def(&data, &pc::GROUPS).filter(|s| !s.trim().is_empty());

            // A row is a group if its column says so or it lists members.
            let is_group = members.is_some()
                || get_field_def(&data, &pc::IS_GROUP)
                    .map(|s| is_truthy(s))
                    .unwrap_or(false);
            let subsumes_members = get_field_def(&data, &pc::SUBSUMES_MEMBERS)
                .map(|s| is_truthy(s))
                .unwrap_or(false);
            let show_individually = get_field_def(&data, &pc::SHOW_INDIVIDUALLY)
                .map(|s| is_truthy(s))
                .unwrap_or(false);

            // One creation path for every presenter: the public tagged API,
            // which assigns a deterministic v5 UUID to new entities.
            let id = match find_or_create_tagged_presenter(self.schedule, &name) {
                Ok(m) => m.as_presenter(),
                Err(e) => {
                    eprintln!("xlsx import: skipping presenter {name:?}: {e}");
                    continue;
                }
            };

            // People-sheet columns are authoritative for these flags.
            let _ = PresenterEntityType::field_set().write_multiple(
                id,
                self.schedule,
                &[
                    FieldUpdate::set(&presenter::FIELD_IS_EXPLICIT_GROUP, is_group),
                    FieldUpdate::set(&presenter::FIELD_SHOW_INDIVIDUALLY, show_individually),
                    FieldUpdate::set(&presenter::FIELD_SUBSUMES_MEMBERS, subsumes_members),
                ],
            );

            self.record_presenter(id, &name, row_source, (0, row, 0));

            // Set origin + route extra columns the first time this presenter is
            // seen from its own People row (members listed elsewhere get no
            // origin until their own row is reached).
            let uuid = id.entity_uuid();
            let needs_origin = self
                .schedule
                .sidecar()
                .get(uuid)
                .is_none_or(|e| e.origin.is_none());
            if needs_origin {
                self.schedule.sidecar_mut().set_origin(
                    uuid,
                    EntityOrigin::Xlsx(XlsxSourceInfo {
                        file_path: self.file_path.map(str::to_owned),
                        sheet_name: range.sheet_name.clone(),
                        row_index: row,
                        import_time: self.import_time,
                    }),
                );
                let ws = self.book.get_sheet_by_name(&range.sheet_name).expect("sheet present");
                route_extra_columns(
                    ws,
                    row,
                    &range,
                    &raw_headers,
                    &canonical_headers,
                    &known_keys,
                    &[],
                    &std::collections::HashSet::new(),
                    uuid,
                    PresenterEntityType::TYPE_NAME,
                    self.schedule,
                );
            }

            // Members column: each listed entry is a member of this (group) row.
            if let Some(members) = members {
                for (sub, entry) in split_names_for_membership(members).into_iter().enumerate() {
                    self.import_member(&entry, id, classification.as_ref(), (0, row, sub as u32 + 1));
                }
            }

            // Groups column: each listed entry is a group this row belongs to.
            if let Some(groups) = groups {
                for (sub, entry) in split_names_for_membership(groups).into_iter().enumerate() {
                    self.import_group(id, &entry, classification.as_ref(), (0, row, sub as u32 + 1));
                }
            }
        }

        Ok(())
    }

    /// Record a presenter encounter for the import: cache its rank claim and
    /// canonical name, mark it seen, and note its XLSX sort key.  The lowest
    /// `(column, row, sub_column)` key wins, so a primary entry (`sub_column = 0`)
    /// beats any secondary appearance of the same presenter.
    fn record_presenter(
        &mut self,
        id: PresenterId,
        name: &str,
        source: RankSource,
        key: (u32, u32, u32),
    ) {
        self.presenter_cache.record(id, name, source);
        let uuid = id.entity_uuid();
        self.seen_presenters.insert(uuid);
        let sidecar = self.schedule.sidecar_mut().get_or_insert(uuid);
        if sidecar.xlsx_sort_key.is_none_or(|cur| key < cur) {
            sidecar.xlsx_sort_key = Some(key);
        }
    }

    /// Resolve (creating if needed) a `Members`-cell `entry` and link it as a
    /// member of `group_id`.  `inherited` supplies the implied rank when the
    /// entry carries no tag prefix of its own.
    fn import_member(
        &mut self,
        entry: &str,
        group_id: PresenterId,
        inherited: Option<&PresenterRank>,
        key: (u32, u32, u32),
    ) {
        let member_id = match find_or_create_tagged_presenter(self.schedule, entry) {
            Ok(m) => m.as_presenter(),
            Err(e) => {
                eprintln!("xlsx import: skipping member {entry:?}: {e}");
                return;
            }
        };
        let name = self.presenter_name(member_id, entry);
        self.record_presenter(member_id, &name, entry_rank_source(entry, inherited), key);
        self.link_member_to_group(member_id, group_id);
    }

    /// Resolve (creating if needed) a `Groups`-cell `entry` as a group that
    /// `member_id` belongs to.  A leading `==` marks the group as subsuming its
    /// members.  `inherited` supplies the implied rank when the entry carries no
    /// tag prefix of its own.
    fn import_group(
        &mut self,
        member_id: PresenterId,
        entry: &str,
        inherited: Option<&PresenterRank>,
        key: (u32, u32, u32),
    ) {
        let (clean, subsumes) = split_subsumes(entry);
        let group_id = match find_or_create_tagged_presenter(self.schedule, clean) {
            Ok(g) => g.as_presenter(),
            Err(e) => {
                eprintln!("xlsx import: skipping group {entry:?}: {e}");
                return;
            }
        };

        // The named entry is an explicit group; honour a leading `==` marker.
        let mut flags = vec![FieldUpdate::set(&presenter::FIELD_IS_EXPLICIT_GROUP, true)];
        if subsumes {
            flags.push(FieldUpdate::set(&presenter::FIELD_SUBSUMES_MEMBERS, true));
        }
        let _ = PresenterEntityType::field_set().write_multiple(group_id, self.schedule, &flags);

        let name = self.presenter_name(group_id, clean);
        self.record_presenter(group_id, &name, entry_rank_source(clean, inherited), key);
        self.link_member_to_group(member_id, group_id);
    }

    /// Add the `member → group` membership edge if it is not already present.
    fn link_member_to_group(&mut self, member_id: PresenterId, group_id: PresenterId) {
        let already = self
            .schedule
            .connected_field_nodes(member_id, EDGE_GROUPS)
            .into_iter()
            .any(|e| e.entity_uuid() == group_id.entity_uuid());
        if !already {
            let _ = self
                .schedule
                .edge_add(member_id, EDGE_GROUPS, std::iter::once(group_id));
        }
    }

    /// Borrow the named worksheet.  Panics only if called after the existence
    /// check at the top of [`Self::read_people`] (the sheet is known present).
    fn sheet(&self, name: &str) -> &umya_spreadsheet::structs::Worksheet {
        self.book.get_sheet_by_name(name).expect("sheet present")
    }

    /// The stored canonical name for `id`, falling back to the trimmed `raw`
    /// credit string when the entity cannot be read.
    fn presenter_name(&self, id: PresenterId, raw: &str) -> String {
        self.schedule
            .get_internal::<PresenterEntityType>(id)
            .map(|d| d.data.name.clone())
            .unwrap_or_else(|| raw.trim().to_string())
    }
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

/// The rank claim for a `Members`/`Groups` entry: the entry's own tag prefix is
/// [`RankSource::Declared`]; an untagged entry inherits `inherited` (the
/// declaring row's classification) as [`RankSource::Implied`].
fn entry_rank_source(entry: &str, inherited: Option<&PresenterRank>) -> RankSource {
    match tag_prefix_rank(entry) {
        Some(r) => RankSource::Declared(r),
        None => inherited
            .cloned()
            .map_or(RankSource::None, RankSource::Implied),
    }
}

/// Split a leading `==` (subsumes-members marker) off a `Groups`-cell entry.
/// Returns the cleaned entry and whether the marker was present.
fn split_subsumes(entry: &str) -> (&str, bool) {
    match entry.trim().strip_prefix("==") {
        Some(rest) => (rest.trim(), true),
        None => (entry.trim(), false),
    }
}

/// Split a comma-separated membership list from the People sheet.
///
/// Uses a simple comma split (not the full presenter-name splitter, since
/// membership lists use only commas and names may contain "and").
fn split_names_for_membership(text: &str) -> Vec<String> {
    text.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_names_for_membership_basic() {
        assert_eq!(
            split_names_for_membership("Alice, Bob, Charlie"),
            vec!["Alice", "Bob", "Charlie"]
        );
        assert_eq!(
            split_names_for_membership("  Pro  ,  Con  "),
            vec!["Pro", "Con"]
        );
        assert_eq!(split_names_for_membership(""), Vec::<String>::new());
    }

    #[test]
    fn test_split_subsumes() {
        assert_eq!(split_subsumes("==My Group"), ("My Group", true));
        assert_eq!(split_subsumes("  ==Band "), ("Band", true));
        assert_eq!(split_subsumes("Band"), ("Band", false));
        assert_eq!(split_subsumes("=Band"), ("=Band", false));
    }

    #[test]
    fn test_entry_rank_source_prefix_is_declared() {
        assert_eq!(
            entry_rank_source("S:Bob", Some(&PresenterRank::Guest)),
            RankSource::Declared(PresenterRank::Staff)
        );
    }

    #[test]
    fn test_entry_rank_source_untagged_inherits_implied() {
        assert_eq!(
            entry_rank_source("Bob", Some(&PresenterRank::Guest)),
            RankSource::Implied(PresenterRank::Guest)
        );
        assert_eq!(entry_rank_source("Bob", None), RankSource::None);
    }

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
