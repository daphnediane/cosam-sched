/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Time-grid layout computation: time slots, room columns, cell spans.

use crate::color::ColorMode;
use crate::model::{Panel, Room, ScheduleData};
use crate::time_fmt;

/// Page orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Landscape,
    Portrait,
}

impl Orientation {
    /// Returns `true` for landscape orientation.
    pub fn is_landscape(self) -> bool {
        matches!(self, Orientation::Landscape)
    }
}

/// Paper size for output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaperSize {
    Letter,
    Legal,
    #[default]
    Tabloid,
    SuperB,
    /// Custom 30"×20" poster (landscape only).
    Poster,
    Postcard4x6,
}

impl PaperSize {
    /// Returns `(width_mm, height_mm)` in portrait orientation.
    pub fn dimensions_mm(&self) -> (f64, f64) {
        match self {
            PaperSize::Letter => (215.9, 279.4),
            PaperSize::Legal => (215.9, 355.6),
            PaperSize::Tabloid => (279.4, 431.8),
            PaperSize::SuperB => (330.2, 482.6),
            PaperSize::Poster => (508.0, 762.0), // 20"×30" portrait basis
            PaperSize::Postcard4x6 => (101.6, 152.4),
        }
    }

    /// Typst paper name used in `#set page(paper: ...)`.
    /// Returns `None` for sizes that require explicit `width`/`height` dimensions
    /// (e.g. `Poster`).
    pub fn typst_name(&self) -> Option<&'static str> {
        match self {
            PaperSize::Letter => Some("us-letter"),
            PaperSize::Legal => Some("us-legal"),
            PaperSize::Tabloid => Some("us-tabloid"),
            PaperSize::SuperB => Some("iso-b3"),
            PaperSize::Poster => None,
            PaperSize::Postcard4x6 => Some("a6"),
        }
    }

    /// Subdirectory name used under the layout output root.
    pub fn dir_name(&self) -> &'static str {
        match self {
            PaperSize::Letter => "letter",
            PaperSize::Legal => "legal",
            PaperSize::Tabloid => "tabloid",
            PaperSize::SuperB => "super-b",
            PaperSize::Poster => "poster",
            PaperSize::Postcard4x6 => "postcard",
        }
    }

    /// Number of columns for a description/workshops listing on this paper.
    ///
    /// Column counts match the legacy `schedule-to-html` CSS files, targeting
    /// a fixed ~3-inch column width across paper sizes.
    pub fn description_columns(&self, orientation: Orientation) -> u32 {
        match self {
            PaperSize::Letter => {
                if orientation.is_landscape() {
                    4
                } else {
                    3
                }
            }
            PaperSize::Legal => {
                if orientation.is_landscape() {
                    4
                } else {
                    3
                }
            }
            PaperSize::Tabloid | PaperSize::SuperB => {
                if orientation.is_landscape() {
                    5
                } else {
                    4
                }
            }
            PaperSize::Poster => 5,
            PaperSize::Postcard4x6 => 1,
        }
    }

    /// Number of columns for a flyer-schedule page on this paper.
    ///
    /// The flyer format devotes the left half of the first page (rounded up) to
    /// the day grid and flows descriptions through the remaining columns, so the
    /// total must be even-friendly: letter uses 4 columns, legal and larger use
    /// 6.  Portrait falls back to narrower counts.
    pub fn flyer_columns(&self, orientation: Orientation) -> u32 {
        match self {
            PaperSize::Letter => {
                if orientation.is_landscape() {
                    4
                } else {
                    2
                }
            }
            PaperSize::Legal | PaperSize::Tabloid | PaperSize::SuperB | PaperSize::Poster => {
                if orientation.is_landscape() {
                    6
                } else {
                    4
                }
            }
            PaperSize::Postcard4x6 => 2,
        }
    }

    /// Base font size (as a Typst length string) for body text on this paper.
    ///
    /// The `Poster` size uses a larger base font so that panels are legible at
    /// reading distance from a printed 30"×20" sheet.
    pub fn base_font_pt(&self) -> &'static str {
        match self {
            PaperSize::Poster => "10pt",
            _ => "9pt",
        }
    }
}

/// How to split sections by entity (room or presenter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionSplit {
    /// One section per room.
    Room,
    /// One section per presenter.
    Presenter,
}

