/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::prelude::*;
use gpui::{
    div, px, rgb, Context, Entity, EventEmitter, FocusHandle, Focusable, MouseButton, SharedString,
    Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use schedule_core::tables::PanelId;

use crate::ui::schedule_data::PanelDisplayInfo;

pub struct DetailPane {
    focus_handle: FocusHandle,
    pub panel_id: PanelId,
    pub info: PanelDisplayInfo,
    pub name_input: Option<Entity<InputState>>,
}

impl DetailPane {
    pub fn new(info: PanelDisplayInfo, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            panel_id: info.panel_id,
            info,
            name_input: None,
        }
    }

    pub fn update_info(&mut self, info: PanelDisplayInfo) {
        if self.panel_id != info.panel_id {
            self.name_input = None; // reset input when panel changes
        }
        self.panel_id = info.panel_id;
        self.info = info;
    }

    fn ensure_name_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.name_input.is_some() {
            return;
        }
        let name_val = self.info.name.clone();
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name_val));
        // Emit close on Enter for convenience
        cx.subscribe(&input, |_this, _input, event: &InputEvent, cx| {
            if let InputEvent::PressEnter { .. } = event {
                cx.emit(DetailPaneEvent::SaveRequested);
            }
        })
        .detach();
        self.name_input = Some(input);
    }
}

impl Focusable for DetailPane {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DetailPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_name_input(window, cx);

        let panel_id = self.panel_id;
        let code = SharedString::from(self.info.code.clone());
        let time_str = SharedString::from(self.info.time_range_str.clone());
        let rooms_str = SharedString::from(if self.info.room_names.is_empty() {
            "—".to_string()
        } else {
            self.info.room_names.join(", ")
        });

        let mut pane = div()
            .id("detail-pane")
            .flex()
            .flex_col()
            .w(px(300.0))
            .flex_shrink_0()
            .border_l_1()
            .border_color(rgb(0xE5E7EB))
            .bg(rgb(0xFFFFFF))
            .overflow_y_scroll();

        // Header
        pane = pane.child(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .items_center()
                .px(px(14.0))
                .py(px(10.0))
                .border_b_1()
                .border_color(rgb(0xE5E7EB))
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(0x6B7280))
                        .child(code),
                )
                .child(
                    div()
                        .id("detail-close")
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .text_sm()
                        .text_color(rgb(0x6B7280))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xF3F4F6)))
                        .child("×")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_this, _ev, _window, cx| {
                                cx.emit(DetailPaneEvent::Close);
                            }),
                        ),
                ),
        );

        // Name input
        pane = pane.child(
            div()
                .px(px(14.0))
                .pt(px(12.0))
                .pb(px(4.0))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6B7280))
                        .mb(px(4.0))
                        .child("Name"),
                )
                .child(
                    self.name_input
                        .as_ref()
                        .map(|input| Input::new(input).into_any_element())
                        .unwrap_or_else(|| div().child("...").into_any_element()),
                ),
        );

        // Time row
        pane = pane.child(
            div()
                .px(px(14.0))
                .py(px(6.0))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6B7280))
                        .mb(px(2.0))
                        .child("Time"),
                )
                .child(div().text_sm().text_color(rgb(0x374151)).child(time_str)),
        );

        // Rooms row
        pane = pane.child(
            div()
                .px(px(14.0))
                .py(px(6.0))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6B7280))
                        .mb(px(2.0))
                        .child("Room"),
                )
                .child(div().text_sm().text_color(rgb(0x374151)).child(rooms_str)),
        );

        // Description (if present)
        if let Some(ref desc) = self.info.description {
            let desc = SharedString::from(desc.clone());
            pane = pane.child(
                div()
                    .px(px(14.0))
                    .py(px(6.0))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x6B7280))
                            .mb(px(2.0))
                            .child("Description"),
                    )
                    .child(div().text_sm().text_color(rgb(0x374151)).child(desc)),
            );
        }

        // Save button
        let name_input_clone = self.name_input.clone();
        pane = pane.child(
            div().px(px(14.0)).pt(px(12.0)).pb(px(16.0)).child(
                div()
                    .id("detail-save")
                    .px(px(16.0))
                    .py(px(7.0))
                    .rounded(px(6.0))
                    .bg(rgb(0x2563EB))
                    .hover(|s| s.bg(rgb(0x1D4ED8)))
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(0xFFFFFF))
                    .cursor_pointer()
                    .child("Save Name")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_this, _ev, _window, cx| {
                            cx.emit(DetailPaneEvent::SaveRequested);
                        }),
                    ),
            ),
        );

        let _ = name_input_clone; // suppress warning
        let _ = panel_id;
        pane
    }
}

#[derive(Debug, Clone)]
pub enum DetailPaneEvent {
    Close,
    SaveRequested,
}

impl EventEmitter<DetailPaneEvent> for DetailPane {}
