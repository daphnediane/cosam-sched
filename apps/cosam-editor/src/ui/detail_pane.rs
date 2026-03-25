/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::prelude::*;
use gpui::{Context, EventEmitter, MouseButton, SharedString, Window, div, px, rgb};
use gpui_component::description_list::DescriptionList;

use schedule_core::data::time;

use crate::data::Panel;

struct SessionEntry {
    session_id: String,
    label: SharedString,
    effective_description: Option<String>,
    effective_note: Option<String>,
    effective_prereq: Option<String>,
    effective_alt_panelist: Option<String>,
    effective_presenters: Vec<String>,
    room_names: Vec<String>,
    start_dt: Option<chrono::NaiveDateTime>,
    end_dt: Option<chrono::NaiveDateTime>,
    duration: u32,
    is_full: bool,
    hide_panelist: bool,
    capacity: Option<String>,
    notes_non_printing: Option<String>,
    workshop_notes: Option<String>,
    power_needs: Option<String>,
    sewing_machines: bool,
    av_notes: Option<String>,
}

#[derive(Clone, Debug)]
pub enum DetailPaneEvent {
    Close,
    OpenEdit { base_id: String, session_id: String },
}

impl EventEmitter<DetailPaneEvent> for DetailPane {}

pub struct DetailPane {
    base_id: String,
    panel_name: String,
    panel_type_name: Option<String>,
    panel_cost: Option<String>,
    panel_difficulty: Option<String>,
    panel_is_free: bool,
    panel_is_kids: bool,
    sessions: Vec<SessionEntry>,
    selected_idx: usize,
    session_dropdown_open: bool,
}

impl DetailPane {
    pub fn new(
        panel: &Panel,
        rooms: &[(u32, String)],
        panel_types: &[(String, String)],
        selected_session_id: &str,
    ) -> Self {
        let sessions = Self::build_entries(panel, rooms);
        let selected_idx = sessions
            .iter()
            .position(|s| s.session_id == selected_session_id)
            .unwrap_or(0);

        let panel_type_name = panel.panel_type.as_deref().and_then(|uid| {
            panel_types
                .iter()
                .find(|(u, _)| u == uid)
                .map(|(_, kind)| kind.clone())
        });

        Self {
            base_id: panel.id.clone(),
            panel_name: panel.name.clone(),
            panel_type_name,
            panel_cost: panel.cost.clone(),
            panel_difficulty: panel.difficulty.clone(),
            panel_is_free: panel.is_free,
            panel_is_kids: panel.is_kids,
            sessions,
            selected_idx,
            session_dropdown_open: false,
        }
    }

    fn build_entries(panel: &Panel, rooms: &[(u32, String)]) -> Vec<SessionEntry> {
        let mut entries = Vec::new();
        for part in &panel.parts {
            for session in &part.sessions {
                let start_dt = session.start_time.as_deref().and_then(time::parse_storage);
                let end_dt = session.end_time.as_deref().and_then(time::parse_storage);

                let label = if let Some(start) = start_dt {
                    SharedString::from(format!(
                        "{}: {} {}",
                        session.id,
                        start.format("%a"),
                        start.format("%-I:%M %p")
                    ))
                } else {
                    SharedString::from(session.id.clone())
                };

                let effective_description = Self::effective_concat(
                    panel.description.as_deref(),
                    part.description.as_deref(),
                    session.description.as_deref(),
                );
                let effective_note = Self::effective_override(
                    panel.note.as_deref(),
                    part.note.as_deref(),
                    session.note.as_deref(),
                );
                let effective_prereq = Self::effective_override(
                    panel.prereq.as_deref(),
                    part.prereq.as_deref(),
                    session.prereq.as_deref(),
                );
                let effective_alt_panelist = Self::effective_override(
                    panel.alt_panelist.as_deref(),
                    part.alt_panelist.as_deref(),
                    session.alt_panelist.as_deref(),
                );

                let mut effective_presenters = panel.credited_presenters.clone();
                for name in &part.credited_presenters {
                    if !effective_presenters.contains(name) {
                        effective_presenters.push(name.clone());
                    }
                }
                for name in &session.credited_presenters {
                    if !effective_presenters.contains(name) {
                        effective_presenters.push(name.clone());
                    }
                }

                let room_names = session
                    .room_ids
                    .iter()
                    .filter_map(|rid| {
                        rooms
                            .iter()
                            .find(|(uid, _)| uid == rid)
                            .map(|(_, name)| name.clone())
                    })
                    .collect();

                entries.push(SessionEntry {
                    session_id: session.id.clone(),
                    label,
                    effective_description,
                    effective_note,
                    effective_prereq,
                    effective_alt_panelist,
                    effective_presenters,
                    room_names,
                    start_dt,
                    end_dt,
                    duration: session.duration,
                    is_full: session.is_full,
                    hide_panelist: session.hide_panelist,
                    capacity: session.capacity.clone(),
                    notes_non_printing: session.notes_non_printing.clone(),
                    workshop_notes: session.workshop_notes.clone(),
                    power_needs: session.power_needs.clone(),
                    sewing_machines: session.sewing_machines,
                    av_notes: session.av_notes.clone(),
                });
            }
        }
        entries
    }

