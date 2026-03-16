use gpui::prelude::*;
use gpui::{Context, EventEmitter, SharedString, Window, div, px, rgb};

pub struct RoomEntry {
    pub uid: u32,
    pub name: SharedString,
}

pub struct Sidebar {
    pub rooms: Vec<RoomEntry>,
    pub selected_room: Option<u32>,
}

impl Sidebar {
    pub fn new(rooms: Vec<RoomEntry>) -> Self {
        Self {
            rooms,
            selected_room: None,
        }
    }

    pub fn set_selected(&mut self, uid: Option<u32>) {
        self.selected_room = uid;
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
            .border_r_1()
            .border_color(rgb(0xE5E7EB))
            .bg(rgb(0xFAFAFA))
            .overflow_y_scroll();

        // Header
        col = col.child(
            div()
                .p(px(12.0))
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
            .py(px(6.0))
            .text_sm()
            .cursor_pointer()
            .child("All Rooms");

        if all_selected {
            all_item = all_item.bg(all_bg).text_color(active_text);
        } else {
            all_item = all_item
                .text_color(inactive_text)
                .hover(|style| style.bg(hover_bg));
        }

        all_item = all_item.on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|this, _ev, _window, cx| {
                this.selected_room = None;
                cx.emit(SidebarEvent::RoomSelected(None));
            }),
        );

        col = col.child(all_item);

        // Room list
        for room in &self.rooms {
            let is_selected = self.selected_room == Some(room.uid);
            let uid = room.uid;

            let mut item = div()
                .id(SharedString::from(format!("room-{uid}")))
                .px(px(12.0))
                .py(px(6.0))
                .text_sm()
                .cursor_pointer()
                .child(room.name.clone());

            if is_selected {
                item = item.bg(active_bg).text_color(active_text);
            } else {
                item = item
                    .text_color(inactive_text)
                    .hover(|style| style.bg(hover_bg));
            }

            item = item.on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _ev, _window, cx| {
                    this.selected_room = Some(uid);
                    cx.emit(SidebarEvent::RoomSelected(Some(uid)));
                }),
            );

            col = col.child(item);
        }

        col
    }
}

#[derive(Debug, Clone)]
pub enum SidebarEvent {
    RoomSelected(Option<u32>),
}

impl EventEmitter<SidebarEvent> for Sidebar {}
