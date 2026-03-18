/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::prelude::*;
use gpui::{Context, SharedString, Window, div, px, rgb};

use crate::data::Event;

fn parse_hex_color(hex: &str) -> u32 {
    let hex = hex.trim_start_matches('#');
    u32::from_str_radix(hex, 16).unwrap_or(0x808080)
}

pub struct EventCard {
    pub name: SharedString,
    pub time_range: SharedString,
    pub room_name: SharedString,
    pub kind: SharedString,
    pub presenters: SharedString,
    pub color: u32,
    pub is_workshop: bool,
}

impl EventCard {
    pub fn new(
        event: &Event,
        room_name: &str,
        panel_type_color: Option<&str>,
        panel_type: Option<&crate::data::panel_type::PanelType>,
    ) -> Self {
        let time_range = format!(
            "{} – {}",
            event.start_time.format("%l:%M %p").to_string().trim(),
            event.end_time.format("%l:%M %p").to_string().trim(),
        );
        let presenters = if event.credits.is_empty() {
            if event.presenters.is_empty() {
                String::new()
            } else {
                event.presenters.join(", ")
            }
        } else {
            event.credits.join(", ")
        };
        let color = panel_type_color.map(parse_hex_color).unwrap_or(0xCCCCCC);

        let kind = panel_type
            .map(|pt| pt.kind.clone())
            .unwrap_or_else(|| "Event".to_string());

        Self {
            name: SharedString::from(event.name.clone()),
            time_range: SharedString::from(time_range),
            room_name: SharedString::from(room_name.to_string()),
            kind: SharedString::from(kind),
            presenters: SharedString::from(presenters),
            color,
            is_workshop: panel_type.map(|pt| pt.is_workshop).unwrap_or(false),
        }
    }
}

impl Render for EventCard {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let border_color = rgb(self.color);
        let bg = rgb(0xFFFFFF);
        let text_secondary = rgb(0x666666);

        let mut card = div()
            .flex()
            .flex_col()
            .p(px(12.0))
            .mb(px(8.0))
            .bg(bg)
            .border_l(px(4.0))
            .border_color(border_color)
            .rounded_r(px(6.0))
            .shadow_sm();

        // Title row
        card = card.child(
            div()
                .flex()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(self.name.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(text_secondary)
                        .child(self.kind.clone()),
                ),
        );

        // Time and room
        card = card.child(
            div()
                .flex()
                .gap(px(12.0))
                .mt(px(4.0))
                .text_xs()
                .text_color(text_secondary)
                .child(self.time_range.clone())
                .child(self.room_name.clone()),
        );

        // Presenters
        if !self.presenters.is_empty() {
            card = card.child(
                div()
                    .mt(px(4.0))
                    .text_xs()
                    .text_color(rgb(0x444444))
                    .child(self.presenters.clone()),
            );
        }

        // Workshop badge
        if self.is_workshop {
            card = card.child(
                div()
                    .mt(px(4.0))
                    .px(px(6.0))
                    .py(px(2.0))
                    .bg(rgb(0xFDEEB5))
                    .rounded(px(4.0))
                    .text_xs()
                    .child("Workshop"),
            );
        }

        card
    }
}
