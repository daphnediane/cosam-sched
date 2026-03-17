use std::path::PathBuf;

use chrono::NaiveDate;
use gpui::prelude::*;
use gpui::{Context, Entity, PathPromptOptions, SharedString, Window, div, px, rgb};

use crate::data::Schedule;
use crate::data::xlsx_import::XlsxImportOptions;
use crate::ui::day_tabs::{DayTabEvent, DayTabs};
use crate::ui::event_card::EventCard;
use crate::ui::sidebar::{RoomEntry, Sidebar, SidebarEvent};

pub struct ScheduleEditor {
    schedule: Option<Schedule>,
    current_path: Option<PathBuf>,
    staff_mode: bool,
    status_message: Option<String>,
    days: Vec<NaiveDate>,
    selected_day_index: usize,
    selected_room: Option<u32>,
    day_tabs: Entity<DayTabs>,
    sidebar: Entity<Sidebar>,
    event_cards: Vec<Entity<EventCard>>,
}

impl ScheduleEditor {
    pub fn new(
        schedule: Option<Schedule>,
        path: Option<PathBuf>,
        staff_mode: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        let days = schedule.as_ref().map(|s| s.days()).unwrap_or_default();

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

        let room_entries = Self::build_room_entries(schedule.as_ref());

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
            current_path: path,
            staff_mode,
            status_message: None,
            days,
            selected_day_index: 0,
            selected_room: None,
            day_tabs,
            sidebar,
            event_cards: Vec::new(),
        };
        editor.rebuild_event_cards(cx);
        editor
    }

    fn build_room_entries(schedule: Option<&Schedule>) -> Vec<RoomEntry> {
        let Some(schedule) = schedule else {
            return Vec::new();
        };
        schedule
            .sorted_rooms()
            .iter()
            .map(|r| RoomEntry {
                uid: r.uid,
                name: SharedString::from(r.long_name.clone()),
            })
            .collect()
    }

    fn load_schedule(&mut self, schedule: Schedule, path: Option<PathBuf>, cx: &mut Context<Self>) {
        self.days = schedule.days();
        self.selected_day_index = 0;
        self.selected_room = None;

        self.day_tabs.update(cx, |tabs, _cx| {
            tabs.days = self.days.clone();
            tabs.selected_index = 0;
        });

        let room_entries = Self::build_room_entries(Some(&schedule));
        self.sidebar.update(cx, |sb, _cx| {
            sb.rooms = room_entries;
            sb.selected_room = None;
        });

        let event_count = schedule.events.len();
        let room_count = schedule.rooms.len();
        self.schedule = Some(schedule);
        self.current_path = path;
        self.status_message = Some(format!("Loaded {event_count} events, {room_count} rooms"));

        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn do_open(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open Schedule (XLSX or JSON)".into()),
        });

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            if ext != "xlsx" && ext != "json" {
                cx.update(|cx| {
                    this.update(cx, |editor, cx| {
                        editor.status_message =
                            Some("Unsupported file type. Please select .xlsx or .json".to_string());
                        cx.notify();
                    })
                })
                .ok();
                return;
            }

            let staff_mode = cx
                .update(|cx| {
                    this.update(cx, |editor, _cx| editor.staff_mode)
                        .unwrap_or(false)
                })
                .unwrap_or(false);

            let import_options = XlsxImportOptions {
                staff_mode,
                ..XlsxImportOptions::default()
            };

            let result = Schedule::load_auto(&path, &import_options);

            cx.update(|cx| {
                this.update(cx, |editor, cx| match result {
                    Ok(schedule) => {
                        editor.load_schedule(schedule, Some(path), cx);
                    }
                    Err(e) => {
                        editor.status_message = Some(format!("Error: {e}"));
                        cx.notify();
                    }
                })
            })
            .ok();
        })
        .detach();
    }

    fn do_save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(ref schedule) = self.schedule else {
            self.status_message = Some("No schedule to save".to_string());
            cx.notify();
            return;
        };

        let default_dir = self
            .current_path
            .as_ref()
            .and_then(|p| p.parent())
            .unwrap_or_else(|| std::path::Path::new("."));

        let suggested_name = self
            .current_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(|stem| format!("{stem}.json"))
            .unwrap_or_else(|| "schedule.json".to_string());

        let receiver = cx.prompt_for_new_path(default_dir, Some(&suggested_name));

        let mut schedule_clone = schedule.clone();
        let staff_mode = self.staff_mode;

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = receiver.await else {
                return;
            };

            if !staff_mode {
                schedule_clone.events.retain(|_e| true);
            }

            let result = schedule_clone.save_json(&path);

            cx.update(|cx| {
                this.update(cx, |editor, cx| match result {
                    Ok(()) => {
                        editor.current_path = Some(path.clone());
                        editor.status_message = Some(format!("Saved: {}", path.display()));
                        cx.notify();
                    }
                    Err(e) => {
                        editor.status_message = Some(format!("Save error: {e}"));
                        cx.notify();
                    }
                })
            })
            .ok();
        })
        .detach();
    }

    fn rebuild_event_cards(&mut self, cx: &mut Context<Self>) {
        let Some(ref schedule) = self.schedule else {
            self.event_cards.clear();
            return;
        };

        let Some(day) = self.days.get(self.selected_day_index) else {
            self.event_cards.clear();
            return;
        };

        let mut events = schedule.events_for_day(day);

        if let Some(room_uid) = self.selected_room {
            events.retain(|e| e.room_id == Some(room_uid));
        }

        events.sort_by_key(|e| e.start_time);

        self.event_cards = events
            .iter()
            .map(|event| {
                let room_name = event
                    .room_id
                    .and_then(|rid| schedule.room_by_id(rid))
                    .map(|r| r.long_name.as_str())
                    .unwrap_or("—");
                let panel_type = event.panel_type.as_ref().and_then(|pt_uid| {
                    self.schedule.as_ref().and_then(|s| {
                        s.panel_types
                            .iter()
                            .find(|pt| pt.effective_uid() == *pt_uid)
                    })
                });
                let panel_color = panel_type.and_then(|pt| pt.color.as_deref());
                cx.new(|_cx| EventCard::new(event, room_name, panel_color, panel_type))
            })
            .collect();
    }
}

