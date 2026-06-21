/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Time-grid layout computation: time slots, room columns, cell spans.

use crate::model::{panel_end_iso, panel_start_iso, Panel, Room, ScheduleData};
use crate::time_fmt;

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
    /// The panel started before the window's first time slot (visual zig-zag on top edge).
    pub truncated_start: bool,
    /// The panel ends after the window's last time slot (visual zig-zag on bottom edge).
    pub truncated_end: bool,
}

/// Computed grid layout for a set of panels.
#[derive(Debug)]
pub struct GridLayout {
    pub room_order: Vec<i32>,
    pub time_slots: Vec<TimeSlot>,
    pub cells: Vec<GridCell>,
    pub break_cells: Vec<GridCell>,
    /// ISO 8601 datetime string marking the start of the visible window, if any.
    pub window_start: Option<String>,
    /// ISO 8601 datetime string marking the end of the visible window, if any.
    pub window_end: Option<String>,
}

impl GridLayout {
    /// Compute the grid layout for the given panels and room list.
    ///
    /// `window_start` / `window_end` are optional ISO 8601 datetime strings that
    /// describe the visible time window for this section (used when the section was
    /// produced by a time-split).  When set:
    /// - Panels whose `start_time` precedes `window_start` are included but their
    ///   `row_start` is clamped to the first slot and `truncated_start` is set.
    /// - Panels whose `end_time` extends past `window_end` are included but their
    ///   `row_end` is clamped to the last rendered slot and `truncated_end` is set.
    pub fn compute(
        panels: &[&Panel],
        data: &ScheduleData,
        window_start: Option<&str>,
        window_end: Option<&str>,
    ) -> Self {
        // Times are epoch seconds; recover wall-clock ISO in the schedule's zone.
        let tz = data.meta.timezone.as_str();
        let start_iso = |p: &Panel| panel_start_iso(p, tz);
        let end_iso = |p: &Panel| panel_end_iso(p, tz);

        let regular: Vec<&&Panel> = panels
            .iter()
            .filter(|p| {
                !data
                    .panel_types
                    .get(p.panel_type.as_deref().unwrap_or(""))
                    .map(|pt| pt.is_break)
                    .unwrap_or(false)
                    && p.start_epoch.is_some()
            })
            .collect();

        let breaks: Vec<&&Panel> = panels
            .iter()
            .filter(|p| {
                data.panel_types
                    .get(p.panel_type.as_deref().unwrap_or(""))
                    .map(|pt| pt.is_break)
                    .unwrap_or(false)
                    && p.start_epoch.is_some()
            })
            .collect();

        // Determine room order from regular events
        let room_ids_used: std::collections::HashSet<i32> = regular
            .iter()
            .flat_map(|p| p.room_ids.iter().copied())
            .collect();

        let mut room_order: Vec<i32> = data
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

        // Normalize the optional window bounds to 16-char ISO keys.
        let win_start: Option<String> = window_start.map(|s| s[..16.min(s.len())].to_string());
        let win_end: Option<String> = window_end.map(|s| s[..16.min(s.len())].to_string());

        // Collect all unique time keys (start + end) and sort.
        // When a window is set, include its boundaries so the grid always
        // opens/closes exactly at the window edge even if no panel starts or
        // ends there.
        let mut time_key_set: std::collections::HashSet<String> = panels
            .iter()
            .flat_map(|p| {
                let mut keys = vec![];
                if let Some(s) = start_iso(p) {
                    keys.push(s[..16.min(s.len())].to_string());
                }
                if let Some(e) = end_iso(p) {
                    keys.push(e[..16.min(e.len())].to_string());
                }
                keys
            })
            .collect();
        if let Some(ref ws) = win_start {
            time_key_set.insert(ws.clone());
        }
        if let Some(ref we) = win_end {
            time_key_set.insert(we.clone());
        }
        let mut time_keys: Vec<String> = time_key_set.into_iter().collect();
        time_keys.sort();

        // Drop time keys that are strictly outside [win_start, win_end].
        // Keys equal to win_start or win_end are kept as boundary markers.
        if win_start.is_some() || win_end.is_some() {
            time_keys.retain(|k| {
                win_start.as_deref().is_none_or(|ws| k.as_str() >= ws)
                    && win_end.as_deref().is_none_or(|we| k.as_str() <= we)
            });
        }

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

        // Helper: look up a time key in the slot list, clamped to [0, len-1].
        let slot_idx =
            |key: &str| -> usize { time_slots.iter().position(|ts| ts.key == key).unwrap_or(0) };
        let n_slots = time_slots.len();

        // Build cells, clamping rows to [0, n_slots) and recording truncation.
        let mut cells = vec![];
        for panel in &regular {
            let start_key = start_iso(panel).map(|s| s[..16.min(s.len())].to_string());
            let end_key = end_iso(panel).map(|s| s[..16.min(s.len())].to_string());
            if let (Some(sk), Some(ek)) = (start_key, end_key) {
                // Detect truncation before doing any clamping.
                let truncated_start = win_start.as_deref().is_some_and(|ws| sk.as_str() < ws);
                let truncated_end = win_end.as_deref().is_some_and(|we| ek.as_str() > we);

                // Clamp the effective start/end keys to the visible window.
                let eff_sk = if truncated_start {
                    win_start.as_deref().unwrap()
                } else {
                    sk.as_str()
                };
                let eff_ek = if truncated_end {
                    win_end.as_deref().unwrap()
                } else {
                    ek.as_str()
                };

                let row_start = slot_idx(eff_sk);
                let row_end = time_slots
                    .iter()
                    .position(|ts| ts.key == eff_ek)
                    .unwrap_or((row_start + 1).min(n_slots));

                if row_end <= row_start {
                    continue; // panel is entirely outside the window
                }

                for room_id in &panel.room_ids {
                    if let Some(col) = room_order.iter().position(|r| r == room_id) {
                        cells.push(GridCell {
                            panel: (**panel).clone(),
                            col,
                            row_start,
                            row_end,
                            truncated_start,
                            truncated_end,
                        });
                    }
                }
            }
        }

        let mut break_cells = vec![];
        for panel in &breaks {
            let start_key = start_iso(panel).map(|s| s[..16.min(s.len())].to_string());
            let end_key = end_iso(panel).map(|s| s[..16.min(s.len())].to_string());
            if let (Some(sk), Some(ek)) = (start_key, end_key) {
                let truncated_start = win_start.as_deref().is_some_and(|ws| sk.as_str() < ws);
                let truncated_end = win_end.as_deref().is_some_and(|we| ek.as_str() > we);

                let eff_sk = if truncated_start {
                    win_start.as_deref().unwrap()
                } else {
                    sk.as_str()
                };
                let eff_ek = if truncated_end {
                    win_end.as_deref().unwrap()
                } else {
                    ek.as_str()
                };

                let row_start = slot_idx(eff_sk);
                let row_end = time_slots
                    .iter()
                    .position(|ts| ts.key == eff_ek)
                    .unwrap_or((row_start + 1).min(n_slots));

                if row_end <= row_start {
                    continue;
                }

                break_cells.push(GridCell {
                    panel: (**panel).clone(),
                    col: 0,
                    row_start,
                    row_end,
                    truncated_start,
                    truncated_end,
                });
            }
        }

        GridLayout {
            room_order,
            time_slots,
            window_start: win_start,
            window_end: win_end,
            cells,
            break_cells,
        }
    }

    /// Look up a room by UID.
    pub fn room_name<'a>(&self, uid: i32, rooms: &'a [Room]) -> &'a str {
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
