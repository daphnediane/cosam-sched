/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::prelude::*;
use gpui::{Context, EventEmitter, MouseButton, SharedString, Window, div, px, rgb};

use crate::data::Event;
use crate::data::SessionDisplayInfo;
use crate::data::source_info::ChangeState;

fn parse_hex_color(hex: &str) -> u32 {
    let hex = hex.trim_start_matches('#');
    u32::from_str_radix(hex, 16).unwrap_or(0x808080)
}

#[derive(Debug, Clone)]
pub enum EventCardEvent {
    Clicked(String),
}

impl EventEmitter<EventCardEvent> for EventCard {}

pub struct EventCard {
    pub event_id: String,
    pub name: SharedString,
    pub time_range: SharedString,
    pub room_name: SharedString,
    pub kind: SharedString,
    pub presenters: SharedString,
    pub color: u32,
    pub is_workshop: bool,
    pub is_selected: bool,
    pub change_state: ChangeState,
}

impl EventCard {
    pub fn new(
        event: &Event,
        room_name: &str,
        panel_type_color: Option<&str>,
        panel_type: Option<&crate::data::panel_type::PanelType>,
        is_selected: bool,
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
            event_id: event.id.clone(),
            name: SharedString::from(event.name.clone()),
            time_range: SharedString::from(time_range),
            room_name: SharedString::from(room_name.to_string()),
            kind: SharedString::from(kind),
            presenters: SharedString::from(presenters),
            color,
            is_workshop: panel_type.map(|pt| pt.is_workshop).unwrap_or(false),
            is_selected,
            change_state: event.change_state,
        }
    }

    pub fn from_session(
        session: &SessionDisplayInfo,
        room_name: &str,
        panel_type_color: Option<&str>,
        panel_type: Option<&crate::data::panel_type::PanelType>,
        is_selected: bool,
    ) -> Self {
        let time_range = format!(
            "{} – {}",
            session.start_time.format("%l:%M %p").to_string().trim(),
            session.end_time.format("%l:%M %p").to_string().trim(),
        );
        let presenters = if session.presenters.is_empty() {
            String::new()
        } else {
            session.presenters.join(", ")
        };
        let color = panel_type_color.map(parse_hex_color).unwrap_or(0xCCCCCC);
        let kind = panel_type
            .map(|pt| pt.kind.clone())
            .unwrap_or_else(|| "Panel".to_string());

        Self {
            event_id: session.session_id.clone(),
            name: SharedString::from(session.name.clone()),
            time_range: SharedString::from(time_range),
            room_name: SharedString::from(room_name.to_string()),
            kind: SharedString::from(kind),
            presenters: SharedString::from(presenters),
            color,
            is_workshop: panel_type.map(|pt| pt.is_workshop).unwrap_or(false),
            is_selected,
            change_state: session.change_state,
        }
    }
}

impl Render for EventCard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border_color = match self.change_state {
            ChangeState::Added => rgb(0x16A34A),
            ChangeState::Replaced => rgb(0xD97706),
            ChangeState::Modified => rgb(0xD97706),
            ChangeState::Deleted => rgb(0xDC2626),
            ChangeState::Converted => rgb(self.color),
            ChangeState::Unchanged => rgb(self.color),
        };

        let bg = match self.change_state {
            ChangeState::Added => rgb(0xF0FDF4),
            ChangeState::Replaced | ChangeState::Modified => rgb(0xFFFBEB),
            ChangeState::Deleted => rgb(0xFFF1F2),
            _ => rgb(0xFFFFFF),
        };

        let opacity = if self.change_state == ChangeState::Deleted {
            0.55
        } else {
            1.0
        };

        let text_secondary = rgb(0x666666);

        let event_id = self.event_id.clone();
        let mut card = div()
            .id(SharedString::from(format!("event-card-{}", self.event_id)))
            .flex()
            .flex_col()
            .p(px(12.0))
            .mb(px(8.0))
            .bg(bg)
            .text_color(rgb(0x111827))
            .border_l(px(4.0))
            .border_color(border_color)
            .rounded_r(px(6.0))
            .shadow_sm()
            .cursor_pointer()
            .opacity(opacity)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_this, _ev, _window, cx| {
                    cx.emit(EventCardEvent::Clicked(event_id.clone()));
                }),
            );

        if self.is_selected {
            card = card.border_1().border_color(rgb(0x2563EB)).rounded(px(6.0));
        }

        // Title row
        let mut title_div = div()
            .text_sm()
            .font_weight(gpui::FontWeight::BOLD)
            .child(self.name.clone());

        if self.change_state == ChangeState::Deleted {
            title_div = title_div.line_through();
        }

        card = card.child(
            div().flex().justify_between().child(title_div).child(
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

        // Change state badge
        let badge_text = match self.change_state {
            ChangeState::Added => Some("Added"),
            ChangeState::Replaced => Some("Replaced"),
            ChangeState::Modified => Some("Modified"),
            ChangeState::Deleted => Some("Deleted"),
            _ => None,
        };
        if let Some(badge) = badge_text {
            let badge_bg = match self.change_state {
                ChangeState::Added => rgb(0xDCFCE7),
                ChangeState::Replaced | ChangeState::Modified => rgb(0xFEF3C7),
                ChangeState::Deleted => rgb(0xFEE2E2),
                _ => rgb(0xF3F4F6),
            };
            let badge_text_color = match self.change_state {
                ChangeState::Added => rgb(0x15803D),
                ChangeState::Replaced | ChangeState::Modified => rgb(0xB45309),
                ChangeState::Deleted => rgb(0xB91C1C),
                _ => rgb(0x6B7280),
            };
            card = card.child(
                div()
                    .mt(px(4.0))
                    .px(px(6.0))
                    .py(px(2.0))
                    .bg(badge_bg)
                    .rounded(px(4.0))
                    .text_xs()
                    .text_color(badge_text_color)
                    .child(badge),
            );
        }

        card
    }
}
