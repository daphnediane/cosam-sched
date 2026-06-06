/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! XLSX import: shared utilities and the top-level [`import_xlsx`] entry point.

pub mod headers;
mod hotel_rooms;
mod panel_types;
mod people;
mod rooms;
mod schedule;
mod timeline;

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use umya_spreadsheet::structs::Worksheet;
use umya_spreadsheet::Spreadsheet;
use uuid::NonNilUuid;

use crate::entity::{EntityId, EntityType, EntityUuid};
use crate::field::set::FieldUpdate;
use crate::schedule::Schedule;
use crate::sidecar::SidecarFormulaField;
use crate::tables::event_room::{EventRoomEntityType, EventRoomId};
use crate::tables::hotel_room::{HotelRoomEntityType, HotelRoomId};
use crate::tables::panel::PanelEntityType;
use crate::tables::panel_type::{PanelTypeEntityType, PanelTypeId};
use crate::tables::presenter::{self, PresenterEntityType, PresenterId, RankSource};
use crate::tables::timeline::TimelineEntityType;
use crate::xlsx::columns::{FieldDef, FormulaColumnDef};

pub use headers::canonical_header;
pub(crate) use headers::{parse_presenter_header, PresenterColumn, PresenterHeader};

// ── Import options ────────────────────────────────────────────────────────────

/// Mode for processing a sheet/table during import or export.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TableImportMode {
    /// Process using the default sheet/table name.
    #[default]
    Process,
    /// Process using a custom sheet/table name.
    ReadFrom(String),
    /// Skip processing this sheet/table.
    Skip,
}

impl TableImportMode {
    /// Get the effective name to use, or None if skipped.
    pub fn effective_name(&self, default: &str) -> Option<String> {
        match self {
            TableImportMode::Process => Some(default.to_string()),
            TableImportMode::ReadFrom(name) => Some(name.clone()),
            TableImportMode::Skip => None,
        }
    }
}

/// Common options controlling which tables/sheets are read during import.
/// Used by both XLSX and CSV import.
#[derive(Debug, Clone, Default)]
pub struct TableImportOptions {
    /// Mode for panel data (default: Process).
    pub schedule: TableImportMode,
    /// Mode for rooms (default: Process).
    pub rooms: TableImportMode,
    /// Mode for panel types (default: Process).
    pub panel_types: TableImportMode,
    /// Mode for the People/Presenters table (default: Process).
    pub people: TableImportMode,
    /// Mode for hotel rooms (default: Process).
    pub hotel_rooms: TableImportMode,
    /// Mode for timelines (default: Process).
    pub timeline: TableImportMode,
}

/// Type alias for XLSX import options (for backward compatibility).
pub type XlsxImportOptions = TableImportOptions;

// ── Import context ───────────────────────────────────────────────────────────────

/// Context structure holding all state for a single XLSX/CSV import pass.
///
/// Encapsulates both the read-side (spreadsheet, file metadata) and the
/// write-side (schedule, options, inter-stage lookups) so that reader
/// methods share a single mutable context rather than receiving a long
/// list of parameters.
pub struct ImportContext<'a> {
    /// The spreadsheet being imported (mutated as CSV files are imported as sheets).
    pub book: &'a mut Spreadsheet,
    /// Optional file path for origin tracking.
    pub file_path: Option<&'a str>,
    /// Timestamp when the import began.
    pub import_time: chrono::DateTime<chrono::Utc>,
    /// Optional CSV file mapping for directory import mode.
    pub csv_map: &'a Option<CsvFileMap>,
    /// The schedule being populated.
    pub schedule: &'a mut Schedule,
    /// Per-table import mode options.
    pub options: &'a TableImportOptions,
    /// Presenter name/rank cache — populated during the pass, flushed by `finalize`.
    pub presenter_cache: PresenterImportCache,
    /// prefix → PanelTypeId; populated by `read_panel_types`.
    pub panel_type_lookup: HashMap<String, PanelTypeId>,
    /// lowercase name → EventRoomId; populated by `read_rooms`.
    pub room_lookup: HashMap<String, EventRoomId>,
    /// lowercase name → HotelRoomId; populated by `read_hotel_rooms`.
    pub hotel_lookup: HashMap<String, HotelRoomId>,
    // ── Before-snapshots (captured at construction) ──────────────────────────
    /// Entity UUIDs present before the import began, per type.
    pub before_panel_types: HashSet<NonNilUuid>,
    pub before_hotel_rooms: HashSet<NonNilUuid>,
    pub before_rooms: HashSet<NonNilUuid>,
    pub before_presenters: HashSet<NonNilUuid>,
    pub before_panels: HashSet<NonNilUuid>,
    pub before_timelines: HashSet<NonNilUuid>,
    // ── Seen-accumulators (populated by read_* methods) ──────────────────────
    /// Entity UUIDs observed during this import pass, per type.
    pub seen_panel_types: HashSet<NonNilUuid>,
    pub seen_hotel_rooms: HashSet<NonNilUuid>,
    pub seen_rooms: HashSet<NonNilUuid>,
    pub seen_presenters: HashSet<NonNilUuid>,
    pub seen_panels: HashSet<NonNilUuid>,
    pub seen_timelines: HashSet<NonNilUuid>,
}

