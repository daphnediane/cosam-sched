/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, MouseButton, SharedString, Window,
    div, px, rgb,
};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::data::Panel;

#[derive(Clone, Debug)]
pub enum PanelEditWindowEvent {
    Save(Panel),
    SessionDeleted { base_id: String, session_id: String },
}

impl EventEmitter<PanelEditWindowEvent> for PanelEditWindow {}

struct PanelEditInputs {
    name_input: Entity<InputState>,
    description_base_input: Entity<InputState>,
    note_base_input: Entity<InputState>,
    prereq_base_input: Entity<InputState>,
    cost_input: Entity<InputState>,
    description_part_input: Option<Entity<InputState>>,
    note_part_input: Option<Entity<InputState>>,
    prereq_part_input: Option<Entity<InputState>>,
    start_time_input: Entity<InputState>,
    end_time_input: Entity<InputState>,
    description_session_input: Entity<InputState>,
    note_session_input: Entity<InputState>,
    prereq_session_input: Entity<InputState>,
    capacity_session_input: Entity<InputState>,
    alt_panelist_input: Entity<InputState>,
}

pub struct PanelEditWindow {
    focus_handle: FocusHandle,
    draft_panel: Panel,
    selected_part_idx: usize,
    selected_session_idx: usize,
    rooms: Vec<(u32, String)>,
    panel_types: Vec<(String, String)>,
    _presenter_names: Vec<String>,
    inputs: Option<PanelEditInputs>,
    panel_type_dropdown_open: bool,
    room_dropdown_open: bool,
    pending_save: Option<gpui::Task<()>>,
    _subscriptions: Vec<gpui::Subscription>,
}

impl PanelEditWindow {
    pub fn new(
        panel: Panel,
        selected_session_id: &str,
        rooms: Vec<(u32, String)>,
        panel_types: Vec<(String, String)>,
        presenter_names: Vec<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        let (selected_part_idx, selected_session_idx) =
            Self::find_session_indices(&panel, selected_session_id);
        Self {
            focus_handle: cx.focus_handle(),
            draft_panel: panel,
            selected_part_idx,
            selected_session_idx,
            rooms,
            panel_types,
            _presenter_names: presenter_names,
            inputs: None,
            panel_type_dropdown_open: false,
            room_dropdown_open: false,
            pending_save: None,
            _subscriptions: Vec::new(),
        }
    }

    fn ensure_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.inputs.is_some() {
            return;
        }

        let name_val = self.draft_panel.name.clone();
        let desc_base_val = self.draft_panel.description.clone().unwrap_or_default();
        let note_base_val = self.draft_panel.note.clone().unwrap_or_default();
        let prereq_base_val = self.draft_panel.prereq.clone().unwrap_or_default();
        let cost_val = self.draft_panel.cost.clone().unwrap_or_default();

        let start_val = self.draft_panel.start_time.clone().unwrap_or_default();
        let end_val = self.draft_panel.end_time.clone().unwrap_or_default();
        let desc_sess_val = self.draft_panel.description.clone().unwrap_or_default();
        let note_sess_val = self.draft_panel.note.clone().unwrap_or_default();
        let prereq_sess_val = self.draft_panel.prereq.clone().unwrap_or_default();
        let capacity_val = self.draft_panel.capacity.clone().unwrap_or_default();
        let alt_panelist_val = self.draft_panel.alt_panelist.clone().unwrap_or_default();