/// How to split sections by time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSplit {
    /// One section per calendar day.
    Day,
    /// One section per AM/PM half-day.
    HalfDay,
}

/// Page-footer content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FooterMode {
    /// Modified/generated timestamps, a centered page number, and the site/org
    /// label (the default).
    #[default]
    Full,
    /// Modified/generated timestamps only — no page number or site label.
    TimestampOnly,
    /// No footer at all.
    None,
}

/// Which content a section renders, with how to split it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentMode {
    /// Schedule grid and panel descriptions side by side.
    Both {
        section: Option<SectionSplit>,
        time: TimeSplit,
    },
    /// Schedule grid only — no descriptions.
    GridOnly {
        section: Option<SectionSplit>,
        time: TimeSplit,
    },
    /// Panel descriptions only — no grid.
    DescriptionOnly {
        section: Option<SectionSplit>,
        time: Option<TimeSplit>,
    },
    /// Compact panel list (name + time + room), the former guest-postcard layout.
    PanelList {
        section: Option<SectionSplit>,
        time: Option<TimeSplit>,
    },
}

impl Default for ContentMode {
    fn default() -> Self {
        ContentMode::Both {
            section: None,
            time: TimeSplit::Day,
        }
    }
}

impl ContentMode {
    /// The section (entity) split, if any.
    pub fn section_split(self) -> Option<SectionSplit> {
        match self {
            ContentMode::Both { section, .. }
            | ContentMode::GridOnly { section, .. }
            | ContentMode::DescriptionOnly { section, .. }
            | ContentMode::PanelList { section, .. } => section,
        }
    }

    /// The time split, if any.
    pub fn time_split(self) -> Option<TimeSplit> {
        match self {
            ContentMode::Both { time, .. } | ContentMode::GridOnly { time, .. } => Some(time),
            ContentMode::DescriptionOnly { time, .. } | ContentMode::PanelList { time, .. } => time,
        }
    }

    /// Whether any split is active (section or time).
    pub fn has_split(self) -> bool {
        self.section_split().is_some() || self.time_split().is_some()
    }

    /// Whether both a section split and a time split are active (two-slot running header).
    pub fn is_two_dim(self) -> bool {
        self.section_split().is_some() && self.time_split().is_some()
    }

    /// Whether this content draws the schedule grid.
    pub fn shows_grid(self) -> bool {
        matches!(
            self,
            ContentMode::Both { .. } | ContentMode::GridOnly { .. }
        )
    }

    /// Whether this content draws panel text (descriptions or list).
    pub fn shows_text(self) -> bool {
        matches!(
            self,
            ContentMode::Both { .. }
                | ContentMode::DescriptionOnly { .. }
                | ContentMode::PanelList { .. }
        )
    }
}

/// Which panels to include in the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelFilter {
    /// Every scheduled panel (the default).
    #[default]
    All,
    /// Workshop panels only (premium workshops included; cafe excluded).
    Workshops,
    /// Premium workshops only.
    Premium,
}

/// Complete configuration for a single layout job.
#[derive(Debug, Clone, Default)]
pub struct LayoutConfig {
    pub paper: PaperSize,
    /// What to render and how to split it.
    pub content: ContentMode,
    /// Which panels to include.
    pub panel_filter: PanelFilter,
    pub orientation: Orientation,
    /// Color or black-and-white output.
    pub color_mode: ColorMode,
    /// Override for the number of layout columns. If `None`, each content mode
    /// uses its own per-paper default (e.g. [`PaperSize::flyer_columns`]).
    pub columns: Option<u32>,
    /// Page-footer content.
    pub footer: FooterMode,
    /// Insert a blank page so each section starts on an odd page (double-sided
    /// booklet printing).
    pub double_sided: bool,
    /// Optional header text: shown on the left for 1-D splits, on the right when
    /// there is no split, and omitted for 2-D splits (where both header slots
    /// carry the running entity/day labels).
    pub header_text: Option<String>,
    /// Override for base font size (e.g., "14pt"). If None, uses paper's default.
    pub base_font_pt: Option<String>,
    /// Override for grid event text size (e.g., "8pt"). If None, uses base_font_pt.
    pub grid_font_pt: Option<String>,
}

impl LayoutConfig {
    /// Resolve the column count, honoring the [`columns`](Self::columns)
    /// override and falling back to `default` (clamped to at least 1).
    pub fn effective_columns(&self, default: u32) -> u32 {
        self.columns.unwrap_or(default).max(1)
    }

