use chrono::NaiveDate;
use gpui::prelude::*;
use gpui::{Context, EventEmitter, MouseButton, SharedString, Window, div, px, rgb};

pub struct DayTabs {
    pub days: Vec<NaiveDate>,
    pub selected_index: usize,
}

impl DayTabs {
    pub fn new(days: Vec<NaiveDate>) -> Self {
        Self {
            days,
            selected_index: 0,
        }
    }

    pub fn set_selected(&mut self, index: usize) {
        if index < self.days.len() {
            self.selected_index = index;
        }
    }
}

impl Render for DayTabs {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_bg = rgb(0x2563EB);
        let active_text = rgb(0xFFFFFF);
        let inactive_bg = rgb(0xF3F4F6);
        let inactive_text = rgb(0x374151);
        let hover_bg = rgb(0xDBEAFE);

        let mut row = div().flex().gap(px(4.0)).p(px(8.0));

        for (i, day) in self.days.iter().enumerate() {
            let label = SharedString::from(day.format("%A, %b %d").to_string());
            let is_selected = i == self.selected_index;
            let idx = i;

            let mut tab = div()
                .id(SharedString::from(format!("day-tab-{i}")))
                .px(px(16.0))
                .py(px(8.0))
                .rounded(px(6.0))
                .text_sm()
                .cursor_pointer()
                .child(label);

            if is_selected {
                tab = tab.bg(active_bg).text_color(active_text);
            } else {
                tab = tab
                    .bg(inactive_bg)
                    .text_color(inactive_text)
                    .hover(|style| style.bg(hover_bg));
            }

            tab = tab.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _ev, _window, cx| {
                    this.selected_index = idx;
                    cx.emit(DayTabEvent::Selected(idx));
                }),
            );

            row = row.child(tab);
        }

        row
    }
}

#[derive(Debug, Clone)]
pub enum DayTabEvent {
    Selected(usize),
}

impl EventEmitter<DayTabEvent> for DayTabs {}
