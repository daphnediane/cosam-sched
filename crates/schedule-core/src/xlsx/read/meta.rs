/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Reads the metadata sheet → schedule-level [`ScheduleMetadata`] fields.

use anyhow::Result;

use crate::value::timezone::parse_tz;
use crate::xlsx::meta::{self as meta_keys, MetaField};
use crate::xlsx::read::TableImportMode;

use super::{find_data_range, get_cell_datetime, get_cell_str};

impl super::ImportContext<'_> {
    /// Read the metadata table and apply recognized columns to the schedule
    /// metadata (timezone, start/end window).
    ///
    /// Matches the canonical Google `Timestamp` table (and our exported `Meta`
    /// table): a header row naming fields, then a data row of values. Columns
    /// are matched by [`meta_keys::classify_header`]; unrecognized columns —
    /// including the legacy, no-longer-authoritative `Last Change Added` — are
    /// ignored. A missing table is a no-op.
    ///
    /// Each applied value is authoritative (it is what was last published), so
    /// callers layering CLI defaults should only fill fields left unset here.
    pub(super) fn read_meta(&mut self) -> Result<()> {
        // No per-table import option; always look for it by name (table first,
        // then sheet) across the recognized names.
        let range = match find_data_range(
            self.book,
            self.csv_map,
            &TableImportMode::Process,
            meta_keys::TABLE_NAMES,
        ) {
            Some(r) => r,
            None => return Ok(()),
        };

        let ws = match self.book.get_sheet_by_name(&range.sheet_name) {
            Some(ws) => ws,
            None => return Ok(()),
        };

        if !range.has_data() {
            return Ok(());
        }

        // Map each column to the field it carries (if recognized).
        let mut cols: Vec<(u32, MetaField)> = Vec::new();
        for col in range.start_col..=range.end_col {
            if let Some(h) = get_cell_str(ws, col, range.header_row) {
                if let Some(field) = meta_keys::classify_header(&h) {
                    cols.push((col, field));
                }
            }
        }

        // Apply values, first non-empty data row wins for each field. Start/end
        // accept both ISO text and Excel serial-number date cells.
        for row in (range.header_row + 1)..=range.end_row {
            for &(col, field) in &cols {
                match field {
                    MetaField::Timezone => {
                        if self.schedule.metadata.timezone.is_none() {
                            if let Some(tz) = get_cell_str(ws, col, row).and_then(|v| parse_tz(&v))
                            {
                                self.schedule.metadata.timezone = Some(tz.name().to_string());
                            }
                        }
                    }
                    MetaField::StartTime => {
                        if self.schedule.metadata.start_time.is_none() {
                            self.schedule.metadata.start_time = get_cell_datetime(ws, col, row);
                        }
                    }
                    MetaField::EndTime => {
                        if self.schedule.metadata.end_time.is_none() {
                            self.schedule.metadata.end_time = get_cell_datetime(ws, col, row);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
