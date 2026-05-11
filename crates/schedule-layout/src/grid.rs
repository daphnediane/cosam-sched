/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Time-grid layout computation: time slots, room columns, cell spans.

use crate::model::{Panel, Room, ScheduleData};

/// Paper size for output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaperSize {
    Legal,
    #[default]
    Tabloid,
    SuperB,
    Postcard4x6,
}

impl PaperSize {
    /// Returns `(width_mm, height_mm)` in portrait orientation.
    pub fn dimensions_mm(&self) -> (f64, f64) {
        match self {
            PaperSize::Legal => (215.9, 355.6),
            PaperSize::Tabloid => (279.4, 431.8),
            PaperSize::SuperB => (330.2, 482.6),
            PaperSize::Postcard4x6 => (101.6, 152.4),
        }
    }

    pub fn typst_name(&self) -> &'static str {
        match self {
            PaperSize::Legal => "us-legal",
            PaperSize::Tabloid => "us-tabloid",
            PaperSize::SuperB => "iso-b3",
            PaperSize::Postcard4x6 => "a6",
        }
    }
}

/// Output layout format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutFormat {
    #[default]
    Schedule,
    WorkshopPoster,
    RoomSigns,
    GuestPostcards,
    Descriptions,
}

/// How to split the schedule output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitMode {
    #[default]
    Day,
    HalfDay,
}

/// Filter criteria for layout generation.
#[derive(Debug, Clone, Default)]
pub struct LayoutFilter {
    /// Workshop poster: include premium-only workshops.
    pub premium_only: bool,
    /// Room signs: filter to specific room UID.
    pub room_uid: Option<i64>,
    /// Guest postcards: filter to specific presenter name.
    pub guest_name: Option<String>,
}

/// Complete configuration for a single layout job.
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub paper: PaperSize,
    pub format: LayoutFormat,
    pub split_by: SplitMode,
    pub filter: LayoutFilter,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            paper: PaperSize::default(),
            format: LayoutFormat::default(),
            split_by: SplitMode::default(),
            filter: LayoutFilter::default(),
        }
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
                    label: format_time_label(key),
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

fn format_time_label(key: &str) -> String {
    // key is "YYYY-MM-DDTHH:MM"
    let time_part = key.get(11..).unwrap_or(key);
    let parts: Vec<&str> = time_part.splitn(2, ':').collect();
    if parts.len() < 2 {
        return time_part.to_string();
    }
    let hour: u32 = parts[0].parse().unwrap_or(0);
    let min: u32 = parts[1].parse().unwrap_or(0);
    let (h12, suffix) = if hour == 0 {
        (12, "AM")
    } else if hour < 12 {
        (hour, "AM")
    } else if hour == 12 {
        (12, "PM")
    } else {
        (hour - 12, "PM")
    };
    if min == 0 {
        format!("{} {}", h12, suffix)
    } else {
        format!("{}:{:02}", h12, min)
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
    fn test_format_time_label_noon() {
        assert_eq!(format_time_label("2026-06-26T12:00"), "12 PM");
    }

    #[test]
    fn test_format_time_label_midnight() {
        assert_eq!(format_time_label("2026-06-26T00:00"), "12 AM");
    }

    #[test]
    fn test_format_time_label_half_hour() {
        assert_eq!(format_time_label("2026-06-26T14:30"), "2:30");
    }

    #[test]
    fn test_format_time_label_pm() {
        assert_eq!(format_time_label("2026-06-26T13:00"), "1 PM");
    }
}