impl<'a> ImportContext<'a> {
    /// Create a new ImportContext for a single import pass.
    ///
    /// Snapshots the current entity UUIDs from `schedule` into the `before_*` fields
    /// so that `finalize` can soft-delete entities removed by the import.
    pub fn new(
        book: &'a mut Spreadsheet,
        file_path: Option<&'a str>,
        import_time: chrono::DateTime<chrono::Utc>,
        csv_map: &'a Option<CsvFileMap>,
        schedule: &'a mut Schedule,
        options: &'a TableImportOptions,
    ) -> Self {
        let before_panel_types = collect_entity_uuids::<PanelTypeEntityType>(schedule);
        let before_hotel_rooms = collect_entity_uuids::<HotelRoomEntityType>(schedule);
        let before_rooms = collect_entity_uuids::<EventRoomEntityType>(schedule);
        let before_presenters = collect_entity_uuids::<PresenterEntityType>(schedule);
        let before_panels = collect_entity_uuids::<PanelEntityType>(schedule);
        let before_timelines = collect_entity_uuids::<TimelineEntityType>(schedule);

        Self {
            book,
            file_path,
            import_time,
            csv_map,
            schedule,
            options,
            presenter_cache: PresenterImportCache::new(),
            panel_type_lookup: HashMap::new(),
            room_lookup: HashMap::new(),
            hotel_lookup: HashMap::new(),
            before_panel_types,
            before_hotel_rooms,
            before_rooms,
            before_presenters,
            before_panels,
            before_timelines,
            seen_panel_types: HashSet::new(),
            seen_hotel_rooms: HashSet::new(),
            seen_rooms: HashSet::new(),
            seen_presenters: HashSet::new(),
            seen_panels: HashSet::new(),
            seen_timelines: HashSet::new(),
        }
    }

