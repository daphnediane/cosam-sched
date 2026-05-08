/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use gpui::prelude::*;
use gpui::{div, px, rgb, Context, EventEmitter, MouseButton, SharedString, Window};
use schedule_core::tables::EventRoomId;

use crate::ui::schedule_data::RoomDisplayInfo;

pub struct Sidebar {
    pub rooms: Vec<RoomDisplayInfo>,
    pub selected_room: Option<EventRoomId>,
    // Stable room IDs extracted from rooms for click handlers
    room_ids: Vec<EventRoomId>,
}

impl Sidebar {
    pub fn new(rooms: Vec<RoomDisplayInfo>) -> Self {
        let room_ids = rooms.iter().map(|r| r.room_id).collect();
        Self {
            rooms,
            selected_room: None,
            room_ids,
        }
    }

    pub fn set_selected(&mut self, room: Option<EventRoomId>) {
        self.selected_room = room;
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let header_color = rgb(0x111827);
        let active_bg = rgb(0xDBEAFE);
        let active_text = rgb(0x1D4ED8);
        let inactive_text = rgb(0x374151);
        let hover_bg = rgb(0xF3F4F6);
        let all_bg = rgb(0xEFF6FF);

        let mut col = div()
            .id("sidebar")
            .flex()
            .flex_col()
            .w(px(200.0))
            .flex_shrink_0()
            .border_r_1()
            .border_color(rgb(0xE5E7EB))
            .bg(rgb(0xFAFAFA))
            .overflow_y_scroll();

        col = col.child(
            div()
                .px(px(12.0))
                .py(px(10.0))
                .text_sm()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(header_color)
                .child("Rooms"),
        );

        // "All rooms" option
        let all_selected = self.selected_room.is_none();
        let mut all_item = div()
            .id("room-all")
            .px(px(12.0))
            .py(px(7.0))
            .text_sm()
            .cursor_pointer()
            .child("All Rooms");

        if all_selected {
            all_item = all_item.bg(all_bg).text_color(active_text);
        } else {
            all_item = all_item.text_color(inactive_text).hover(|s| s.bg(hover_bg));
        }

        all_item = all_item.on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _ev, _window, cx| {
                this.selected_room = None;
                cx.emit(SidebarEvent::RoomSelected(None));
            }),
        );
        col = col.child(all_item);

        for (i, room) in self.rooms.iter().enumerate() {
            let room_id = self.room_ids[i];
            let is_selected = self.selected_room == Some(room_id);
            let name = SharedString::from(room.display_name.clone());

            let mut item = div()
                .id(SharedString::from(format!("room-{i}")))
                .px(px(12.0))
                .py(px(7.0))
                .text_sm()
                .cursor_pointer()
                .child(name);

            if is_selected {
                item = item.bg(active_bg).text_color(active_text);
            } else {
                item = item.text_color(inactive_text).hover(|s| s.bg(hover_bg));
            }

            item = item.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _ev, _window, cx| {
                    this.selected_room = Some(room_id);
                    cx.emit(SidebarEvent::RoomSelected(Some(room_id)));
                }),
            );

            col = col.child(item);
        }

        col
    }
}

#[derive(Debug, Clone)]
pub enum SidebarEvent {
    RoomSelected(Option<EventRoomId>),
}

impl EventEmitter<SidebarEvent> for Sidebar {}
