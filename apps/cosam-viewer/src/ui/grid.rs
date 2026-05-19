/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Grid view component — rooms × time slots, mirroring the JS widget's grid mode.

use std::collections::HashMap;

use chrono::Timelike as _;
use dioxus::prelude::*;

use crate::data::WidgetRoom;
use crate::state::PanelView;

/// Slot granularity in minutes.
const SLOT_MINUTES: i64 = 30;

// ---------------------------------------------------------------------------
// Internal layout types
// ---------------------------------------------------------------------------

struct PlacedPanel {
    id: String,
    name: String,
    time_str: String,
    credits: Vec<String>,
    color: Option<String>,
    is_break: bool,
    is_workshop: bool,
    is_premium: bool,
    is_full: bool,
    is_kids: bool,
    /// 1-based grid column (col 1 = time label; rooms start at 2).
    col: usize,
    /// Number of room columns spanned (≥ 1).
    col_span: usize,
    /// 1-based grid row (row 1 = room header; time slots start at 2).
    row_start: usize,
    /// Number of slot rows spanned (≥ 1).
    row_span: usize,
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// Grid view: columns = rooms, rows = 30-minute time slots.
///
/// `rooms` should be the full list of visible rooms (sorted); filters are
/// already applied to `panels` by `ViewerState::panels_for_day`.
#[component]
pub fn GridView(
    panels: Vec<PanelView>,
    rooms: Vec<WidgetRoom>,
    on_panel_click: EventHandler<String>,
) -> Element {
    if rooms.is_empty() || panels.is_empty() {
        return rsx! {
            div { class: "empty-state-inline",
                "No panels to display in grid view."
            }
        };
    }

    // -----------------------------------------------------------------------
    // Time bounds (minutes from midnight, snapped to slot boundaries)
    //
    // Use non-break panels only so that overnight breaks (which may list an
    // end time of e.g. 01:00 next day, recorded here as 01:00 = 60 min) do
    // not push the grid start back to midnight.
    // -----------------------------------------------------------------------
    let raw_start = panels
        .iter()
        .filter(|p| !p.is_break)
        .filter_map(|p| p.start_time)
        .map(|dt| dt.hour() as i64 * 60 + dt.minute() as i64)
        .min()
        .unwrap_or(8 * 60);
    let day_start_min = (raw_start / SLOT_MINUTES) * SLOT_MINUTES;

    let raw_end = panels
        .iter()
        .filter(|p| !p.is_break)
        .filter_map(|p| p.end_time)
        .map(|dt| dt.hour() as i64 * 60 + dt.minute() as i64)
        .max()
        .unwrap_or(day_start_min + 8 * 60);
    let day_end_min = ((raw_end + SLOT_MINUTES - 1) / SLOT_MINUTES) * SLOT_MINUTES;

    let total_slots = ((day_end_min - day_start_min) / SLOT_MINUTES).max(1) as usize;
    let n_rooms = rooms.len();

    // Room uid → 1-based column index within rooms (add 1 for time-label column).
    let room_col: HashMap<i32, usize> = rooms
        .iter()
        .enumerate()
        .map(|(i, r)| (r.uid, i + 2))
        .collect();

    // -----------------------------------------------------------------------
    // Time slot labels
    // -----------------------------------------------------------------------
    let time_labels: Vec<(usize, String)> = (0..total_slots)
        .map(|i| {
            let minutes = day_start_min + i as i64 * SLOT_MINUTES;
            let h = (minutes / 60) % 24;
            let m = minutes % 60;
            let ampm = if h < 12 { "AM" } else { "PM" };
            let h12 = if h % 12 == 0 { 12 } else { h % 12 };
            let label = if m == 0 {
                format!("{h12} {ampm}")
            } else {
                format!("{h12}:{m:02}")
            };
            (i, label)
        })
        .collect();

    // -----------------------------------------------------------------------
    // Pre-compute each panel's grid position
    // -----------------------------------------------------------------------
    let placed: Vec<PlacedPanel> = panels
        .iter()
        .filter_map(|p| {
            let start_min = p
                .start_time
                .map(|dt| dt.hour() as i64 * 60 + dt.minute() as i64)?;
            let end_min = p
                .end_time
                .map(|dt| dt.hour() as i64 * 60 + dt.minute() as i64);
            let duration_min = end_min.map(|e| e - start_min).unwrap_or(SLOT_MINUTES);

            let row_offset = (start_min - day_start_min) / SLOT_MINUTES;
            // Panels that start before the grid window (e.g. an overnight break
            // whose start_time is technically on the previous calendar day) are
            // skipped to avoid negative row indices.
            if row_offset < 0 {
                return None;
            }
            let row_start = row_offset as usize + 2;
            let row_span = (duration_min / SLOT_MINUTES).max(1) as usize;

            if p.is_break {
                return Some(PlacedPanel {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    time_str: p.time_str.clone(),
                    credits: vec![],
                    color: None,
                    is_break: true,
                    is_workshop: false,
                    is_premium: false,
                    is_full: false,
                    is_kids: false,
                    col: 2,
                    col_span: n_rooms,
                    row_start,
                    row_span,
                });
            }

            // Determine column and span from room UIDs.
            let cols: Vec<usize> = p
                .room_ids
                .iter()
                .filter_map(|uid| room_col.get(uid).copied())
                .collect();
            let (col, col_span) = if cols.is_empty() {
                (2usize, 1usize)
            } else {
                let min_col = *cols.iter().min().unwrap();
                let max_col = *cols.iter().max().unwrap();
                // Only span if all room slots are consecutive and present.
                let span = if cols.len() == (max_col - min_col + 1) {
                    max_col - min_col + 1
                } else {
                    1
                };
                (min_col, span)
            };

            Some(PlacedPanel {
                id: p.id.clone(),
                name: p.name.clone(),
                time_str: p.time_str.clone(),
                credits: p.credits.clone(),
                color: p.type_color.clone(),
                is_break: false,
                is_workshop: p.is_workshop,
                is_premium: p.is_premium,
                is_full: p.is_full,
                is_kids: p.is_kids,
                col,
                col_span,
                row_start,
                row_span,
            })
        })
        .collect();

    // -----------------------------------------------------------------------
    // CSS grid dimensions (inline style on the canvas element)
    // -----------------------------------------------------------------------
    let grid_style = format!(
        "grid-template-columns: 64px repeat({n_rooms}, minmax(140px, 1fr)); \
         grid-template-rows: 36px repeat({total_slots}, 60px);"
    );

    // -----------------------------------------------------------------------
    // Render
    // -----------------------------------------------------------------------
    rsx! {
        div {
            class: "grid-scroll-wrapper",
            role: "region",
            aria_label: "Schedule grid",

            div { class: "grid-canvas", style: "{grid_style}",

                // Corner — sticky top-left
                div { class: "grid-corner" }

                // Room header row (sticky top)
                for (i, room) in rooms.iter().enumerate() {
                    div {
                        class: "grid-room-header",
                        style: "grid-column: {i + 2}; grid-row: 1;",
                        title: "{room.long_name}",
                        "{room.short_name}"
                    }
                }

                // Time-slot labels (sticky left)
                for (slot_idx, label) in &time_labels {
                    div {
                        class: "grid-time-label",
                        style: "grid-column: 1; grid-row: {slot_idx + 2};",
                        "{label}"
                    }
                }

                // Background cells — one per room per slot (create gridlines via border)
                for room_i in 0..n_rooms {
                    for slot_i in 0..total_slots {
                        div {
                            class: "grid-bg-cell",
                            style: "grid-column: {room_i + 2}; grid-row: {slot_i + 2};",
                        }
                    }
                }

                // Panels and breaks (rendered last so they appear above bg cells)
                for panel in &placed {
                    if panel.is_break {
                        {
                            let cell_style = format!(
                                "grid-column: 2 / span {}; grid-row: {} / span {};",
                                panel.col_span, panel.row_start, panel.row_span
                            );
                            let label = panel.name.clone();
                            rsx! {
                                div {
                                    class: "grid-break",
                                    style: "{cell_style}",
                                    role: "separator",
                                    aria_label: "{label}",
                                    span { class: "break-label", "{label}" }
                                }
                            }
                        }
                    } else {
                        {
                            let pid = panel.id.clone();
                            let pid2 = pid.clone();
                            let pname = panel.name.clone();
                            let time_str = panel.time_str.clone();
                            let credits = panel.credits.clone();
                            let color = panel.color.clone();
                            let is_workshop = panel.is_workshop;
                            let is_premium = panel.is_premium;
                            let is_full = panel.is_full;
                            let is_kids = panel.is_kids;
                            let cell_style = format!(
                                "grid-column: {} / span {}; grid-row: {} / span {};",
                                panel.col, panel.col_span, panel.row_start, panel.row_span
                            );
                            rsx! {
                                article {
                                    class: "grid-panel",
                                    style: "{cell_style}",
                                    tabindex: "0",
                                    role: "button",
                                    aria_label: "View details for {pname}",
                                    onclick: move |_| on_panel_click.call(pid.clone()),
                                    onkeydown: move |e| {
                                        if e.key() == Key::Enter
                                            || e.key() == Key::Character(" ".to_string())
                                        {
                                            on_panel_click.call(pid2.clone());
                                        }
                                    },

                                    div {
                                        class: "card-color-bar",
                                        style: if let Some(ref c) = color {
                                            format!("background:{c}")
                                        } else {
                                            String::new()
                                        },
                                    }
                                    div { class: "grid-panel-body",
                                        div { class: "grid-panel-name", "{pname}" }
                                        if !time_str.is_empty() {
                                            div { class: "grid-panel-time", "{time_str}" }
                                        }
                                        if !credits.is_empty() {
                                            div {
                                                class: "grid-panel-credits",
                                                "{credits.join(\", \")}"
                                            }
                                        }
                                        div { class: "card-badges",
                                            if is_workshop {
                                                span {
                                                    class: "badge badge-workshop",
                                                    title: "Workshop",
                                                    "W"
                                                }
                                            }
                                            if is_premium {
                                                span {
                                                    class: "badge badge-paid",
                                                    title: "Paid",
                                                    "$"
                                                }
                                            }
                                            if is_full {
                                                span { class: "badge badge-full", "Full" }
                                            }
                                            if is_kids {
                                                span {
                                                    class: "badge badge-kids",
                                                    title: "Kids programming",
                                                    "Kids"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