    /// Get the effective base font size for this layout.
    pub fn effective_font_pt(&self) -> &str {
        self.base_font_pt
            .as_deref()
            .unwrap_or_else(|| self.paper.base_font_pt())
    }

    /// Parse the base font size as an f64 value.
    pub fn base_font_value(&self) -> f64 {
        self.effective_font_pt()
            .trim_end_matches("pt")
            .trim_end_matches("px")
            .parse::<f64>()
            .unwrap_or(9.0)
    }

    /// Get the effective grid font size for this layout.
    /// Falls back to base_font_pt if grid_font_pt is not set.
    pub fn grid_font_value(&self) -> f64 {
        self.grid_font_pt
            .as_deref()
            .unwrap_or_else(|| self.effective_font_pt())
            .trim_end_matches("pt")
            .trim_end_matches("px")
            .parse::<f64>()
            .unwrap_or_else(|_| self.base_font_value())
    }
}

/// A computed time slot in the grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSlot {
    /// ISO 8601 local datetime string (minutes precision), e.g. `"2026-06-26T14:00"`.
    pub key: String,
    /// Human-readable label, e.g. `"2 PM"` or `"2:30"`.
    pub label: String,
    /// Whether this is an on-the-hour slot.
    pub is_major: bool,
    /// Day label shown at the first slot of a new day, if split mode is all-days.
    pub day_label: Option<String>,
}

/// A single event cell in the grid.
#[derive(Debug, Clone)]
pub struct GridCell {
    pub panel: Panel,
    /// Column index (0-based, into `room_order`).
    pub col: usize,
    /// Start row index (into `time_slots`).
    pub row_start: usize,
    /// Exclusive end row index.
    pub row_end: usize,
}

/// Computed grid layout for a set of panels.
#[derive(Debug)]
pub struct GridLayout {
    pub room_order: Vec<i64>,
    pub time_slots: Vec<TimeSlot>,
    pub cells: Vec<GridCell>,
    pub break_cells: Vec<GridCell>,
}