    /// Finalize the import: normalize sort indices, flush the presenter cache,
    /// then soft-delete entities that were present before the import but not seen
    /// during it.
    ///
    /// Call after all `read_*` methods have completed.
    ///
    /// Returns an error if validation detects that entities were created during
    /// import but not tracked in the seen_* sets. Note that the schedule may be
    /// in a partially modified state when this error occurs.
    pub(super) fn finalize(&mut self) -> Result<()> {
        // Flush presenter cache: write canonical names and explicit ranks.
        // Sort by UUID for deterministic write order (ensures idempotent imports).
        let mut cache_entries: Vec<_> = self.presenter_cache.entries.drain().collect();
        cache_entries.sort_by_key(|(a, _)| *a);
        for (uuid, (claim, canonical_name)) in cache_entries {
            // SAFETY: uuid was obtained from a live PresenterId during this pass.
            let id = unsafe { PresenterId::new_unchecked(uuid) };

            // Reconcile the pass's accumulated claim with the stored rank.  A
            // `Declared` claim from the file is authoritative (it may lower a
            // previously stored rank); `Implied`/`None` claims promote but never
            // lower a stored declaration.
            let stored = self
                .schedule
                .get_internal::<PresenterEntityType>(id)
                .map(|d| d.data.rank.clone())
                .unwrap_or_default();
            let resolved = match claim {
                RankSource::Declared(_) => claim,
                other => stored.resolve(other),
            };

            let updates = vec![
                FieldUpdate::set(&presenter::FIELD_NAME, canonical_name.as_str()),
                FieldUpdate::set(&presenter::FIELD_RANK, resolved.as_field_str().as_str()),
            ];
            let _ = PresenterEntityType::field_set().write_multiple(id, self.schedule, &updates);
        }

        // Validate that all entities are properly tracked in before/seen sets.
        // This detects bugs where entities are created during import but not
        // added to the corresponding seen_* field.
        validate_seen_tracking::<PanelTypeEntityType>(
            self.schedule,
            &self.before_panel_types,
            &self.seen_panel_types,
            "PanelType",
        )?;
        validate_seen_tracking::<HotelRoomEntityType>(
            self.schedule,
            &self.before_hotel_rooms,
            &self.seen_hotel_rooms,
            "HotelRoom",
        )?;
        validate_seen_tracking::<EventRoomEntityType>(
            self.schedule,
            &self.before_rooms,
            &self.seen_rooms,
            "EventRoom",
        )?;
        validate_seen_tracking::<PresenterEntityType>(
            self.schedule,
            &self.before_presenters,
            &self.seen_presenters,
            "Presenter",
        )?;
        validate_seen_tracking::<PanelEntityType>(
            self.schedule,
            &self.before_panels,
            &self.seen_panels,
            "Panel",
        )?;
        validate_seen_tracking::<TimelineEntityType>(
            self.schedule,
            &self.before_timelines,
            &self.seen_timelines,
            "Timeline",
        )?;

        // Soft-delete entities not seen in this import.
        // Only soft-delete for entity types whose sheet was processed.
        if self.options.panel_types != TableImportMode::Skip {
            soft_delete_unseen::<PanelTypeEntityType>(
                self.schedule,
                &self.before_panel_types,
                &self.seen_panel_types,
            );
        }

        // Hotel rooms: combined from the Hotels sheet and inline Rooms sheet columns.
        if self.options.hotel_rooms != TableImportMode::Skip
            || self.options.rooms != TableImportMode::Skip
        {
            soft_delete_unseen::<HotelRoomEntityType>(
                self.schedule,
                &self.before_hotel_rooms,
                &self.seen_hotel_rooms,
            );
        }

        if self.options.rooms != TableImportMode::Skip {
            soft_delete_unseen::<EventRoomEntityType>(
                self.schedule,
                &self.before_rooms,
                &self.seen_rooms,
            );
        }

        // Presenters: combined from People sheet and Schedule sheet presenter columns.
        if self.options.schedule != TableImportMode::Skip {
            soft_delete_unseen::<PresenterEntityType>(
                self.schedule,
                &self.before_presenters,
                &self.seen_presenters,
            );
        }

        // Timelines: from the Timeline sheet and timeline-type rows on the Schedule sheet.
        if self.options.timeline != TableImportMode::Skip
            || self.options.schedule != TableImportMode::Skip
        {
            soft_delete_unseen::<TimelineEntityType>(
                self.schedule,
                &self.before_timelines,
                &self.seen_timelines,
            );
        }

        if self.options.schedule != TableImportMode::Skip {
            soft_delete_unseen::<PanelEntityType>(
                self.schedule,
                &self.before_panels,
                &self.seen_panels,
            );
        }

        Ok(())
    }
}

// ── CSV file mapping for directory import ───────────────────────────────────────

/// Mapping from lowercase sheet names (without extension) to full CSV/TXT file paths.
/// Used when importing a directory of CSV files via xlsx import.
#[derive(Debug, Clone)]
pub struct CsvFileMap {
    /// Map of lowercase names (e.g., "schedule") to full file paths (e.g., "/path/to/schedule.csv")
    files: HashMap<String, String>,
}

impl CsvFileMap {
    /// Scan a directory for CSV and TXT files and build a mapping.
    pub fn from_directory(dir_path: &Path) -> Result<Self> {
        let mut files = HashMap::new();

        let entries = fs::read_dir(dir_path)
            .with_context(|| format!("Failed to read directory {}", dir_path.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Only process regular files
            if !path.is_file() {
                continue;
            }

            // Check for .csv or .txt extension
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());

            if extension.as_deref() != Some("csv") && extension.as_deref() != Some("txt") {
                continue;
            }

            // Get the filename without extension as the key (lowercase)
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid filename: {}", path.display()))?;

            let key = stem.to_lowercase();
            files.insert(key, path.to_string_lossy().to_string());
        }

        Ok(CsvFileMap { files })
    }

    /// Get the full file path for a given lowercase sheet name.
    pub fn get(&self, name: &str) -> Option<&String> {
        self.files.get(&name.to_lowercase())
    }

    /// Check if a file exists for the given name.
    pub fn contains(&self, name: &str) -> bool {
        self.files.contains_key(&name.to_lowercase())
    }
}