impl Render for ScheduleEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = rgb(0xF9FAFB);
        let title_color = rgb(0x111827);
        let subtitle_color = rgb(0x6B7280);
        let empty_color = rgb(0x9CA3AF);
        let btn_bg = rgb(0xE5E7EB);
        let btn_hover = rgb(0xD1D5DB);
        let btn_active_bg = rgb(0x2563EB);
        let btn_active_text = rgb(0xFFFFFF);
        let status_color = rgb(0x059669);

        let title = self
            .schedule
            .as_ref()
            .map(|s| s.meta.title.clone())
            .unwrap_or_else(|| "No schedule loaded".to_string());
        let title = SharedString::from(title);

        let event_count = self.schedule.as_ref().map(|s| s.events.len()).unwrap_or(0);
        let event_count_text = SharedString::from(format!("{event_count} events"));

        // Build toolbar buttons
        let open_button = div()
            .id("btn-open")
            .px(px(12.0))
            .py(px(6.0))
            .bg(btn_bg)
            .rounded(px(4.0))
            .text_sm()
            .cursor_pointer()
            .hover(|style| style.bg(btn_hover))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _ev, window, cx| {
                    this.do_open(window, cx);
                }),
            )
            .child("Open…");

        let save_button = div()
            .id("btn-save")
            .px(px(12.0))
            .py(px(6.0))
            .bg(btn_bg)
            .rounded(px(4.0))
            .text_sm()
            .cursor_pointer()
            .hover(|style| style.bg(btn_hover))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _ev, window, cx| {
                    this.do_save(window, cx);
                }),
            )
            .child("Save JSON…");

        let staff_label = if self.staff_mode {
            "Staff Mode: ON"
        } else {
            "Staff Mode: OFF"
        };
        let staff_bg = if self.staff_mode {
            btn_active_bg
        } else {
            btn_bg
        };
        let staff_text = if self.staff_mode {
            btn_active_text
        } else {
            title_color
        };

        let staff_button = div()
            .id("btn-staff")
            .px(px(12.0))
            .py(px(6.0))
            .bg(staff_bg)
            .text_color(staff_text)
            .rounded(px(4.0))
            .text_sm()
            .cursor_pointer()
            .hover(|style| style.bg(btn_hover))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _ev, _window, cx| {
                    this.staff_mode = !this.staff_mode;
                    cx.notify();
                }),
            )
            .child(staff_label);

        let toolbar = div()
            .flex()
            .gap(px(8.0))
            .items_center()
            .child(open_button)
            .child(save_button)
            .child(staff_button);

        // Status bar (if there's a message)
        let status_bar = self.status_message.as_ref().map(|msg| {
            div()
                .px(px(16.0))
                .py(px(4.0))
                .bg(rgb(0xECFDF5))
                .text_xs()
                .text_color(status_color)
                .child(SharedString::from(msg.clone()))
        });

        // Build the content area
        let mut content = div()
            .id("content-scroll")
            .flex()
            .flex_col()
            .flex_grow()
            .overflow_y_scroll()
            .p(px(16.0))
            .bg(bg);

        if self.schedule.is_none() {
            content = content.child(
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .items_center()
                    .py(px(80.0))
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_lg()
                            .text_color(empty_color)
                            .child("No schedule loaded"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(subtitle_color)
                            .child("Use Open to load an XLSX spreadsheet or JSON file"),
                    ),
            );
        } else if self.event_cards.is_empty() {
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
        let mut layout = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0xFFFFFF))
            // Title bar with toolbar
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .px(px(16.0))
                    .py(px(8.0))
                    .border_b_1()
                    .border_color(rgb(0xE5E7EB))
                    .child(
                        div()
                            .flex()
                            .gap(px(16.0))
                            .items_center()
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
                    .child(toolbar),
            );

        // Status bar
        if let Some(status) = status_bar {
            layout = layout.child(status);
        }

        // Day tabs
        layout = layout.child(
            div()
                .border_b_1()
                .border_color(rgb(0xE5E7EB))
                .child(self.day_tabs.clone()),
        );

        // Body: sidebar + content
        layout = layout.child(
            div()
                .flex()
                .flex_grow()
                .overflow_hidden()
                .child(self.sidebar.clone())
                .child(content),
        );

        layout
    }
}