        let name_input = cx.new(|cx| InputState::new(window, cx).default_value(name_val));
        let description_base_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(desc_base_val)
                .multi_line(true)
        });
        let note_base_input = cx.new(|cx| InputState::new(window, cx).default_value(note_base_val));
        let prereq_base_input =
            cx.new(|cx| InputState::new(window, cx).default_value(prereq_base_val));
        let cost_input = cx.new(|cx| InputState::new(window, cx).default_value(cost_val));

        let description_part_input: Option<gpui::Entity<InputState>> = None;
        let note_part_input: Option<gpui::Entity<InputState>> = None;
        let prereq_part_input: Option<gpui::Entity<InputState>> = None;

        let start_time_input = cx.new(|cx| InputState::new(window, cx).default_value(start_val));
        let end_time_input = cx.new(|cx| InputState::new(window, cx).default_value(end_val));
        let description_session_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(desc_sess_val)
                .multi_line(true)
        });
        let note_session_input =
            cx.new(|cx| InputState::new(window, cx).default_value(note_sess_val));
        let prereq_session_input =
            cx.new(|cx| InputState::new(window, cx).default_value(prereq_sess_val));
        let capacity_session_input =
            cx.new(|cx| InputState::new(window, cx).default_value(capacity_val));
        let alt_panelist_input =
            cx.new(|cx| InputState::new(window, cx).default_value(alt_panelist_val));

        self._subscriptions.push(cx.subscribe(
            &name_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.name = text.to_string();
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &description_base_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.description = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &note_base_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.note = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &prereq_base_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.prereq = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &cost_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.cost = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &start_time_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.start_time = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &end_time_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.end_time = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &description_session_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.description = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &note_session_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.note = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &prereq_session_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.prereq = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &capacity_session_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.capacity = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));
        self._subscriptions.push(cx.subscribe(
            &alt_panelist_input,
            |this, entity, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = entity.read(cx).value();
                    this.draft_panel.alt_panelist = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    this.schedule_save(cx);
                }
            },
        ));

        let _ = description_part_input.as_ref();
        let _ = note_part_input.as_ref();
        let _ = prereq_part_input.as_ref();

        self.inputs = Some(PanelEditInputs {
            name_input,
            description_base_input,
            note_base_input,
            prereq_base_input,
            cost_input,
            description_part_input,
            note_part_input,
            prereq_part_input,
            start_time_input,
            end_time_input,
            description_session_input,
            note_session_input,
            prereq_session_input,
            capacity_session_input,
            alt_panelist_input,
        });
    }

    fn find_session_indices(_panel: &Panel, _session_id: &str) -> (usize, usize) {
        (0, 0)
    }

    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.pending_save = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            this.update(cx, |view, cx| {
                let panel = view.draft_panel.clone();
                cx.emit(PanelEditWindowEvent::Save(panel));
            })
            .ok();
        }));
    }

    fn delete_current_session(&mut self, cx: &mut Context<Self>) {
        let session_id = self.draft_panel.id.clone();
        let base_id = self.draft_panel.base_id.clone();
        cx.emit(PanelEditWindowEvent::SessionDeleted {
            base_id,
            session_id,
        });
    }

    fn render_field_row(
        label: &str,
        field_id: &str,
        input: &Entity<InputState>,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_start()
            .gap(px(8.0))
            .mb(px(8.0))
            .child(
                div()
                    .w(px(100.0))
                    .flex_shrink_0()
                    .pt(px(6.0))
                    .text_xs()
                    .text_color(rgb(0x6B7280))
                    .child(SharedString::from(label.to_string())),
            )
            .child(
                div()
                    .id(SharedString::from(field_id.to_string()))
                    .flex_grow()
                    .child(Input::new(input)),
            )
    }

    fn render_section_header(title: &str) -> impl IntoElement {
        div()
            .mt(px(16.0))
            .mb(px(8.0))
            .pb(px(4.0))
            .border_b_1()
            .border_color(rgb(0xE5E7EB))
            .text_xs()
            .font_weight(gpui::FontWeight::BOLD)
            .text_color(rgb(0x374151))
            .child(SharedString::from(title.to_string()))
    }
}