/// Import a CSV file into a spreadsheet as a new sheet.
fn import_csv_to_sheet(book: &mut Spreadsheet, csv_path: &Path, sheet_name: &str) -> Result<()> {
    // Detect CSV format
    let format = crate::csv::read::detect_csv_format(csv_path)?;

    // Read the CSV file
    let mut file =
        File::open(csv_path).with_context(|| format!("Failed to open {}", csv_path.display()))?;

    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .with_context(|| format!("Failed to read {}", csv_path.display()))?;

    // Convert encoding if needed
    let content_str = match format {
        crate::csv::read::CsvFormat::Utf16Tab => {
            let (decoded, _, _) = encoding_rs::UTF_16LE.decode(&content);
            decoded.to_string()
        }
        crate::csv::read::CsvFormat::Utf8Tab | crate::csv::read::CsvFormat::Utf8Comma => {
            String::from_utf8(content)
                .with_context(|| format!("Failed to decode {} as UTF-8", csv_path.display()))?
        }
    };

    // Parse CSV content
    let delimiter = match format {
        crate::csv::read::CsvFormat::Utf8Tab | crate::csv::read::CsvFormat::Utf16Tab => b'\t',
        crate::csv::read::CsvFormat::Utf8Comma => b',',
    };

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_reader(content_str.as_bytes());

    // Create a new sheet
    let _ = book.new_sheet(sheet_name);

    // Get the sheet (it should be the last one added)
    let sheet_index = book.get_sheet_count() - 1;
    let sheet = book
        .get_sheet_mut(&sheet_index)
        .ok_or_else(|| anyhow::anyhow!("Failed to get sheet {}", sheet_name))?;

    // Write headers (row 1)
    let headers = rdr.headers()?;
    for (col_idx, header) in headers.iter().enumerate() {
        let col = (col_idx + 1) as u32;
        sheet.get_cell_mut((col, 1)).set_value(header);
    }

    // Write data rows (starting from row 2)
    for (row_idx, result) in (2u32..).zip(rdr.records()) {
        let record = result?;
        for (col_idx, value) in record.iter().enumerate() {
            let col = (col_idx + 1) as u32;
            sheet.get_cell_mut((col, row_idx)).set_value(value);
        }
    }

    Ok(())
}

// ── Public entry points ───────────────────────────────────────────────────────

// ── Presenter import cache ────────────────────────────────────────────────────

/// Per-import-pass cache for presenter name and rank resolution.
///
/// Collects the canonical name (first-seen spelling, People sheet wins) and the
/// best *explicit* rank seen for each presenter during a single import pass.
/// Rank and name are **not** written inline; call [`PresenterImportCache::flush`]
/// after all sheets are processed to apply them in the same cycle as soft-delete.
///
/// **Rank tiers ([`RankSource`]):**
/// Each encounter contributes a [`RankSource`] claim describing both the rank
/// and the authority behind it:
/// - Named presenter columns and People-sheet `Classification` values are
///   [`RankSource::Declared`].
/// - `Other` cells / membership entries with a tag prefix are `Declared`;
///   without one they are [`RankSource::Implied`] (inherited rank) or
///   [`RankSource::None`] (no rank information).
///
/// Within a pass, claims accumulate via [`RankSource::resolve`] (higher tier
/// wins; equal tier promotes to the higher rank).  At flush the accumulated
/// claim is reconciled with the stored value: a `Declared` claim from the file
/// is authoritative and may lower a previously stored rank, while `Implied` /
/// `None` claims promote but never lower a stored declaration.  Only presenters
/// present in the new file are affected; absent presenters keep their data.
#[derive(Default)]
pub struct PresenterImportCache {
    /// Maps presenter UUID → (accumulated rank claim this pass, canonical name).
    entries: HashMap<uuid::NonNilUuid, (RankSource, String)>,
}

impl PresenterImportCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Record a presenter encounter for this import pass.
    ///
    /// `source` is the rank claim for this encounter (see [`RankSource`]); pass
    /// [`RankSource::None`] for an encounter that carries no rank information but
    /// should still guarantee name correction at flush.  The canonical name from
    /// the first encounter is preserved; the rank claim accumulates across
    /// encounters via [`RankSource::resolve`].
    pub fn record(&mut self, id: PresenterId, name: &str, source: RankSource) {
        let uuid = id.entity_uuid();
        match self.entries.get_mut(&uuid) {
            Some((claim, _)) => {
                *claim = std::mem::take(claim).resolve(source);
            }
            None => {
                self.entries.insert(uuid, (source, name.to_string()));
            }
        }
    }
}

