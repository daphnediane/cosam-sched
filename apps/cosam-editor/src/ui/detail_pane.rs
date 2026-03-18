/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use gpui::prelude::*;
use gpui::{Context, Entity, EventEmitter, MouseButton, SharedString, Window, div, px, rgb};
use gpui_component::Sizable;
use gpui_component::calendar::Date;
use gpui_component::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::data::Event;
use crate::data::source_info::ChangeState;

pub enum DetailPaneMode {
    Editing(Event),
    Creating {
        day: Option<NaiveDate>,
        default_start: Option<NaiveDateTime>,
        default_room: Option<u32>,
        default_panel_type: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub enum DetailPaneEvent {
    Save(Event),
    Delete(String),
    Cancel,
}

impl EventEmitter<DetailPaneEvent> for DetailPane {}

pub struct DetailPane {
    draft_id: String,
    draft_name: String,
    draft_description: String,
    draft_date: Option<NaiveDate>,
    draft_start_hour: u32,
    draft_start_minute: u32,
    draft_end_hour: u32,
    draft_end_minute: u32,
    draft_duration_minutes: u32,
    draft_room_id: Option<u32>,
    draft_panel_type_uid: Option<String>,
    draft_presenters: Vec<String>,
    draft_cost: String,
    draft_capacity: String,
    draft_difficulty: String,
    draft_note: String,
    draft_prereq: String,
    draft_ticket_url: String,
    draft_alt_panelist: String,
    draft_is_free: bool,
    draft_is_full: bool,
    draft_is_kids: bool,
    draft_hide_panelist: bool,

    original_event: Option<Event>,
    is_new: bool,

    rooms: Vec<(u32, String)>,
    panel_types: Vec<(String, String)>,
    presenter_names: Vec<String>,

    presenter_panel_open: bool,
    room_dropdown_open: bool,
    panel_type_dropdown_open: bool,

    initialized: bool,
    name_input: Option<Entity<InputState>>,
    description_input: Option<Entity<InputState>>,
    event_id_input: Option<Entity<InputState>>,
    date_picker: Option<Entity<DatePickerState>>,
    cost_input: Option<Entity<InputState>>,
    capacity_input: Option<Entity<InputState>>,
    difficulty_input: Option<Entity<InputState>>,
    note_input: Option<Entity<InputState>>,
    prereq_input: Option<Entity<InputState>>,
    ticket_url_input: Option<Entity<InputState>>,
    alt_panelist_input: Option<Entity<InputState>>,

    _subscriptions: Vec<gpui::Subscription>,
}

impl DetailPane {
    pub fn new(
        mode: DetailPaneMode,
        rooms: Vec<(u32, String)>,
        panel_types: Vec<(String, String)>,
        presenter_names: Vec<String>,
        _cx: &mut Context<Self>,
    ) -> Self {
        let (
            draft_id,
            draft_name,
            draft_description,
            draft_date,
            draft_start_hour,
            draft_start_minute,
            draft_end_hour,
            draft_end_minute,
            draft_duration_minutes,
            draft_room_id,
            draft_panel_type_uid,
            draft_presenters,
            draft_cost,
            draft_capacity,
            draft_difficulty,
            draft_note,
            draft_prereq,
            draft_ticket_url,
            draft_alt_panelist,
            draft_is_free,
            draft_is_full,
            draft_is_kids,
            draft_hide_panelist,
            original_event,
            is_new,
        ) = match mode {
            DetailPaneMode::Editing(event) => {
                let start = event.start_time;
                let end = event.end_time;
                let dur = event.duration;
                (
                    event.id.clone(),
                    event.name.clone(),
                    event.description.clone().unwrap_or_default(),
                    Some(start.date()),
                    start.hour(),
                    start.minute(),
                    end.hour(),
                    end.minute(),
                    dur,
                    event.room_id,
                    event.panel_type.clone(),
                    event.presenters.clone(),
                    event.cost.clone().unwrap_or_default(),
                    event.capacity.clone().unwrap_or_default(),
                    event.difficulty.clone().unwrap_or_default(),
                    event.note.clone().unwrap_or_default(),
                    event.prereq.clone().unwrap_or_default(),
                    event.ticket_url.clone().unwrap_or_default(),
                    event.alt_panelist.clone().unwrap_or_default(),
                    event.is_free,
                    event.is_full,
                    event.is_kids,
                    event.hide_panelist,
                    Some(event),
                    false,
                )
            }
            DetailPaneMode::Creating {
                day,
                default_start,
                default_room,
                default_panel_type,
            } => {
                let (start_h, start_m, end_h, end_m) = if let Some(dt) = default_start {
                    let end = dt + chrono::Duration::minutes(60);
                    (dt.hour(), dt.minute(), end.hour(), end.minute())
                } else {
                    (9, 0, 10, 0)
                };
                (
                    String::new(),
                    String::new(),
                    String::new(),
                    day,
                    start_h,
                    start_m,
                    end_h,
                    end_m,
                    60u32,
                    default_room,
                    default_panel_type,
                    Vec::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    false,
                    false,
                    false,
                    false,
                    None,
                    true,
                )
            }
        };

        Self {
            draft_id,
            draft_name,
            draft_description,
            draft_date,
            draft_start_hour,
            draft_start_minute,
            draft_end_hour,
            draft_end_minute,
            draft_duration_minutes,
            draft_room_id,
            draft_panel_type_uid,
            draft_presenters,
            draft_cost,
            draft_capacity,
            draft_difficulty,
            draft_note,
            draft_prereq,
            draft_ticket_url,
            draft_alt_panelist,
            draft_is_free,
            draft_is_full,
            draft_is_kids,
            draft_hide_panelist,
            original_event,
            is_new,
            rooms,
            panel_types,
            presenter_names,
            presenter_panel_open: false,
            room_dropdown_open: false,
            panel_type_dropdown_open: false,
            initialized: false,
            name_input: None,
            description_input: None,
            event_id_input: None,
            date_picker: None,
            cost_input: None,
            capacity_input: None,
            difficulty_input: None,
            note_input: None,
            prereq_input: None,
            ticket_url_input: None,
            alt_panelist_input: None,
            _subscriptions: Vec::new(),
        }
    }

    fn recalculate_end_from_duration(&mut self) {
        let start_minutes = self.draft_start_hour * 60 + self.draft_start_minute;
        let end_minutes = start_minutes + self.draft_duration_minutes;
        self.draft_end_hour = (end_minutes / 60) % 24;
        self.draft_end_minute = end_minutes % 60;
    }

    fn recalculate_duration_from_end(&mut self) {
        let start_minutes = self.draft_start_hour * 60 + self.draft_start_minute;
        let end_minutes = self.draft_end_hour * 60 + self.draft_end_minute;
        if end_minutes > start_minutes {
            self.draft_duration_minutes = end_minutes - start_minutes;
        } else {
            self.draft_duration_minutes = 60;
        }
    }

    fn build_event(&self) -> Event {
        let date = self
            .draft_date
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());

        let start_time = NaiveDateTime::new(
            date,
            NaiveTime::from_hms_opt(self.draft_start_hour, self.draft_start_minute, 0)
                .unwrap_or_else(|| NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
        );
        let end_time = NaiveDateTime::new(
            date,
            NaiveTime::from_hms_opt(self.draft_end_hour, self.draft_end_minute, 0)
                .unwrap_or_else(|| NaiveTime::from_hms_opt(10, 0, 0).unwrap()),
        );

        let base = self.original_event.clone().unwrap_or_else(|| Event {
            id: self.draft_id.clone(),
            name: String::new(),
            description: None,
            start_time,
            end_time,
            duration: self.draft_duration_minutes,
            room_id: None,
            panel_type: None,
            cost: None,
            capacity: None,
            difficulty: None,
            note: None,
            prereq: None,
            ticket_url: None,
            presenters: Vec::new(),
            credits: Vec::new(),
            conflicts: Vec::new(),
            is_free: false,
            is_full: false,
            is_kids: false,
            hide_panelist: false,
            alt_panelist: None,
            source: None,
            change_state: ChangeState::Added,
        });

        Event {
            id: self.draft_id.clone(),
            name: self.draft_name.clone(),
            description: if self.draft_description.is_empty() {
                None
            } else {
                Some(self.draft_description.clone())
            },
            start_time,
            end_time,
            duration: self.draft_duration_minutes,
            room_id: self.draft_room_id,
            panel_type: self.draft_panel_type_uid.clone(),
            cost: if self.draft_cost.is_empty() {
                None
            } else {
                Some(self.draft_cost.clone())
            },
            capacity: if self.draft_capacity.is_empty() {
                None
            } else {
                Some(self.draft_capacity.clone())
            },
            difficulty: if self.draft_difficulty.is_empty() {
                None
            } else {
                Some(self.draft_difficulty.clone())
            },
            note: if self.draft_note.is_empty() {
                None
            } else {
                Some(self.draft_note.clone())
            },
            prereq: if self.draft_prereq.is_empty() {
                None
            } else {
                Some(self.draft_prereq.clone())
            },
            ticket_url: if self.draft_ticket_url.is_empty() {
                None
            } else {
                Some(self.draft_ticket_url.clone())
            },
            presenters: self.draft_presenters.clone(),
            credits: base.credits.clone(),
            conflicts: base.conflicts.clone(),
            is_free: self.draft_is_free,
            is_full: self.draft_is_full,
            is_kids: self.draft_is_kids,
            hide_panelist: self.draft_hide_panelist,
            alt_panelist: if self.draft_alt_panelist.is_empty() {
                None
            } else {
                Some(self.draft_alt_panelist.clone())
            },
            source: base.source.clone(),
            change_state: base.change_state,
        }
    }

    fn ensure_initialized(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        macro_rules! make_input {
            ($field:expr, $placeholder:expr, $update:expr) => {{
                let val = $field.clone();
                let input = cx.new(|cx| {
                    let mut state = InputState::new(window, cx);
                    if !val.is_empty() {
                        state = state.default_value(val);
                    }
                    state.placeholder($placeholder)
                });
                let sub = cx.subscribe(&input, $update);
                self._subscriptions.push(sub);
                input
            }};
        }

        let name_input = make_input!(
            self.draft_name,
            "Event name",
            |this: &mut Self, entity: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.draft_name = entity.read(cx).value().to_string();
                }
            }
        );
        self.name_input = Some(name_input);

        let desc_val = self.draft_description.clone();
        let description_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .auto_grow(2, 6)
                .placeholder("Description");
            if !desc_val.is_empty() {
                state = state.default_value(desc_val);
            }
            state
        });
        let sub = cx.subscribe(
            &description_input,
            |this: &mut Self, entity: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.draft_description = entity.read(cx).value().to_string();
                }
            },
        );
        self._subscriptions.push(sub);
        self.description_input = Some(description_input);

        let id_val = self.draft_id.clone();
        let id_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            if !id_val.is_empty() {
                state = state.default_value(id_val);
            }
            state.placeholder("e.g. GP001")
        });
        if self.is_new {
            let sub = cx.subscribe(
                &id_input,
                |this: &mut Self, entity: Entity<InputState>, event: &InputEvent, cx| {
                    if let InputEvent::Change = event {
                        this.draft_id = entity.read(cx).value().to_string();
                    }
                },
            );
            self._subscriptions.push(sub);
        }
        self.event_id_input = Some(id_input);

        let date_picker = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).date_format("%B %d, %Y");
            if let Some(date) = self.draft_date {
                state.set_date(date, window, cx);
            }
            state
        });
        let sub = cx.subscribe(
            &date_picker,
            |this: &mut Self, _entity, event: &DatePickerEvent, _cx| {
                let DatePickerEvent::Change(date) = event;
                if let Date::Single(Some(d)) = date {
                    this.draft_date = Some(*d);
                }
            },
        );
        self._subscriptions.push(sub);
        self.date_picker = Some(date_picker);

        let cost_input = make_input!(
            self.draft_cost,
            "$0.00",
            |this: &mut Self, entity: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.draft_cost = entity.read(cx).value().to_string();
                }
            }
        );
        self.cost_input = Some(cost_input);

        let capacity_input = make_input!(
            self.draft_capacity,
            "Capacity",
            |this: &mut Self, entity: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.draft_capacity = entity.read(cx).value().to_string();
                }
            }
        );
        self.capacity_input = Some(capacity_input);

        let difficulty_input = make_input!(
            self.draft_difficulty,
            "e.g. Beginner",
            |this: &mut Self, entity: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.draft_difficulty = entity.read(cx).value().to_string();
                }
            }
        );
        self.difficulty_input = Some(difficulty_input);

        let note_input = make_input!(
            self.draft_note,
            "Internal note",
            |this: &mut Self, entity: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.draft_note = entity.read(cx).value().to_string();
                }
            }
        );
        self.note_input = Some(note_input);

        let prereq_input = make_input!(
            self.draft_prereq,
            "Prerequisites",
            |this: &mut Self, entity: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.draft_prereq = entity.read(cx).value().to_string();
                }
            }
        );
        self.prereq_input = Some(prereq_input);

        let ticket_url_input = make_input!(
            self.draft_ticket_url,
            "https://...",
            |this: &mut Self, entity: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.draft_ticket_url = entity.read(cx).value().to_string();
                }
            }
        );
        self.ticket_url_input = Some(ticket_url_input);

        let alt_panelist_input = make_input!(
            self.draft_alt_panelist,
            "Alternate panelist name",
            |this: &mut Self, entity: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.draft_alt_panelist = entity.read(cx).value().to_string();
                }
            }
        );
        self.alt_panelist_input = Some(alt_panelist_input);
    }

    fn render_section_header(title: &'static str) -> impl IntoElement {
        div()
            .px(px(16.0))
            .py(px(6.0))
            .bg(rgb(0xF3F4F6))
            .text_xs()
            .font_weight(gpui::FontWeight::BOLD)
            .text_color(rgb(0x6B7280))
            .child(title)
    }

    fn render_field_row(label: &'static str, content: impl IntoElement) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .px(px(16.0))
            .py(px(6.0))
            .gap(px(4.0))
            .child(div().text_xs().text_color(rgb(0x6B7280)).child(label))
            .child(content)
    }

    fn render_time_row(
        hour_dec_id: &'static str,
        hour_inc_id: &'static str,
        minute_dec_id: &'static str,
        minute_inc_id: &'static str,
        hour: u32,
        minute: u32,
        on_hour_dec: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        on_hour_inc: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        on_minute_dec: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        on_minute_inc: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let hour_text = SharedString::from(format!("{:02}", hour));
        let min_text = SharedString::from(format!("{:02}", minute));
        let btn = |bg: gpui::Rgba| {
            div()
                .px(px(8.0))
                .py(px(5.0))
                .bg(bg)
                .text_sm()
                .cursor_pointer()
        };
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(rgb(0xD1D5DB))
                    .rounded(px(4.0))
                    .overflow_hidden()
                    .child(btn(rgb(0xF3F4F6)).id(hour_dec_id).child("−").on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _ev, window, cx| {
                            on_hour_dec(this, window, cx);
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .px(px(10.0))
                            .py(px(5.0))
                            .text_sm()
                            .font_weight(gpui::FontWeight::BOLD)
                            .min_w(px(28.0))
                            .flex()
                            .justify_center()
                            .child(hour_text),
                    )
                    .child(btn(rgb(0xF3F4F6)).id(hour_inc_id).child("+").on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _ev, window, cx| {
                            on_hour_inc(this, window, cx);
                            cx.notify();
                        }),
                    )),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(":"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(rgb(0xD1D5DB))
                    .rounded(px(4.0))
                    .overflow_hidden()
                    .child(
                        btn(rgb(0xF3F4F6))
                            .id(minute_dec_id)
                            .child("−")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _ev, window, cx| {
                                    on_minute_dec(this, window, cx);
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .px(px(10.0))
                            .py(px(5.0))
                            .text_sm()
                            .font_weight(gpui::FontWeight::BOLD)
                            .min_w(px(28.0))
                            .flex()
                            .justify_center()
                            .child(min_text),
                    )
                    .child(
                        btn(rgb(0xF3F4F6))
                            .id(minute_inc_id)
                            .child("+")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _ev, window, cx| {
                                    on_minute_inc(this, window, cx);
                                    cx.notify();
                                }),
                            ),
                    ),
            )
    }

    fn render_checkbox<F: Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static>(
        id: SharedString,
        label: &'static str,
        checked: bool,
        on_toggle: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<F> {
        div()
            .id(id)
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(16.0))
            .py(px(4.0))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _ev, window, cx| {
                    on_toggle(this, window, cx);
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .border_1()
                    .border_color(if checked {
                        rgb(0x2563EB)
                    } else {
                        rgb(0xD1D5DB)
                    })
                    .bg(if checked {
                        rgb(0x2563EB)
                    } else {
                        rgb(0xFFFFFF)
                    })
                    .rounded(px(3.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(checked, |d| {
                        d.child(div().text_xs().text_color(rgb(0xFFFFFF)).child("✓"))
                    }),
            )
            .child(div().text_sm().text_color(rgb(0x374151)).child(label))
    }
}

impl Render for DetailPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_initialized(window, cx);

        let label_color = rgb(0x6B7280);
        let border_color = rgb(0xE5E7EB);
        let id_display = if self.draft_id.is_empty() {
            SharedString::from("NEW")
        } else {
            SharedString::from(self.draft_id.clone())
        };

        let is_new = self.is_new;

        let mut pane = div()
            .flex()
            .flex_col()
            .w_full()
            .text_color(rgb(0x111827))
            .bg(rgb(0xFFFFFF));

        // ── ID badge (top) ──────────────────────────────────────────
        pane = pane.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(16.0))
                .py(px(10.0))
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
                                .child(id_display),
                        )
                        .when(is_new, |d| {
                            if let Some(ref input) = self.event_id_input {
                                d.child(Input::new(input).small())
                            } else {
                                d
                            }
                        }),
                )
                .child(
                    div()
                        .id("detail-close-btn")
                        .px(px(8.0))
                        .py(px(4.0))
                        .text_sm()
                        .text_color(label_color)
                        .cursor_pointer()
                        .child("✕")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_this, _ev, _window, cx| {
                                cx.emit(DetailPaneEvent::Cancel);
                            }),
                        ),
                ),
        );

        // ── Name ────────────────────────────────────────────────────
        if let Some(ref input) = self.name_input {
            pane = pane
                .child(Self::render_section_header("EVENT"))
                .child(Self::render_field_row("Name", Input::new(input)));
        }

        // ── Description ─────────────────────────────────────────────
        if let Some(ref input) = self.description_input {
            pane = pane.child(Self::render_field_row("Description", Input::new(input)));
        }

        // ── Date & Time ─────────────────────────────────────────────
        pane = pane.child(Self::render_section_header("DATE & TIME"));

        if let Some(ref picker) = self.date_picker {
            pane = pane.child(Self::render_field_row("Date", DatePicker::new(picker)));
        }

        let start_time = Self::render_time_row(
            "start-h-dec",
            "start-h-inc",
            "start-m-dec",
            "start-m-inc",
            self.draft_start_hour,
            self.draft_start_minute,
            |this, _w, _cx| {
                this.draft_start_hour = (this.draft_start_hour + 23) % 24;
                this.recalculate_duration_from_end();
            },
            |this, _w, _cx| {
                this.draft_start_hour = (this.draft_start_hour + 1) % 24;
                this.recalculate_duration_from_end();
            },
            |this, _w, _cx| {
                this.draft_start_minute = (this.draft_start_minute + 55) % 60;
                this.recalculate_duration_from_end();
            },
            |this, _w, _cx| {
                this.draft_start_minute = (this.draft_start_minute + 5) % 60;
                this.recalculate_duration_from_end();
            },
            cx,
        );
        pane = pane.child(Self::render_field_row("Start Time", start_time));

        let dur_text = SharedString::from(format!("{} min", self.draft_duration_minutes));
        let duration_row = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .id("dur-down")
                    .px(px(10.0))
                    .py(px(4.0))
                    .bg(rgb(0xE5E7EB))
                    .rounded(px(4.0))
                    .text_sm()
                    .cursor_pointer()
                    .child("−")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _ev, _window, cx| {
                            if this.draft_duration_minutes >= 5 {
                                this.draft_duration_minutes -= 5;
                            }
                            this.recalculate_end_from_duration();
                            cx.notify();
                        }),
                    ),
            )
            .child(
                div()
                    .px(px(12.0))
                    .py(px(4.0))
                    .bg(rgb(0xF9FAFB))
                    .border_1()
                    .border_color(rgb(0xD1D5DB))
                    .rounded(px(4.0))
                    .text_sm()
                    .font_weight(gpui::FontWeight::BOLD)
                    .min_w(px(80.0))
                    .flex()
                    .justify_center()
                    .child(dur_text),
            )
            .child(
                div()
                    .id("dur-up")
                    .px(px(10.0))
                    .py(px(4.0))
                    .bg(rgb(0xE5E7EB))
                    .rounded(px(4.0))
                    .text_sm()
                    .cursor_pointer()
                    .child("+")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _ev, _window, cx| {
                            this.draft_duration_minutes += 5;
                            this.recalculate_end_from_duration();
                            cx.notify();
                        }),
                    ),
            );
        pane = pane.child(Self::render_field_row("Duration", duration_row));

        let end_time = Self::render_time_row(
            "end-h-dec",
            "end-h-inc",
            "end-m-dec",
            "end-m-inc",
            self.draft_end_hour,
            self.draft_end_minute,
            |this, _w, _cx| {
                this.draft_end_hour = (this.draft_end_hour + 23) % 24;
                this.recalculate_duration_from_end();
            },
            |this, _w, _cx| {
                this.draft_end_hour = (this.draft_end_hour + 1) % 24;
                this.recalculate_duration_from_end();
            },
            |this, _w, _cx| {
                this.draft_end_minute = (this.draft_end_minute + 55) % 60;
                this.recalculate_duration_from_end();
            },
            |this, _w, _cx| {
                this.draft_end_minute = (this.draft_end_minute + 5) % 60;
                this.recalculate_duration_from_end();
            },
            cx,
        );
        pane = pane.child(Self::render_field_row("End Time", end_time));

        // ── Room ────────────────────────────────────────────────────
        pane = pane.child(Self::render_section_header("LOCATION & TYPE"));

        let selected_room_name = self
            .draft_room_id
            .and_then(|id| self.rooms.iter().find(|(uid, _)| *uid == id))
            .map(|(_, name)| name.as_str())
            .unwrap_or("— No Room —");
        let room_open = self.room_dropdown_open;
        let mut room_widget = div().flex().flex_col().gap(px(2.0));
        let room_toggle = div()
            .id("room-toggle")
            .flex()
            .items_center()
            .justify_between()
            .px(px(10.0))
            .py(px(6.0))
            .border_1()
            .border_color(rgb(0xD1D5DB))
            .rounded(px(4.0))
            .cursor_pointer()
            .child(
                div()
                    .text_sm()
                    .child(SharedString::from(selected_room_name.to_string())),
            )
            .child(div().text_xs().text_color(label_color).child(if room_open {
                "▲"
            } else {
                "▼"
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _ev, _window, cx| {
                    this.room_dropdown_open = !this.room_dropdown_open;
                    cx.notify();
                }),
            );
        room_widget = room_widget.child(room_toggle);
        if room_open {
            let rooms = self.rooms.clone();
            let current_room = self.draft_room_id;
            let mut list = div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(rgb(0xD1D5DB))
                .rounded(px(4.0))
                .overflow_hidden();
            let none_bg = if current_room.is_none() {
                rgb(0xEFF6FF)
            } else {
                rgb(0xFFFFFF)
            };
            list = list.child(
                div()
                    .id("room-none")
                    .px(px(10.0))
                    .py(px(6.0))
                    .bg(none_bg)
                    .text_sm()
                    .cursor_pointer()
                    .child("— No Room —")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _ev, _window, cx| {
                            this.draft_room_id = None;
                            this.room_dropdown_open = false;
                            cx.notify();
                        }),
                    ),
            );
            for (uid, name) in rooms {
                let is_sel = current_room == Some(uid);
                let bg = if is_sel { rgb(0xEFF6FF) } else { rgb(0xFFFFFF) };
                let name_str = SharedString::from(name);
                let item_id = SharedString::from(format!("room-{uid}"));
                list = list.child(
                    div()
                        .id(item_id)
                        .px(px(10.0))
                        .py(px(6.0))
                        .bg(bg)
                        .text_sm()
                        .cursor_pointer()
                        .child(name_str)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _ev, _window, cx| {
                                this.draft_room_id = Some(uid);
                                this.room_dropdown_open = false;
                                cx.notify();
                            }),
                        ),
                );
            }
            room_widget = room_widget.child(list);
        }
        pane = pane.child(Self::render_field_row("Room", room_widget));

        // ── Panel Type ──────────────────────────────────────────────
        let selected_pt_name = self
            .draft_panel_type_uid
            .as_deref()
            .and_then(|uid| self.panel_types.iter().find(|(u, _)| u == uid))
            .map(|(_, kind)| kind.as_str())
            .unwrap_or("— None —");
        let pt_open = self.panel_type_dropdown_open;
        let mut pt_widget = div().flex().flex_col().gap(px(2.0));
        let pt_toggle = div()
            .id("pt-toggle")
            .flex()
            .items_center()
            .justify_between()
            .px(px(10.0))
            .py(px(6.0))
            .border_1()
            .border_color(rgb(0xD1D5DB))
            .rounded(px(4.0))
            .cursor_pointer()
            .child(
                div()
                    .text_sm()
                    .child(SharedString::from(selected_pt_name.to_string())),
            )
            .child(div().text_xs().text_color(label_color).child(if pt_open {
                "▲"
            } else {
                "▼"
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _ev, _window, cx| {
                    this.panel_type_dropdown_open = !this.panel_type_dropdown_open;
                    cx.notify();
                }),
            );
        pt_widget = pt_widget.child(pt_toggle);
        if pt_open {
            let pts = self.panel_types.clone();
            let current_pt = self.draft_panel_type_uid.clone();
            let mut list = div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(rgb(0xD1D5DB))
                .rounded(px(4.0))
                .overflow_hidden();
            for (uid, kind) in pts {
                let is_sel = current_pt.as_deref() == Some(uid.as_str());
                let bg = if is_sel { rgb(0xEFF6FF) } else { rgb(0xFFFFFF) };
                let kind_str = SharedString::from(kind);
                let item_id = SharedString::from(format!("pt-{uid}"));
                let uid_clone = uid.clone();
                list = list.child(
                    div()
                        .id(item_id)
                        .px(px(10.0))
                        .py(px(6.0))
                        .bg(bg)
                        .text_sm()
                        .cursor_pointer()
                        .child(kind_str)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _ev, _window, cx| {
                                this.draft_panel_type_uid = Some(uid_clone.clone());
                                this.panel_type_dropdown_open = false;
                                cx.notify();
                            }),
                        ),
                );
            }
            pt_widget = pt_widget.child(list);
        }
        pane = pane.child(Self::render_field_row("Panel Type", pt_widget));

        // ── Presenters ──────────────────────────────────────────────
        pane = pane.child(Self::render_section_header("GUESTS"));
        let presenter_open = self.presenter_panel_open;
        let selected_count = self.draft_presenters.len();
        let presenters_label = if selected_count == 0 {
            SharedString::from("Select guests…")
        } else {
            SharedString::from(format!("{selected_count} selected"))
        };
        let presenter_toggle = div()
            .id("presenter-toggle")
            .px(px(16.0))
            .py(px(6.0))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x374151))
                    .child(presenters_label),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(label_color)
                    .child(if presenter_open { "▲" } else { "▼" }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _ev, _window, cx| {
                    this.presenter_panel_open = !this.presenter_panel_open;
                    cx.notify();
                }),
            );
        pane = pane.child(presenter_toggle);

        if presenter_open {
            let names = self.presenter_names.clone();
            let selected = self.draft_presenters.clone();
            for name in names {
                let is_checked = selected.contains(&name);
                let name_clone = name.clone();
                let checkbox_id = SharedString::from(format!("presenter-{}", name));
                pane = pane.child(
                    div()
                        .id(checkbox_id)
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(24.0))
                        .py(px(4.0))
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _ev, _window, cx| {
                                let n = name_clone.clone();
                                if this.draft_presenters.contains(&n) {
                                    this.draft_presenters.retain(|p| p != &n);
                                } else {
                                    this.draft_presenters.push(n);
                                }
                                cx.notify();
                            }),
                        )
                        .child(
                            div()
                                .w(px(14.0))
                                .h(px(14.0))
                                .border_1()
                                .border_color(if is_checked {
                                    rgb(0x2563EB)
                                } else {
                                    rgb(0xD1D5DB)
                                })
                                .bg(if is_checked {
                                    rgb(0x2563EB)
                                } else {
                                    rgb(0xFFFFFF)
                                })
                                .rounded(px(2.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(is_checked, |d| {
                                    d.child(div().text_xs().text_color(rgb(0xFFFFFF)).child("✓"))
                                }),
                        )
                        .child(div().text_sm().child(SharedString::from(name))),
                );
            }
        }

        if !self.draft_presenters.is_empty() {
            let names_text = SharedString::from(self.draft_presenters.join(", "));
            pane = pane.child(
                div()
                    .px(px(16.0))
                    .py(px(4.0))
                    .text_xs()
                    .text_color(label_color)
                    .child(names_text),
            );
        }

        // Panelist display options
        let hide_panelist_check = Self::render_checkbox(
            SharedString::from("check-hide-panelist"),
            "Hide panelist names",
            self.draft_hide_panelist,
            |this, _w, _cx| {
                this.draft_hide_panelist = !this.draft_hide_panelist;
            },
            cx,
        );
        pane = pane.child(hide_panelist_check);

        if let Some(ref input) = self.alt_panelist_input {
            pane = pane.child(Self::render_field_row(
                "Alt Panelist Display",
                Input::new(input).small(),
            ));
        }

        // ── Optional fields ─────────────────────────────────────────
        pane = pane.child(Self::render_section_header("OPTIONAL"));
        if let Some(ref input) = self.cost_input {
            pane = pane.child(Self::render_field_row("Cost", Input::new(input).small()));
        }
        if let Some(ref input) = self.capacity_input {
            pane = pane.child(Self::render_field_row(
                "Capacity",
                Input::new(input).small(),
            ));
        }
        if let Some(ref input) = self.difficulty_input {
            pane = pane.child(Self::render_field_row(
                "Difficulty",
                Input::new(input).small(),
            ));
        }
        if let Some(ref input) = self.note_input {
            pane = pane.child(Self::render_field_row(
                "Internal Note",
                Input::new(input).small(),
            ));
        }
        if let Some(ref input) = self.prereq_input {
            pane = pane.child(Self::render_field_row(
                "Prerequisite",
                Input::new(input).small(),
            ));
        }
        if let Some(ref input) = self.ticket_url_input {
            pane = pane.child(Self::render_field_row(
                "Ticket URL",
                Input::new(input).small(),
            ));
        }

        // ── Flags ───────────────────────────────────────────────────
        pane = pane.child(Self::render_section_header("FLAGS"));
        let is_free_check = Self::render_checkbox(
            SharedString::from("check-is-free"),
            "Free event",
            self.draft_is_free,
            |this, _w, _cx| {
                this.draft_is_free = !this.draft_is_free;
            },
            cx,
        );
        let is_full_check = Self::render_checkbox(
            SharedString::from("check-is-full"),
            "Event is full",
            self.draft_is_full,
            |this, _w, _cx| {
                this.draft_is_full = !this.draft_is_full;
            },
            cx,
        );
        let is_kids_check = Self::render_checkbox(
            SharedString::from("check-is-kids"),
            "Kids event",
            self.draft_is_kids,
            |this, _w, _cx| {
                this.draft_is_kids = !this.draft_is_kids;
            },
            cx,
        );
        pane = pane
            .child(is_free_check)
            .child(is_full_check)
            .child(is_kids_check);

        // ── Buttons ─────────────────────────────────────────────────
        pane = pane.child(div().h(px(1.0)).bg(border_color).mx(px(16.0)).mt(px(12.0)));

        if is_new {
            pane = pane.child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .px(px(16.0))
                    .py(px(12.0))
                    .child(
                        div()
                            .id("detail-add-btn")
                            .flex_grow()
                            .px(px(16.0))
                            .py(px(8.0))
                            .bg(rgb(0x2563EB))
                            .hover(|s| s.bg(rgb(0x1D4ED8)))
                            .rounded(px(6.0))
                            .text_sm()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(0xFFFFFF))
                            .flex()
                            .justify_center()
                            .cursor_pointer()
                            .child("Add")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _ev, _window, cx| {
                                    let event = this.build_event();
                                    cx.emit(DetailPaneEvent::Save(event));
                                }),
                            ),
                    )
                    .child(
                        div()
                            .id("detail-cancel-btn")
                            .px(px(16.0))
                            .py(px(8.0))
                            .bg(rgb(0xF3F4F6))
                            .hover(|s| s.bg(rgb(0xE5E7EB)))
                            .rounded(px(6.0))
                            .text_sm()
                            .text_color(rgb(0x374151))
                            .flex()
                            .justify_center()
                            .cursor_pointer()
                            .child("Cancel")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|_this, _ev, _window, cx| {
                                    cx.emit(DetailPaneEvent::Cancel);
                                }),
                            ),
                    ),
            );
        } else {
            let event_id_for_delete = self.draft_id.clone();
            pane = pane.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .px(px(16.0))
                    .py(px(12.0))
                    .child(
                        div()
                            .id("detail-save-btn")
                            .flex_grow()
                            .px(px(16.0))
                            .py(px(8.0))
                            .bg(rgb(0x2563EB))
                            .hover(|s| s.bg(rgb(0x1D4ED8)))
                            .rounded(px(6.0))
                            .text_sm()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(0xFFFFFF))
                            .flex()
                            .justify_center()
                            .cursor_pointer()
                            .child("Save Changes")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _ev, _window, cx| {
                                    let event = this.build_event();
                                    cx.emit(DetailPaneEvent::Save(event));
                                }),
                            ),
                    )
                    .child(
                        div()
                            .id("detail-delete-btn")
                            .flex_grow()
                            .px(px(16.0))
                            .py(px(10.0))
                            .bg(rgb(0xDC2626))
                            .hover(|s| s.bg(rgb(0xB91C1C)))
                            .rounded(px(6.0))
                            .text_sm()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(0xFFFFFF))
                            .flex()
                            .justify_center()
                            .cursor_pointer()
                            .child("Delete Event")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_this, _ev, _window, cx| {
                                    cx.emit(DetailPaneEvent::Delete(event_id_for_delete.clone()));
                                }),
                            ),
                    ),
            );
        }

        pane.child(div().h(px(24.0)))
    }
}
