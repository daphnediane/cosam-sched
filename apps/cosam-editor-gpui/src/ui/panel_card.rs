/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::prelude::*;
use gpui::{div, px, rgb, Context, EventEmitter, MouseButton, SharedString, Window};
use schedule_core::tables::PanelId;
use schedule_core::ChangeState;

use crate::ui::schedule_data::PanelDisplayInfo;

pub struct PanelCard {
    pub info: PanelDisplayInfo,
    pub is_selected: bool,
}

impl PanelCard {
    pub fn new(info: PanelDisplayInfo, is_selected: bool) -> Self {
        Self { info, is_selected }
    }
}

impl Render for PanelCard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (border_color, bg_color) = match self.info.change_state {
            ChangeState::Added => (rgb(0x16A34A), rgb(0xF0FDF4)),
            ChangeState::Modified => (rgb(0xD97706), rgb(0xFFFBEB)),
            ChangeState::Deleted => (rgb(0xDC2626), rgb(0xFEF2F2)),
            ChangeState::Unchanged => (rgb(0xE5E7EB), rgb(0xFFFFFF)),
        };

        let is_deleted = self.info.change_state == ChangeState::Deleted;
        let panel_id = self.info.panel_id;

        let name = SharedString::from(self.info.name.clone());
        let time_room = SharedString::from(format!(
            "{} · {}",
            self.info.time_range_str,
            if self.info.room_names.is_empty() {
                "—".to_string()
            } else {
                self.info.room_names.join(", ")
            }
        ));
        let code = SharedString::from(self.info.code.clone());

        let mut card = div()
            .id(SharedString::from(format!("panel-{}", self.info.code)))
            .flex()
            .flex_row()
            .mb(px(4.0))
            .rounded(px(6.0))
            .bg(bg_color)
            .border_1()
            .border_color(if self.is_selected {
                rgb(0x2563EB)
            } else {
                rgb(0xE5E7EB)
            })
            .cursor_pointer()
            .opacity(if is_deleted { 0.55 } else { 1.0 });

        // Left color border
        card = card.child(
            div()
                .w(px(4.0))
                .flex_shrink_0()
                .rounded_l(px(5.0))
                .bg(border_color),
        );

        // Card content
        let mut content = div()
            .flex()
            .flex_col()
            .flex_grow()
            .gap(px(2.0))
            .px(px(10.0))
            .py(px(8.0));

        // Name row
        let mut name_div = div().flex().flex_row().justify_between().items_baseline();

        let mut name_text = div()
            .text_sm()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(rgb(0x111827))
            .child(name);

        if is_deleted {
            name_text = name_text.line_through();
        }

        name_div = name_div
            .child(name_text)
            .child(div().text_xs().text_color(rgb(0x9CA3AF)).child(code));

        content = content.child(name_div);

        // Time + room row
        content = content.child(div().text_xs().text_color(rgb(0x6B7280)).child(time_room));

        // Change state badge for non-unchanged
        if self.info.change_state != ChangeState::Unchanged {
            let badge_text = match self.info.change_state {
                ChangeState::Added => "Added",
                ChangeState::Modified => "Modified",
                ChangeState::Deleted => "Deleted",
                ChangeState::Unchanged => unreachable!(),
            };
            let badge_color = match self.info.change_state {
                ChangeState::Added => rgb(0x15803D),
                ChangeState::Modified => rgb(0xB45309),
                ChangeState::Deleted => rgb(0xB91C1C),
                ChangeState::Unchanged => unreachable!(),
            };
            content = content.child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(badge_color)
                    .child(badge_text),
            );
        }

        card = card.child(content);

        card.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_this, _ev, _window, cx| {
                cx.emit(PanelCardEvent::Clicked(panel_id));
            }),
        )
    }
}

#[derive(Debug, Clone)]
pub enum PanelCardEvent {
    Clicked(PanelId),
}

impl EventEmitter<PanelCardEvent> for PanelCard {}
