use chrono::NaiveDate;
use gpui::prelude::*;
use gpui::{Context, Entity, SharedString, Window, div, px, rgb};

use crate::data::Schedule;
use crate::ui::day_tabs::{DayTabEvent, DayTabs};
use crate::ui::event_card::EventCard;
use crate::ui::sidebar::{RoomEntry, Sidebar, SidebarEvent};

pub struct ScheduleEditor {
    schedule: Schedule,
    days: Vec<NaiveDate>,
    selected_day_index: usize,
    selected_room: Option<u32>,
    day_tabs: Entity<DayTabs>,
    sidebar: Entity<Sidebar>,
    event_cards: Vec<Entity<EventCard>>,
}

impl ScheduleEditor {
    pub fn new(schedule: Schedule, cx: &mut Context<Self>) -> Self {
        let days = schedule.days();
        let selected_day_index = 0;
        let selected_room: Option<u32> = None;

        let day_tabs = cx.new(|_cx| DayTabs::new(days.clone()));
        cx.subscribe(
            &day_tabs,
            |this: &mut Self, _entity, event: &DayTabEvent, cx| match event {
                DayTabEvent::Selected(idx) => {
                    this.selected_day_index = *idx;
                    this.day_tabs
                        .update(cx, |tabs, _cx| tabs.set_selected(*idx));
                    this.rebuild_event_cards(cx);
                    cx.notify();
                }
            },
        )
        .detach();

        let room_entries: Vec<RoomEntry> = schedule
            .sorted_rooms()
            .iter()
            .map(|r| RoomEntry {
                uid: r.uid,
                name: SharedString::from(r.long_name.clone()),
            })
            .collect();

        let sidebar = cx.new(|_cx| Sidebar::new(room_entries));
        cx.subscribe(
            &sidebar,
            |this: &mut Self, _entity, event: &SidebarEvent, cx| match event {
                SidebarEvent::RoomSelected(uid) => {
                    this.selected_room = *uid;
                    this.sidebar.update(cx, |sb, _cx| sb.set_selected(*uid));
                    this.rebuild_event_cards(cx);
                    cx.notify();
                }
            },
        )
        .detach();

        let mut editor = Self {
            schedule,
            days,
            selected_day_index,
            selected_room,
            day_tabs,
            sidebar,
            event_cards: Vec::new(),
        };
        editor.rebuild_event_cards(cx);
        editor
    }

    fn rebuild_event_cards(&mut self, cx: &mut Context<Self>) {
        let Some(day) = self.days.get(self.selected_day_index) else {
            self.event_cards.clear();
            return;
        };

        let mut events = self.schedule.events_for_day(day);

        if let Some(room_uid) = self.selected_room {
            events.retain(|e| e.room_id == Some(room_uid));
        }

        // Sort by start time
        events.sort_by_key(|e| e.start_time);

        self.event_cards = events
            .iter()
            .map(|event| {
                let room_name = event
                    .room_id
                    .and_then(|rid| self.schedule.room_by_id(rid))
                    .map(|r| r.long_name.as_str())
                    .unwrap_or("—");
                cx.new(|_cx| EventCard::new(event, room_name))
            })
            .collect();
    }
}

impl Render for ScheduleEditor {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let bg = rgb(0xF9FAFB);
        let title_color = rgb(0x111827);
        let subtitle_color = rgb(0x6B7280);
        let empty_color = rgb(0x9CA3AF);

        let title = SharedString::from(self.schedule.meta.title.clone());

        let event_count_text = SharedString::from(format!("{} events", self.schedule.events.len()));

        // Build the content area
        let mut content = div()
            .id("content-scroll")
            .flex()
            .flex_col()
            .flex_grow()
            .overflow_y_scroll()
            .p(px(16.0))
            .bg(bg);

        if self.event_cards.is_empty() {
            content = content.child(
                div()
                    .flex()
                    .justify_center()
                    .items_center()
                    .py(px(48.0))
                    .text_color(empty_color)
                    .child("No events for this selection"),
            );
        } else {
            for card in &self.event_cards {
                content = content.child(card.clone());
            }
        }

        // Main layout
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0xFFFFFF))
            // Title bar
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(rgb(0xE5E7EB))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(title_color)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(subtitle_color)
                            .child(event_count_text),
                    ),
            )
            // Day tabs
            .child(
                div()
                    .border_b_1()
                    .border_color(rgb(0xE5E7EB))
                    .child(self.day_tabs.clone()),
            )
            // Body: sidebar + content
            .child(
                div()
                    .flex()
                    .flex_grow()
                    .overflow_hidden()
                    .child(self.sidebar.clone())
                    .child(content),
            )
    }
}