impl GridLayout {
    /// Compute the grid layout for the given panels and room list.
    pub fn compute(panels: &[&Panel], data: &ScheduleData) -> Self {
        let regular: Vec<&&Panel> = panels
            .iter()
            .filter(|p| {
                !data
                    .panel_types
                    .get(p.panel_type.as_deref().unwrap_or(""))
                    .map(|pt| pt.is_break)
                    .unwrap_or(false)
                    && p.start_time.is_some()
            })
            .collect();

        let breaks: Vec<&&Panel> = panels
            .iter()
            .filter(|p| {
                data.panel_types
                    .get(p.panel_type.as_deref().unwrap_or(""))
                    .map(|pt| pt.is_break)
                    .unwrap_or(false)
                    && p.start_time.is_some()
            })
            .collect();

        // Determine room order from regular events
        let room_ids_used: std::collections::HashSet<i64> = regular
            .iter()
            .flat_map(|p| p.room_ids.iter().copied())
            .collect();

        let mut room_order: Vec<i64> = data
            .sorted_rooms()
            .iter()
            .filter(|r| room_ids_used.contains(&r.uid))
            .map(|r| r.uid)
            .collect();

        // Append any room IDs not in the rooms list
        for id in &room_ids_used {
            if !room_order.contains(id) {
                room_order.push(*id);
            }
        }

        // Collect all unique time keys (start + end) and sort
        let mut time_keys: Vec<String> = panels
            .iter()
            .flat_map(|p| {
                let mut keys = vec![];
                if let Some(s) = &p.start_time {
                    keys.push(s[..16.min(s.len())].to_string());
                }
                if let Some(e) = &p.end_time {
                    keys.push(e[..16.min(e.len())].to_string());
                }
                keys
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        time_keys.sort();

        let time_slots: Vec<TimeSlot> = time_keys
            .iter()
            .map(|key| {
                let is_major =
                    !key.ends_with(":30") && !key.ends_with(":15") && !key.ends_with(":45");
                TimeSlot {
                    key: key.clone(),
                    label: time_fmt::format_time(key),
                    is_major,
                    day_label: None,
                }
            })
            .collect();

        // Build cells
        let mut cells = vec![];
        for panel in &regular {
            let start_key = panel
                .start_time
                .as_ref()
                .map(|s| s[..16.min(s.len())].to_string());
            let end_key = panel
                .end_time
                .as_ref()
                .map(|s| s[..16.min(s.len())].to_string());
            if let (Some(sk), Some(ek)) = (start_key, end_key) {
                let row_start = time_slots.iter().position(|ts| ts.key == sk).unwrap_or(0);
                let row_end = time_slots
                    .iter()
                    .position(|ts| ts.key == ek)
                    .unwrap_or(row_start + 1);
                for room_id in &panel.room_ids {
                    if let Some(col) = room_order.iter().position(|r| r == room_id) {
                        cells.push(GridCell {
                            panel: (**panel).clone(),
                            col,
                            row_start,
                            row_end,
                        });
                    }
                }
            }
        }

        let mut break_cells = vec![];
        for panel in &breaks {
            let start_key = panel
                .start_time
                .as_ref()
                .map(|s| s[..16.min(s.len())].to_string());
            let end_key = panel
                .end_time
                .as_ref()
                .map(|s| s[..16.min(s.len())].to_string());
            if let (Some(sk), Some(ek)) = (start_key, end_key) {
                let row_start = time_slots.iter().position(|ts| ts.key == sk).unwrap_or(0);
                let row_end = time_slots
                    .iter()
                    .position(|ts| ts.key == ek)
                    .unwrap_or(row_start + 1);
                break_cells.push(GridCell {
                    panel: (**panel).clone(),
                    col: 0,
                    row_start,
                    row_end,
                });
            }
        }

        GridLayout {
            room_order,
            time_slots,
            cells,
            break_cells,
        }
    }

    /// Look up a room by UID.
    pub fn room_name<'a>(&self, uid: i64, rooms: &'a [Room]) -> &'a str {
        rooms
            .iter()
            .find(|r| r.uid == uid)
            .map(|r| {
                if !r.long_name.is_empty() {
                    r.long_name.as_str()
                } else {
                    r.short_name.as_str()
                }
            })
            .unwrap_or("?")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_size_dimensions() {
        let (w, h) = PaperSize::Tabloid.dimensions_mm();
        assert!(w > 0.0 && h > 0.0);
        assert!(h > w); // portrait: height > width
    }

    #[test]
    fn test_paper_size_letter_dimensions() {
        let (w, h) = PaperSize::Letter.dimensions_mm();
        assert!(w > 0.0 && h > 0.0);
        assert!(h > w); // portrait
    }

    #[test]
    fn test_paper_size_poster_dimensions() {
        let (w, h) = PaperSize::Poster.dimensions_mm();
        // 20"×30" in portrait basis → 508mm × 762mm
        assert!((w - 508.0).abs() < 1.0);
        assert!((h - 762.0).abs() < 1.0);
    }

    #[test]
    fn test_paper_size_typst_name() {
        assert_eq!(PaperSize::Letter.typst_name(), Some("us-letter"));
        assert_eq!(PaperSize::Tabloid.typst_name(), Some("us-tabloid"));
        assert_eq!(PaperSize::Poster.typst_name(), None);
    }

    #[test]
    fn test_paper_size_dir_name() {
        assert_eq!(PaperSize::Letter.dir_name(), "letter");
        assert_eq!(PaperSize::Legal.dir_name(), "legal");
        assert_eq!(PaperSize::Tabloid.dir_name(), "tabloid");
        assert_eq!(PaperSize::Poster.dir_name(), "poster");
        assert_eq!(PaperSize::Postcard4x6.dir_name(), "postcard");
    }

    #[test]
    fn test_paper_size_description_columns() {
        assert_eq!(
            PaperSize::Letter.description_columns(Orientation::Portrait),
            3
        );
        assert_eq!(
            PaperSize::Letter.description_columns(Orientation::Landscape),
            4
        );
        assert_eq!(
            PaperSize::Legal.description_columns(Orientation::Portrait),
            3
        );
        assert_eq!(
            PaperSize::Tabloid.description_columns(Orientation::Landscape),
            5
        );
        assert_eq!(
            PaperSize::Poster.description_columns(Orientation::Landscape),
            5
        );
    }

    #[test]
    fn test_paper_size_base_font_pt() {
        assert_eq!(PaperSize::Letter.base_font_pt(), "9pt");
        assert_eq!(PaperSize::Poster.base_font_pt(), "10pt");
    }
}