    fn effective_concat(
        base: Option<&str>,
        part: Option<&str>,
        session: Option<&str>,
    ) -> Option<String> {
        let parts: Vec<&str> = [base, part, session]
            .iter()
            .filter_map(|s| s.filter(|s| !s.is_empty()))
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }

    fn effective_override(
        base: Option<&str>,
        part: Option<&str>,
        session: Option<&str>,
    ) -> Option<String> {
        session
            .filter(|s| !s.is_empty())
            .or_else(|| part.filter(|s| !s.is_empty()))
            .or_else(|| base.filter(|s| !s.is_empty()))
            .map(str::to_string)
    }
}

impl Render for DetailPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border_color = rgb(0xE5E7EB);
        let label_color = rgb(0x6B7280);
        let base_id = SharedString::from(self.base_id.clone());

        let mut outer = div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(rgb(0xFFFFFF))
            .text_color(rgb(0x111827));

        // ── Header: ID badge + Edit button + close ───────────────────
        let base_id_for_edit = self.base_id.clone();
        let session_id_for_edit = self
            .sessions
            .get(self.selected_idx)
            .map(|s| s.session_id.clone())
            .unwrap_or_default();
        outer = outer.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(12.0))
                .py(px(8.0))
                .border_b_1()
                .border_color(border_color)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .px(px(8.0))
                                .py(px(3.0))
                                .bg(rgb(0xE0E7FF))
                                .rounded(px(4.0))
                                .text_xs()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(rgb(0x3730A3))
                                .child(base_id),
                        )
                        .child(
                            div()
                                .id("detail-edit-btn")
                                .px(px(10.0))
                                .py(px(4.0))
                                .bg(rgb(0x2563EB))
                                .hover(|s| s.bg(rgb(0x1D4ED8)))
                                .rounded(px(4.0))
                                .text_xs()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(rgb(0xFFFFFF))
                                .cursor_pointer()
                                .child("Edit")
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |_this, _ev, _window, cx| {
                                        cx.emit(DetailPaneEvent::OpenEdit {
                                            base_id: base_id_for_edit.clone(),
                                            session_id: session_id_for_edit.clone(),
                                        });
                                    }),
                                ),
                        ),
                )
                .child(
                    div()
                        .id("detail-close-btn")
                        .px(px(8.0))
                        .py(px(4.0))
                        .text_sm()
                        .text_color(label_color)
                        .cursor_pointer()
                        .child("\u{2715}")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_this, _ev, _window, cx| {
                                cx.emit(DetailPaneEvent::Close);
                            }),
                        ),
                ),
        );

        // ── Session selector dropdown ─────────────────────────────────
        let dropdown_open = self.session_dropdown_open;
        let has_multiple = self.sessions.len() > 1;
        let current_label = self
            .sessions
            .get(self.selected_idx)
            .map(|s| s.label.clone())
            .unwrap_or_else(|| SharedString::from("No sessions"));

        let session_selector =
            div()
                .px(px(12.0))
                .py(px(6.0))
                .border_b_1()
                .border_color(border_color)
                .child(
                    div()
                        .id("session-selector")
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(8.0))
                        .py(px(5.0))
                        .border_1()
                        .border_color(rgb(0xD1D5DB))
                        .rounded(px(4.0))
                        .when(has_multiple, |d| d.cursor_pointer())
                        .child(div().text_xs().child(current_label))
                        .when(has_multiple, |d| {
                            d.child(div().text_xs().text_color(label_color).child(
                                if dropdown_open {
                                    "\u{25b2}"
                                } else {
                                    "\u{25bc}"
                                },
                            ))
                        })
                        .when(has_multiple, |d| {
                            d.on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _ev, _window, cx| {
                                    this.session_dropdown_open = !this.session_dropdown_open;
                                    cx.notify();
                                }),
                            )
                        }),
                );

        let mut session_section = div().flex().flex_col();
        session_section = session_section.child(session_selector);

        if dropdown_open {
            let mut list = div()
                .mx(px(12.0))
                .mb(px(6.0))
                .border_1()
                .border_color(rgb(0xD1D5DB))
                .rounded(px(4.0))
                .overflow_hidden();
            for (idx, entry) in self.sessions.iter().enumerate() {
                let is_selected = idx == self.selected_idx;
                let bg = if is_selected {
                    rgb(0xEFF6FF)
                } else {
                    rgb(0xFFFFFF)
                };
                let label = entry.label.clone();
                let item_id = SharedString::from(format!("session-opt-{idx}"));
                list = list.child(
                    div()
                        .id(item_id)
                        .px(px(10.0))
                        .py(px(6.0))
                        .bg(bg)
                        .text_xs()
                        .cursor_pointer()
                        .child(label)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _ev, _window, cx| {
                                this.selected_idx = idx;
                                this.session_dropdown_open = false;
                                cx.notify();
                            }),
                        ),
                );
            }
            session_section = session_section.child(list);
        }
        outer = outer.child(session_section);

        // ── Panel name ───────────────────────────────────────────────
        outer = outer.child(
            div()
                .px(px(12.0))
                .py(px(10.0))
                .border_b_1()
                .border_color(border_color)
                .text_sm()
                .font_weight(gpui::FontWeight::BOLD)
                .child(SharedString::from(self.panel_name.clone())),
        );

        // ── Description list ─────────────────────────────────────────
        if let Some(entry) = self.sessions.get(self.selected_idx) {
            let time_range = match (entry.start_dt, entry.end_dt) {
                (Some(start), Some(end)) => SharedString::from(format!(
                    "{} – {}",
                    start.format("%-I:%M %p"),
                    end.format("%-I:%M %p")
                )),
                (Some(start), None) => SharedString::from(start.format("%-I:%M %p").to_string()),
                _ => SharedString::from("—"),
            };
            let date_str = entry
                .start_dt
                .map(|dt| SharedString::from(dt.format("%A, %B %-d, %Y").to_string()))
                .unwrap_or_else(|| SharedString::from("—"));
            let duration_str = SharedString::from(format!("{} min", entry.duration));
            let rooms_str = if entry.room_names.is_empty() {
                SharedString::from("—")
            } else {
                SharedString::from(entry.room_names.join(", "))
            };
            let presenters_str = if entry.effective_presenters.is_empty() {
                SharedString::from("—")
            } else {
                SharedString::from(entry.effective_presenters.join(", "))
            };

            let mut dl = DescriptionList::new().columns(1).label_width(px(100.0));

            if let Some(ref type_name) = self.panel_type_name {
                dl = dl.item("Type", SharedString::from(type_name.clone()), 1);
            }
            dl = dl
                .item("Date", date_str, 1)
                .item("Time", time_range, 1)
                .item("Duration", duration_str, 1)
                .item("Room", rooms_str, 1)
                .item("Guests", presenters_str, 1);

            if let Some(ref desc) = entry.effective_description {
                dl = dl.item("Description", SharedString::from(desc.clone()), 1);
            }
            if let Some(ref note) = entry.effective_note {
                dl = dl.item("Note", SharedString::from(note.clone()), 1);
            }
            if let Some(ref prereq) = entry.effective_prereq {
                dl = dl.item("Prerequisite", SharedString::from(prereq.clone()), 1);
            }
            if let Some(ref alt) = entry.effective_alt_panelist {
                dl = dl.item("Alt Panelist", SharedString::from(alt.clone()), 1);
            }
            if let Some(ref cost) = self.panel_cost {
                dl = dl.item("Cost", SharedString::from(cost.clone()), 1);
            }
            if let Some(ref cap) = entry.capacity {
                dl = dl.item("Capacity", SharedString::from(cap.clone()), 1);
            }
            if let Some(ref diff) = self.panel_difficulty {
                dl = dl.item("Difficulty", SharedString::from(diff.clone()), 1);
            }

            let mut flags: Vec<&str> = Vec::new();
            if self.panel_is_free {
                flags.push("Free");
            }
            if self.panel_is_kids {
                flags.push("Kids");
            }
            if entry.is_full {
                flags.push("Full");
            }
            if entry.hide_panelist {
                flags.push("Hide Panelist");
            }
            if !flags.is_empty() {
                dl = dl.item("Flags", SharedString::from(flags.join(", ")), 1);
            }
            if let Some(ref wn) = entry.workshop_notes {
                dl = dl.item("Workshop", SharedString::from(wn.clone()), 1);
            }
            if let Some(ref pn) = entry.power_needs {
                dl = dl.item("Power", SharedString::from(pn.clone()), 1);
            }
            if entry.sewing_machines {
                dl = dl.item("Sewing", SharedString::from("Required"), 1);
            }
            if let Some(ref av) = entry.av_notes {
                dl = dl.item("AV", SharedString::from(av.clone()), 1);
            }
            if let Some(ref notes) = entry.notes_non_printing {
                dl = dl.item("Notes", SharedString::from(notes.clone()), 1);
            }

            outer = outer.child(
                div()
                    .id("detail-scroll")
                    .flex_grow()
                    .overflow_y_scroll()
                    .p(px(12.0))
                    .child(dl),
            );
        } else {
            outer = outer.child(
                div()
                    .flex_grow()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .text_color(label_color)
                    .child("No session data"),
            );
        }

        outer
    }
}