/// Update an existing [`Schedule`] from an XLSX spreadsheet (or CSV directory).
///
/// This is the primary import function.  It merges data from the file into the
/// schedule with upsert semantics — entities matched by their natural key
/// (deterministic v5 UUID from code/name) are updated in place; new entities
/// are created; entities that were present before the import but do not appear
/// in the file are soft-deleted.
///
/// Read order:
/// 1. PanelTypes — so panel-type lookups work during schedule import.
/// 2. Hotels — creates HotelRoom entities with richer metadata (optional).
/// 3. Rooms — so room lookups work during schedule import.
///    Links event rooms to hotel rooms from Hotels sheet or inline Hotel Room column.
/// 4. People — establishes presenter rank/flags before the Schedule sheet
///    creates presenter entities from column headers.
/// 5. Timeline — creates Timeline entities separately from panels (optional).
/// 6. Schedule — panels, timing, rooms, panel type, and presenter edges.
///    Timeline rows (is_timeline panel type) are skipped if Timeline sheet was processed.
///
/// Soft-delete is only applied for entity types whose sheet was processed
/// (not set to [`TableImportMode::Skip`]).
pub fn update_schedule_from_xlsx(
    schedule: &mut Schedule,
    path: &Path,
    options: &XlsxImportOptions,
) -> Result<()> {
    // Create a checkpoint before import so we can roll back on validation error.
    let checkpoint = schedule.save();

    let (mut book, csv_map) = if path.is_dir() {
        let csv_map = CsvFileMap::from_directory(path)?;
        let book = umya_spreadsheet::new_file();
        (book, Some(csv_map))
    } else {
        let book = umya_spreadsheet::reader::xlsx::read(path)
            .with_context(|| format!("Failed to open {}", path.display()))?;
        (book, None)
    };

    // Resolve the source timestamp now but defer writing it until we know
    // whether any entity data actually changed.
    let source_modified = resolve_source_modified(&book, path);

    let file_path = path.to_str().map(str::to_owned);
    let import_time = chrono::Utc::now();

    // Snapshot document heads so we can detect new automerge commits.
    let pre_import_heads = schedule.get_heads();

    // Run import inside a block so `ctx`'s mutable borrow on `schedule` is
    // released before we need `schedule` again in the match below.
    let finalize_result = {
        let mut ctx = ImportContext::new(
            &mut book,
            file_path.as_deref(),
            import_time,
            &csv_map,
            schedule,
            options,
        );

        ctx.read_panel_types()?;
        ctx.read_hotel_rooms()?;
        ctx.read_rooms()?;
        ctx.read_people()?;
        ctx.read_timeline()?;
        ctx.read_schedule()?;

        ctx.finalize()
    };

    // Validate and finalize. If validation fails, restore from checkpoint.
    match finalize_result {
        Ok(()) => {
            // Only advance modified_at and write a history marker when the
            // document actually gained new commits — re-importing an identical
            // source is a no-op.
            if schedule.get_heads() != pre_import_heads {
                schedule.metadata.modified_at = source_modified;
                // Write a labelled bookmark so `cosam-modify log` can show
                // which XLSX files were merged and when.
                let label = path.file_name().and_then(|n| n.to_str()).unwrap_or("xlsx");
                schedule.commit_marker(&format!("Import {label}"));
            }
            Ok(())
        }
        Err(e) => {
            // Roll back to checkpoint on validation error.
            *schedule = Schedule::load(&checkpoint).with_context(|| {
                "Failed to restore schedule from checkpoint after validation error"
            })?;
            Err(e)
        }
    }
}

/// Import an XLSX spreadsheet (or CSV directory) and return a populated [`Schedule`].
///
/// Creates a blank schedule and delegates to [`update_schedule_from_xlsx`].
/// For updating an existing schedule in place, call that function directly.
pub fn import_xlsx(path: &Path, options: &XlsxImportOptions) -> Result<Schedule> {
    let mut schedule = Schedule::new();
    update_schedule_from_xlsx(&mut schedule, path, options)?;
    Ok(schedule)
}

// ── Modified-time resolution ──────────────────────────────────────────────────

/// Resolve the best "last modified" timestamp for the imported spreadsheet.
///
/// Precedence:
/// 1. `dcterms:modified` from `docProps/core.xml` inside the XLSX, if the
///    value parses as RFC 3339 and is after 2010 (earlier values are a known
///    Google Sheets export bug where it writes `2006-09-16`).
/// 2. File-system mtime of `path`.
/// 3. `None` if neither is available.
fn resolve_source_modified(book: &Spreadsheet, path: &Path) -> Option<DateTime<Utc>> {
    // Cutoff: reject xlsx-internal timestamps from 2010 or earlier as suspect.
    let cutoff: DateTime<Utc> = "2010-01-01T00:00:00Z".parse().ok()?;

    let props_raw = book.get_properties().get_modified();
    if !props_raw.is_empty() {
        if let Ok(ts) = DateTime::parse_from_rfc3339(props_raw) {
            let ts_utc = ts.with_timezone(&Utc);
            if ts_utc > cutoff {
                return Some(ts_utc);
            }
        }
    }

    // Fall back to file-system mtime.
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from)
}

