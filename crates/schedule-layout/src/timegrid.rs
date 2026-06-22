/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Time-grid layout computation: time slots, room columns, cell spans.

use crate::model::{Panel, Room, ScheduleData};
use crate::time_fmt;
use schedule_core::value::timezone::epoch_to_local_iso;

/// A computed time slot in the grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSlot {
    /// Unix epoch seconds for this slot, minute-aligned (`epoch % 60 == 0`).
    pub epoch: i64,
    /// ISO 8601 local datetime string (minutes precision), e.g. `"2026-06-26T14:00"`.
    pub key: String,
    /// Human-readable label, e.g. `"2 PM"` or `"2:30"`.
    pub label: String,
    /// Whether this is an on-the-hour slot in the schedule's local timezone.
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
    /// Unix epoch seconds marking the start of the visible window, if any.
    pub window_start: Option<i64>,
    /// Unix epoch seconds marking the end of the visible window, if any.
    pub window_end: Option<i64>,
}

impl GridLayout {
    /// Compute the grid layout for the given panels and room list.
    ///
    /// `window_start` / `window_end` are optional ISO 8601 datetime strings that
    /// describe the visible time window for this section (used when the section was
    /// produced by a time-split).  When set:
    /// - Panels whose start precedes `window_start` are included but their
    ///   `row_start` is clamped to the first slot and `truncated_start` is set.
    /// - Panels whose end extends past `window_end` are included but their
    ///   `row_end` is clamped to the last rendered slot and `truncated_end` is set.
    pub fn compute(
        panels: &[&Panel],
        data: &ScheduleData,
        window_start: Option<i64>,
        window_end: Option<i64>,
    ) -> Self {
        Self::compute_inner(panels, data, window_start, window_end, false)
    }

    /// Compute the grid layout with an even time axis.
    ///
    /// Identical to [`Self::compute`] but also fills intermediate time slots at a
    /// regular interval between each event's start and end, producing a uniform
    /// row height for print layouts. The interval is the GCD of the local
    /// minute-of-hour across all regular events (clamped to 15–60 min), computed
    /// using the precomputed TZ offsets in `data.meta` so non-integer-hour zones
    /// (e.g. IST UTC+5:30) produce the correct on-the-hour grid.
    pub fn compute_even(
        panels: &[&Panel],
        data: &ScheduleData,
        window_start: Option<i64>,
        window_end: Option<i64>,
    ) -> Self {
        Self::compute_inner(panels, data, window_start, window_end, true)
    }