impl Render for PanelEditWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_inputs(window, cx);

        let border_color = rgb(0xE5E7EB);
        let bg = rgb(0xFFFFFF);
        let panel_id = SharedString::from(self.draft_panel.id.clone());

        let is_full = self.draft_panel.is_full;
        let hide_panelist = self.draft_panel.hide_panelist;
        let panel_is_free = self.draft_panel.is_free;
        let panel_is_kids = self.draft_panel.is_kids;
        let current_room_id = self.draft_panel.room_ids.first().copied();
        let current_room_name = current_room_id
            .and_then(|rid| self.rooms.iter().find(|(uid, _)| *uid == rid))
            .map(|(_, name): &(u32, String)| name.as_str())
            .unwrap_or("— No room —");
        let current_panel_type_uid = self.draft_panel.panel_type.clone().unwrap_or_default();
        let current_panel_type_name = self
            .panel_types
            .iter()
            .find(|(uid, _)| uid == &current_panel_type_uid)
            .map(|(_, kind)| kind.as_str())
            .unwrap_or("— None —");

        let has_multiple_parts = false;
        let has_sessions = true;
        let total_sessions: usize = 1;

        let inputs = self.inputs.as_ref().expect("ensure_inputs was called");

        let mut outer = div()
            .id("panel-edit-window")
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(bg)
            .text_color(rgb(0x111827));

        // ── Title bar ─────────────────────────────────────────────
        outer = outer.child(
            div()
                .flex()
                .items_center()
                .px(px(16.0))
                .py(px(10.0))
                .gap(px(8.0))
                .border_b_1()
                .border_color(border_color)
                .child(
                    div()
                        .px(px(8.0))
                        .py(px(3.0))
                        .bg(rgb(0xE0E7FF))
                        .rounded(px(4.0))
                        .text_xs()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(0x3730A3))
                        .child(panel_id),
                )
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Edit Panel"),
                ),
        );

        // ── Scrollable form ───────────────────────────────────────
        let mut form = div()
            .id("edit-scroll")
            .flex_grow()
            .overflow_y_scroll()
            .px(px(16.0))
            .py(px(12.0))
            .flex()
            .flex_col();

        // BASE section
        form = form.child(Self::render_section_header("BASE"));
        form = form.child(Self::render_field_row(
            "Name",
            "edit-name",
            &inputs.name_input,
        ));

        // Panel type dropdown
        let panel_type_open = self.panel_type_dropdown_open;
        let type_label = SharedString::from(current_panel_type_name.to_string());
        let mut type_row = div()
            .flex()
            .items_start()
            .gap(px(8.0))
            .mb(px(8.0))
            .child(
                div()
                    .w(px(100.0))
                    .flex_shrink_0()
                    .pt(px(6.0))
                    .text_xs()
                    .text_color(rgb(0x6B7280))
                    .child("Type"),
            )
            .child(
                div().flex_grow().flex().flex_col().child(
                    div()
                        .id("panel-type-selector")
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(8.0))
                        .py(px(5.0))
                        .border_1()
                        .border_color(rgb(0xD1D5DB))
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .child(div().text_xs().child(type_label))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x6B7280))
                                .child(if panel_type_open { "▲" } else { "▼" }),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.panel_type_dropdown_open = !this.panel_type_dropdown_open;
                                this.room_dropdown_open = false;
                                cx.notify();
                            }),
                        ),
                ),
            );
        if panel_type_open {
            let panel_types = self.panel_types.clone();
            let current_uid = current_panel_type_uid.clone();
            let mut list = div()
                .border_1()
                .border_color(rgb(0xD1D5DB))
                .rounded(px(4.0))
                .overflow_hidden()
                .mt(px(2.0));
            for (uid, kind) in &panel_types {
                let is_selected = uid == &current_uid;
                let item_uid = uid.clone();
                let kind_label = SharedString::from(kind.clone());
                list = list.child(
                    div()
                        .id(SharedString::from(format!("pt-{uid}")))
                        .px(px(8.0))
                        .py(px(5.0))
                        .text_xs()
                        .cursor_pointer()
                        .bg(if is_selected {
                            rgb(0xEFF6FF)
                        } else {
                            rgb(0xFFFFFF)
                        })
                        .child(kind_label)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.draft_panel.panel_type = Some(item_uid.clone());
                                this.panel_type_dropdown_open = false;
                                this.schedule_save(cx);
                                cx.notify();
                            }),
                        ),
                );
            }
            type_row = type_row.child(list);
        }
        form = form.child(type_row);

        // is_free / is_kids checkboxes
        form = form.child(
            div()
                .flex()
                .items_center()
                .mb(px(8.0))
                .ml(px(108.0))
                .child(
                    div()
                        .id("chk-is-free")
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .mr(px(16.0))
                        .cursor_pointer()
                        .text_xs()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.draft_panel.is_free = !this.draft_panel.is_free;
                                this.schedule_save(cx);
                                cx.notify();
                            }),
                        )
                        .child(if panel_is_free { "☑" } else { "☐" })
                        .child("Free"),
                )
                .child(
                    div()
                        .id("chk-is-kids")
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .cursor_pointer()
                        .text_xs()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.draft_panel.is_kids = !this.draft_panel.is_kids;
                                this.schedule_save(cx);
                                cx.notify();
                            }),
                        )
                        .child(if panel_is_kids { "☑" } else { "☐" })
                        .child("Kids"),
                ),
        );

        form = form.child(Self::render_field_row(
            "Description",
            "edit-desc-base",
            &inputs.description_base_input,
        ));
        form = form.child(Self::render_field_row(
            "Note",
            "edit-note-base",
            &inputs.note_base_input,
        ));
        form = form.child(Self::render_field_row(
            "Prereq",
            "edit-prereq-base",
            &inputs.prereq_base_input,
        ));
        form = form.child(Self::render_field_row(
            "Cost",
            "edit-cost",
            &inputs.cost_input,
        ));

        // PART section (only when multiple parts)
        if has_multiple_parts {
            if let Some(ref desc_input) = inputs.description_part_input {
                form = form.child(Self::render_section_header(&format!(
                    "PART {}",
                    self.selected_part_idx + 1
                )));
                form = form.child(Self::render_field_row(
                    "Description",
                    "edit-desc-part",
                    desc_input,
                ));
            }
            if let Some(ref note_input) = inputs.note_part_input {
                form = form.child(Self::render_field_row("Note", "edit-note-part", note_input));
            }
            if let Some(ref prereq_input) = inputs.prereq_part_input {
                form = form.child(Self::render_field_row(
                    "Prereq",
                    "edit-prereq-part",
                    prereq_input,
                ));
            }
        }

        // SESSION section
        if has_sessions {
            let session_num = self.selected_session_idx + 1;
            form = form.child(Self::render_section_header(&format!(
                "SESSION {session_num}"
            )));

            form = form.child(Self::render_field_row(
                "Start",
                "edit-start-time",
                &inputs.start_time_input,
            ));
            form = form.child(Self::render_field_row(
                "End",
                "edit-end-time",
                &inputs.end_time_input,
            ));

            // Room dropdown
            let room_open = self.room_dropdown_open;
            let room_label = SharedString::from(current_room_name.to_string());
            let mut room_row = div()
                .flex()
                .items_start()
                .gap(px(8.0))
                .mb(px(8.0))
                .child(
                    div()
                        .w(px(100.0))
                        .flex_shrink_0()
                        .pt(px(6.0))
                        .text_xs()
                        .text_color(rgb(0x6B7280))
                        .child("Room"),
                )
                .child(
                    div().flex_grow().flex().flex_col().child(
                        div()
                            .id("room-selector")
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(8.0))
                            .py(px(5.0))
                            .border_1()
                            .border_color(rgb(0xD1D5DB))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .child(div().text_xs().child(room_label))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x6B7280))
                                    .child(if room_open { "▲" } else { "▼" }),
                            )
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.room_dropdown_open = !this.room_dropdown_open;
                                    this.panel_type_dropdown_open = false;
                                    cx.notify();
                                }),
                            ),
                    ),
                );
            if room_open {
                let rooms = self.rooms.clone();
                let mut list = div()
                    .border_1()
                    .border_color(rgb(0xD1D5DB))
                    .rounded(px(4.0))
                    .overflow_hidden()
                    .mt(px(2.0));
                for (uid, name) in &rooms {
                    let is_selected = Some(*uid) == current_room_id;
                    let item_uid = *uid;
                    let room_name = SharedString::from(name.clone());
                    list = list.child(
                        div()
                            .id(SharedString::from(format!("room-{uid}")))
                            .px(px(8.0))
                            .py(px(5.0))
                            .text_xs()
                            .cursor_pointer()
                            .bg(if is_selected {
                                rgb(0xEFF6FF)
                            } else {
                                rgb(0xFFFFFF)
                            })
                            .child(room_name)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.draft_panel.room_ids = vec![item_uid];
                                    this.room_dropdown_open = false;
                                    this.schedule_save(cx);
                                    cx.notify();
                                }),
                            ),
                    );
                }
                room_row = room_row.child(list);
            }
            form = form.child(room_row);

            form = form.child(Self::render_field_row(
                "Description",
                "edit-desc-session",
                &inputs.description_session_input,
            ));
            form = form.child(Self::render_field_row(
                "Note",
                "edit-note-session",
                &inputs.note_session_input,
            ));
            form = form.child(Self::render_field_row(
                "Prereq",
                "edit-prereq-session",
                &inputs.prereq_session_input,
            ));
            form = form.child(Self::render_field_row(
                "Capacity",
                "edit-capacity-session",
                &inputs.capacity_session_input,
            ));
            form = form.child(Self::render_field_row(
                "Alt Panelist",
                "edit-alt-panelist",
                &inputs.alt_panelist_input,
            ));

            // is_full / hide_panelist checkboxes
            form = form.child(
                div()
                    .flex()
                    .items_center()
                    .mb(px(8.0))
                    .ml(px(108.0))
                    .child(
                        div()
                            .id("chk-is-full")
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .mr(px(16.0))
                            .cursor_pointer()
                            .text_xs()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.draft_panel.is_full = !this.draft_panel.is_full;
                                    this.schedule_save(cx);
                                    cx.notify();
                                }),
                            )
                            .child(if is_full { "☑" } else { "☐" })
                            .child("Full"),
                    )
                    .child(
                        div()
                            .id("chk-hide-panelist")
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .cursor_pointer()
                            .text_xs()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, _, cx| {
                                    this.draft_panel.hide_panelist =
                                        !this.draft_panel.hide_panelist;
                                    this.schedule_save(cx);
                                    cx.notify();
                                }),
                            )
                            .child(if hide_panelist { "☑" } else { "☐" })
                            .child("Hide Panelist"),
                    ),
            );
        }

        outer = outer.child(form);

        // ── Footer: delete session button ─────────────────────────
        if total_sessions > 0 {
            outer = outer.child(
                div()
                    .px(px(16.0))
                    .py(px(10.0))
                    .border_t_1()
                    .border_color(border_color)
                    .child(
                        div()
                            .id("btn-delete-session")
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .py(px(6.0))
                            .bg(rgb(0xFEF2F2))
                            .rounded(px(4.0))
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(0xDC2626))
                            .cursor_pointer()
                            .child("Delete Session")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.delete_current_session(cx);
                                }),
                            ),
                    ),
            );
        }

        outer
    }
}

impl Focusable for PanelEditWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