// ── Shared data-range utilities ───────────────────────────────────────────────

/// A contiguous data range in a worksheet (all coordinates are 1-based umya values).
/// `header_row` holds the column headers; data rows start at `header_row + 1`.
pub(super) struct DataRange {
    pub(super) sheet_name: String,
    pub(super) start_col: u32,
    pub(super) header_row: u32,
    pub(super) end_col: u32,
    pub(super) end_row: u32,
}

impl DataRange {
    pub(super) fn has_data(&self) -> bool {
        self.end_row > self.header_row && self.end_col >= self.start_col
    }
}

/// Find a named table by name (case-insensitive).
///
/// Returns the table data range if exactly one table matches, otherwise returns None.
fn find_table(book: &Spreadsheet, names: &[&str]) -> Option<DataRange> {
    let mut matches = Vec::new();

    for sheet in book.get_sheet_collection() {
        for table in sheet.get_tables() {
            let table_lower = table.get_name().to_lowercase();
            for name in names {
                if table_lower == name.to_lowercase() {
                    let (start, end) = table.get_area();
                    matches.push(DataRange {
                        sheet_name: sheet.get_name().to_string(),
                        start_col: *start.get_col_num(),
                        header_row: *start.get_row_num(),
                        end_col: *end.get_col_num(),
                        end_row: *end.get_row_num(),
                    });
                    break;
                }
            }
        }
    }

    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

/// Find a named sheet by name (case-insensitive).
///
/// Returns the sheet data range if exactly one sheet matches, otherwise returns None.
fn find_sheet(book: &Spreadsheet, names: &[&str]) -> Option<DataRange> {
    let mut matches = Vec::new();

    for sheet in book.get_sheet_collection() {
        let sheet_lower = sheet.get_name().to_lowercase();
        for name in names {
            if sheet_lower == name.to_lowercase() {
                let end_col = sheet.get_highest_column();
                let end_row = sheet.get_highest_row();
                if end_row >= 2 && end_col >= 1 {
                    matches.push(DataRange {
                        sheet_name: sheet.get_name().to_string(),
                        start_col: 1,
                        header_row: 1,
                        end_col,
                        end_row,
                    });
                    break;
                }
            }
        }
    }

    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

/// Find a named table or sheet by name.
///
/// If `csv_map` is provided and no matching sheet/table is found, it will attempt
/// to import a CSV file with the matching name into the spreadsheet.
///
/// Search order:
/// 1. If `TableImportMode::ReadFrom(name)`:
///    a. Check tables for that name
///    b. If not found, check sheets for that name
/// 2. If still not found, check tables for fallback_table_names (error if multiple matches)
/// 3. If still not found, check sheets for fallback_table_names (error if multiple matches)
/// 4. If `csv_map` is provided and still not found, try to import CSV file
/// 5. If `TableImportMode::Skip`, return None immediately
pub(super) fn find_data_range(
    book: &mut Spreadsheet,
    csv_map: &Option<CsvFileMap>,
    primary_mode: &TableImportMode,
    fallback_table_names: &[&str],
) -> Option<DataRange> {
    match primary_mode {
        TableImportMode::Skip => return None,
        TableImportMode::ReadFrom(name) => {
            // Check tables for the specific name first
            if let Some(range) = find_table(book, &[name]) {
                return Some(range);
            }
            // If not found, check sheets for the specific name
            if let Some(range) = find_sheet(book, &[name]) {
                return Some(range);
            }
            // If still not found, fall through to fallback names
        }
        TableImportMode::Process => {
            // No specific name provided, skip to fallback names
        }
    }

    // Check tables for fallback names
    if let Some(range) = find_table(book, fallback_table_names) {
        return Some(range);
    }

    // Check sheets for fallback names
    if let Some(range) = find_sheet(book, fallback_table_names) {
        return Some(range);
    }

    // If csv_map is provided and still not found, try to import CSV file
    if let Some(csv_map) = csv_map {
        // Try each fallback name to see if there's a CSV file
        for name in fallback_table_names {
            if let Some(csv_path) = csv_map.get(name) {
                let csv_path = Path::new(csv_path);
                if let Ok(()) = import_csv_to_sheet(book, csv_path, name) {
                    // Try to find the sheet again after importing
                    if let Some(range) = find_sheet(book, &[name]) {
                        return Some(range);
                    }
                }
            }
        }
    }

    None
}

/// Return the trimmed string value of a cell, or `None` if empty.
pub(super) fn get_cell_str(ws: &Worksheet, col: u32, row: u32) -> Option<String> {
    let value = ws.get_value((col, row));
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Return the numeric value of a cell if it has one.
pub(super) fn get_cell_number(ws: &Worksheet, col: u32, row: u32) -> Option<f64> {
    ws.get_value_number((col, row))
}

/// Build header maps for a data range.
///
/// Returns `(raw_headers, canonical_headers, col_map)`:
/// - `raw_headers`: raw strings in column order (first occurrence wins in `col_map`).
/// - `canonical_headers`: `canonical_header()` of each raw string, `None` for blanks.
/// - `col_map`: canonical key → 1-based column index.
pub(super) fn build_column_map(
    ws: &Worksheet,
    range: &DataRange,
) -> (Vec<String>, Vec<Option<String>>, HashMap<String, u32>) {
    let mut raw_headers = Vec::new();
    let mut canonical_headers = Vec::new();
    let mut col_map: HashMap<String, u32> = HashMap::new();

    for col in range.start_col..=range.end_col {
        let raw = ws.get_value((col, range.header_row)).trim().to_string();
        let canon = canonical_header(&raw);
        if let Some(ref key) = canon {
            col_map.entry(key.clone()).or_insert(col);
        }
        raw_headers.push(raw);
        canonical_headers.push(canon);
    }

    (raw_headers, canonical_headers, col_map)
}

/// Convert one data row to a map keyed by both raw header and canonical header.
pub(super) fn row_to_map(
    ws: &Worksheet,
    row: u32,
    range: &DataRange,
    raw_headers: &[String],
    canonical_headers: &[Option<String>],
) -> HashMap<String, String> {
    let mut data = HashMap::new();
    for (i, col) in (range.start_col..=range.end_col).enumerate() {
        if let Some(value) = get_cell_str(ws, col, row) {
            if !raw_headers[i].is_empty() {
                data.insert(raw_headers[i].clone(), value.clone());
            }
            if let Some(ref key) = canonical_headers[i] {
                data.entry(key.clone()).or_insert(value);
            }
        }
    }
    data
}

/// Look up the first non-`None` key from a [`FieldDef`]'s key set in `row_data`.
pub(super) fn get_field_def<'a>(
    row_data: &'a HashMap<String, String>,
    field: &FieldDef,
) -> Option<&'a String> {
    for key in field.keys() {
        if let Some(val) = row_data.get(key) {
            return Some(val);
        }
    }
    None
}

/// Return `true` for any non-blank, non-falsy string value.
pub(super) fn is_truthy(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    !lower.is_empty() && lower != "0" && lower != "no" && lower != "false"
}

// ── Extra-column routing ──────────────────────────────────────────────────────

/// Route all columns not in `known_field_keys` for a given row to either the
/// CRDT `__extra` map or the sidecar `formula_extras`, following the priority:
///
/// 1. Explicit ignore (e.g. `OLD_UNIQ_ID`) — skipped before this call.
/// 2. [`FormulaColumnDef`] list → sidecar `formula_extras`.
/// 3. [`crate::extra_field::ExtraFieldDescriptor`] registry → CRDT `__extra`.
/// 4. Unknown: formula cell → sidecar; plain value → CRDT `__extra`.
///
/// `known_field_keys` should be the union of all canonical keys and aliases
/// from the sheet's `FieldDef::ALL` slice plus any explicitly-ignored columns.
/// `skip_columns` is an additional set of column names to skip (e.g., presenter columns).
#[allow(clippy::too_many_arguments)]
pub(super) fn route_extra_columns(
    ws: &Worksheet,
    row: u32,
    range: &DataRange,
    raw_headers: &[String],
    canonical_headers: &[Option<String>],
    known_field_keys: &std::collections::HashSet<String>,
    formula_columns: &[FormulaColumnDef],
    skip_columns: &std::collections::HashSet<String>,
    entity_uuid: uuid::NonNilUuid,
    entity_type_name: &str,
    schedule: &mut Schedule,
) {
    for (i, col) in (range.start_col..=range.end_col).enumerate() {
        let raw = &raw_headers[i];
        if raw.is_empty() {
            continue;
        }
        let canonical = match &canonical_headers[i] {
            Some(c) => c.as_str(),
            None => continue,
        };
        // Skip columns already handled by the field system.
        if known_field_keys.contains(canonical) {
            continue;
        }
        // Skip columns explicitly marked to skip (e.g., presenter columns).
        if skip_columns.contains(raw) {
            continue;
        }

        // --- Step 2: FormulaColumnDef list → sidecar formula extras ---
        let is_formula_col = formula_columns
            .iter()
            .any(|fd| fd.keys().any(|k| k == canonical));
        if is_formula_col {
            let formula_str = ws
                .get_cell((col, row))
                .map(|c| c.get_formula().to_string())
                .filter(|f| !f.is_empty());
            let display_value = get_cell_str(ws, col, row).unwrap_or_default();
            if !display_value.is_empty() || formula_str.is_some() {
                schedule.sidecar_mut().set_formula_extra(
                    entity_uuid,
                    raw.clone(),
                    SidecarFormulaField {
                        formula: formula_str,
                        display_value,
                    },
                );
            }
            continue;
        }

        // --- Step 3: ExtraFieldDescriptor registry → CRDT __extra ---
        if crate::extra_field::find_extra_descriptor(raw, entity_type_name).is_some() {
            if let Some(value) = get_cell_str(ws, col, row) {
                let _ = schedule.write_extra_field(entity_type_name, entity_uuid, raw, &value);
            }
            continue;
        }

        // --- Step 4: Unknown column: detect formula vs. plain value ---
        let formula_str = ws
            .get_cell((col, row))
            .map(|c| c.get_formula().to_string())
            .filter(|f| !f.is_empty());
        let display_value = get_cell_str(ws, col, row).unwrap_or_default();

        if let Some(formula) = formula_str {
            // Formula cell → sidecar (preserve formula for update_xlsx round-trip)
            if !display_value.is_empty() || !formula.is_empty() {
                schedule.sidecar_mut().set_formula_extra(
                    entity_uuid,
                    raw.clone(),
                    SidecarFormulaField {
                        formula: Some(formula),
                        display_value,
                    },
                );
            }
        } else if !display_value.is_empty() {
            // Plain value → CRDT __extra (shared, merged between users)
            let _ = schedule.write_extra_field(entity_type_name, entity_uuid, raw, &display_value);
        }
    }
}

// ── Soft-delete helpers ───────────────────────────────────────────────────────

/// Collect all live entity UUIDs of type `E` from the schedule.
pub(super) fn collect_entity_uuids<E: EntityType>(schedule: &Schedule) -> HashSet<NonNilUuid> {
    schedule
        .iter_entities::<E>()
        .map(|(id, _)| id.entity_uuid())
        .collect()
}

/// Validate that all entities of type `E` are either in `before` or `seen`.
///
/// This detects bugs where entities are created during import but not tracked
/// in the seen set. Every entity added during import should be added to the
/// corresponding seen_* field.
fn validate_seen_tracking<E: EntityType>(
    schedule: &Schedule,
    before: &HashSet<NonNilUuid>,
    seen: &HashSet<NonNilUuid>,
    type_name: &str,
) -> Result<()> {
    let current = collect_entity_uuids::<E>(schedule);
    let before_or_seen: HashSet<_> = before.iter().chain(seen.iter()).cloned().collect();

    for uuid in &current {
        if !before_or_seen.contains(uuid) {
            anyhow::bail!(
                "Import bug: {} entity {} was created during import but not tracked in seen set. \
                 Every entity added during import must be added to the corresponding seen_* field.",
                type_name,
                uuid
            );
        }
    }
    Ok(())
}

/// Soft-delete any entity of type `E` whose UUID was in `before` but not in `seen`.
pub(super) fn soft_delete_unseen<E: EntityType>(
    schedule: &mut Schedule,
    before: &HashSet<NonNilUuid>,
    seen: &HashSet<NonNilUuid>,
) {
    for &uuid in before {
        if !seen.contains(&uuid) {
            // SAFETY: uuid came from iter_entities at snapshot time.
            let id = unsafe { EntityId::<E>::new_unchecked(uuid) };
            schedule.remove_entity::<E>(id);
        }
    }
}

/// Build the set of canonical keys that belong to the field system for a sheet.
///
/// Pass the sheet's `FieldDef::ALL` slice and any additional explicit-ignore
/// columns (e.g. `&[sc::OLD_UNIQ_ID]`).
pub(super) fn known_field_key_set(
    all_fields: &[FieldDef],
    ignore: &[FieldDef],
) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    for fd in all_fields.iter().chain(ignore.iter()) {
        for key in fd.keys() {
            set.insert(key.to_string());
        }
    }
    set
}