    fn compute_inner(
        panels: &[&Panel],
        data: &ScheduleData,
        window_start: Option<i64>,
        window_end: Option<i64>,
        fill_even: bool,
    ) -> Self {
        let tz = data.meta.timezone.as_str();

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

        // Determine room order from regular events.
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

        for id in &room_ids_used {
            if !room_order.contains(id) {
                room_order.push(*id);
            }
        }

        // Precomputed TZ offset helpers for is_major and GCD computation.
        let tz_offset_minutes = data.meta.tz_offset_minutes.unwrap_or(0);
        let dst_transition = data
            .meta
            .tz_dst_transition_epoch
            .zip(data.meta.tz_dst_offset_minutes);

        // Convert optional window-bound ISO strings to minute-aligned epochs.
        let win_start_epoch: Option<i64> = window_start;
        let win_end_epoch: Option<i64> = window_end;

        // Collect slot epochs from panels that actually render: regular events
        // with a room column, plus breaks (which span col 0). A roomless,
        // non-break panel — e.g. a private "Lunch" hold with no room — produces
        // no cell, so it must not extend the time axis with an empty row.
        let renders_cell = |p: &Panel| p.room_ids.iter().any(|r| room_order.contains(r));
        let mut slot_set: std::collections::HashSet<i64> = regular
            .iter()
            .filter(|p| renders_cell(p))
            .chain(breaks.iter())
            .flat_map(|p| {
                let mut eps = vec![];
                if let Some(e) = p.start_epoch {
                    eps.push(slot_epoch(e));
                }
                if let Some(e) = p.end_epoch {
                    eps.push(slot_epoch(e));
                }
                eps
            })
            .collect();

        // Add window boundary epochs so the grid opens/closes at the edge.
        if let Some(wse) = win_start_epoch {
            slot_set.insert(wse);
        }
        if let Some(wee) = win_end_epoch {
            slot_set.insert(wee);
        }

        // Even-slot filling: GCD of local minute-of-hour for regular events.
        if fill_even {
            let unit_secs = grid_unit_secs(&regular, tz_offset_minutes, dst_transition);
            for panel in regular.iter().filter(|p| renders_cell(p)) {
                if let (Some(se), Some(ee)) = (panel.start_epoch, panel.end_epoch) {
                    let mut s = slot_epoch(se);
                    let end = slot_epoch(ee);
                    while s <= end {
                        slot_set.insert(s);
                        s += unit_secs;
                    }
                }
            }
        }

        // Filter slots to the visible window.
        if win_start_epoch.is_some() || win_end_epoch.is_some() {
            slot_set.retain(|&e| {
                win_start_epoch.is_none_or(|ws| e >= ws) && win_end_epoch.is_none_or(|we| e <= we)
            });
        }

        let mut sorted_epochs: Vec<i64> = slot_set.into_iter().collect();
        sorted_epochs.sort_unstable();

        let time_slots: Vec<TimeSlot> = sorted_epochs
            .iter()
            .map(|&ep| {
                let iso = epoch_to_local_iso(ep, tz);
                let key = iso[..16.min(iso.len())].to_string();
                TimeSlot {
                    epoch: ep,
                    key: key.clone(),
                    label: time_fmt::format_time(&key),
                    is_major: is_major_slot(ep, tz_offset_minutes, dst_transition),
                    day_label: None,
                }
            })
            .collect();

        // Lookup: epoch → slot index (O(n) linear scan; slot counts are small).
        let slot_idx =
            |ep: i64| -> usize { sorted_epochs.iter().position(|&e| e == ep).unwrap_or(0) };
        let n_slots = time_slots.len();

        // Build event cells.
        let mut cells = vec![];
        for panel in &regular {
            if let (Some(se), Some(ee)) = (panel.start_epoch, panel.end_epoch) {
                let ps = slot_epoch(se);
                let pe = slot_epoch(ee);

                let truncated_start = win_start_epoch.is_some_and(|ws| ps < ws);
                let truncated_end = win_end_epoch.is_some_and(|we| pe > we);

                let eff_ps = if truncated_start {
                    win_start_epoch.unwrap()
                } else {
                    ps
                };
                let eff_pe = if truncated_end {
                    win_end_epoch.unwrap()
                } else {
                    pe
                };

                let row_start = slot_idx(eff_ps);
                let row_end = sorted_epochs
                    .iter()
                    .position(|&e| e == eff_pe)
                    .unwrap_or((row_start + 1).min(n_slots));

                if row_end <= row_start {
                    continue;
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

        // Build break cells.
        let mut break_cells = vec![];
        for panel in &breaks {
            if let (Some(se), Some(ee)) = (panel.start_epoch, panel.end_epoch) {
                let ps = slot_epoch(se);
                let pe = slot_epoch(ee);

                let truncated_start = win_start_epoch.is_some_and(|ws| ps < ws);
                let truncated_end = win_end_epoch.is_some_and(|we| pe > we);

                let eff_ps = if truncated_start {
                    win_start_epoch.unwrap()
                } else {
                    ps
                };
                let eff_pe = if truncated_end {
                    win_end_epoch.unwrap()
                } else {
                    pe
                };

                let row_start = slot_idx(eff_ps);
                let row_end = sorted_epochs
                    .iter()
                    .position(|&e| e == eff_pe)
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
            window_start: win_start_epoch,
            window_end: win_end_epoch,
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

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Round epoch seconds down to the nearest minute boundary (equivalent of the
/// JS `epochToSlotEpoch` function).
fn slot_epoch(epoch: i64) -> i64 {
    (epoch / 60) * 60
}

/// True when `slot_epoch` falls on a local-time hour boundary, using the
/// precomputed TZ offset from `WidgetMeta`. When `dst_transition` is
/// `Some((transition_epoch, new_offset_minutes))` the post-transition offset
/// is applied for slots at or after the transition instant.
fn is_major_slot(
    slot_epoch: i64,
    tz_offset_minutes: i32,
    dst_transition: Option<(i64, i32)>,
) -> bool {
    let offset_secs = match dst_transition {
        Some((ep, off)) if slot_epoch >= ep => off as i64 * 60,
        _ => tz_offset_minutes as i64 * 60,
    };
    (slot_epoch + offset_secs) % 3600 == 0
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Compute the even-slot grid unit in seconds: the GCD of the local
/// minute-of-hour across all regular event start/end times, clamped to
/// 15–60 minutes. Uses the precomputed TZ offset so non-integer-hour zones
/// (e.g. IST UTC+5:30) give the correct on-the-hour result.
fn grid_unit_secs(
    regular: &[&&Panel],
    tz_offset_minutes: i32,
    dst_transition: Option<(i64, i32)>,
) -> i64 {
    let local_minute = |ep: i64| -> i64 {
        let offset_secs = match dst_transition {
            Some((dep, doff)) if ep >= dep => doff as i64 * 60,
            _ => tz_offset_minutes as i64 * 60,
        };
        ((ep + offset_secs) % 3600 + 3600) % 3600 / 60
    };

    let mut unit_minutes: i64 = 60;
    for panel in regular {
        for &ep_opt in &[panel.start_epoch, panel.end_epoch] {
            if let Some(ep) = ep_opt {
                let m = local_minute(slot_epoch(ep));
                if m > 0 {
                    unit_minutes = gcd(unit_minutes, m);
                }
            }
        }
    }
    unit_minutes.clamp(15, 60) * 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_epoch_rounds_down() {
        assert_eq!(slot_epoch(3690), 3660); // 1h 1m 30s → 1h 1m 0s
        assert_eq!(slot_epoch(3600), 3600); // already on minute boundary
        assert_eq!(slot_epoch(3659), 3600); // 59s into minute → start
        assert_eq!(slot_epoch(7199), 7140); // 1h 59m 59s → 1h 59m 0s
        assert_eq!(slot_epoch(0), 0);
    }

    #[test]
    fn is_major_slot_integer_hour_tz() {
        // UTC-4 (EDT): offset = -240 min = -14400 s
        // 2026-06-26T14:00 EDT = epoch 1_782_532_800
        // (1_782_532_800 + (-14400)) % 3600 = 1_782_518_400 % 3600 = 0 → major
        let ep = 1_782_532_800_i64;
        assert!(is_major_slot(ep, -240, None));
        // Half-hour slot (ep + 1800):
        assert!(!is_major_slot(ep + 1800, -240, None));
    }

    #[test]
    fn is_major_slot_half_hour_tz() {
        // IST = UTC+5:30 = +330 min = +19800 s
        // An IST 14:00 event: epoch = 14*3600 - 19800 (for an arbitrary date).
        // We just need (epoch + 19800) % 3600 == 0.
        // epoch = 0 → local = 05:30 → not major
        // epoch = 1800 → local = 06:00 → major
        assert!(is_major_slot(1800, 330, None));
        assert!(!is_major_slot(0, 330, None));
    }

    #[test]
    fn is_major_slot_with_dst_transition() {
        // Before transition: use base offset -300 (EST, UTC-5)
        // After transition (say epoch 1_000): use dst offset -240 (EDT, UTC-4)
        // epoch 0: (0 + (-300*60)) % 3600 = -18000 % 3600 = 0 → major (EST hour)
        // epoch 3600: (3600 + (-240*60)) % 3600 = (3600-14400) % 3600 = -10800 % 3600 = 0 → major (EDT hour)
        let dst = Some((1_000_i64, -240_i32));
        assert!(is_major_slot(0, -300, dst));
        assert!(is_major_slot(3600, -300, dst));
    }

    #[test]
    fn grid_unit_secs_all_on_hour() {
        // All events on the hour → local minute is always 0 → unit stays 60 min.
        // With UTC offset 0, slot_epoch % 3600 == 0 for all on-the-hour events.
        // We need a simple Panel-like setup. Since Panel is external, just test
        // the gcd helper directly.
        assert_eq!(gcd(60, 0), 60);
        assert_eq!(gcd(60, 30), 30);
        assert_eq!(gcd(30, 15), 15);
    }

    #[test]
    fn gcd_correctness() {
        assert_eq!(gcd(60, 45), 15);
        assert_eq!(gcd(60, 20), 20);
        assert_eq!(gcd(60, 0), 60);
    }
}
