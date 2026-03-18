/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::PathBuf;

use chrono::NaiveDate;
use gpui::prelude::*;
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, SharedString, Window, actions, div, px, rgb,
};
use gpui_component::resizable::{h_resizable, resizable_panel};

use crate::data::source_info::ChangeState;
use crate::data::xlsx_export;
use crate::data::xlsx_import::XlsxImportOptions;
use crate::data::xlsx_update;
use crate::data::{Event, JsonExportMode, Schedule};
use crate::ui::day_tabs::{DayTabEvent, DayTabs};
use crate::ui::detail_pane::{DetailPane, DetailPaneEvent, DetailPaneMode};
use crate::ui::event_card::{EventCard, EventCardEvent};
use crate::ui::sidebar::{RoomEntry, Sidebar, SidebarEvent};

const MAX_UNDO_STEPS: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq)]
enum FileType {
    Json,
    Xlsx,
}

actions!(
    schedule_editor,
    [
        FileOpen,
        FileSave,
        FileSaveAs,
        FileExportPublicJson,
        EditUndo,
        EditRedo,
        NewEvent,
    ]
);

pub struct ScheduleEditor {
    focus_handle: FocusHandle,
    schedule: Option<Schedule>,
    current_path: Option<PathBuf>,
    current_file_type: Option<FileType>,
    has_unsaved_changes: bool,
    status_message: Option<String>,
    days: Vec<NaiveDate>,
    selected_day_index: usize,
    selected_room: Option<u32>,
    selected_event_id: Option<String>,
    day_tabs: Entity<DayTabs>,
    sidebar: Entity<Sidebar>,
    event_cards: Vec<Entity<EventCard>>,
    detail_pane: Option<Entity<DetailPane>>,
    undo_stack: Vec<Vec<Event>>,
    redo_stack: Vec<Vec<Event>>,
    #[cfg(not(target_os = "macos"))]
    menu_bar: Entity<crate::menu::WindowsMenuBar>,
}

impl ScheduleEditor {
    pub fn new(schedule: Option<Schedule>, path: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
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

        let current_file_type = path.as_ref().and_then(|p| {
            p.extension().and_then(|ext| ext.to_str()).map(|ext| {
                match ext.to_lowercase().as_str() {
                    "json" => FileType::Json,
                    "xlsx" => FileType::Xlsx,
                    _ => FileType::Json, // Default to JSON for unknown extensions
                }
            })
        });

        let mut editor = Self {
            focus_handle: cx.focus_handle(),
            schedule,
            current_path: path,
            current_file_type,
            has_unsaved_changes: false,
            status_message: None,
            days,
            selected_day_index: 0,
            selected_room: None,
            selected_event_id: None,
            day_tabs,
            sidebar,
            event_cards: Vec::new(),
            detail_pane: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            #[cfg(not(target_os = "macos"))]
            menu_bar: cx.new(|cx| crate::menu::WindowsMenuBar::new(cx)),
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
        self.selected_event_id = None;
        self.detail_pane = None;
        self.undo_stack.clear();
        self.redo_stack.clear();

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
        self.current_path = path.clone();

        // Update file type based on the loaded path
        self.current_file_type = path.as_ref().and_then(|p| {
            p.extension().and_then(|ext| ext.to_str()).map(|ext| {
                match ext.to_lowercase().as_str() {
                    "json" => FileType::Json,
                    "xlsx" => FileType::Xlsx,
                    _ => FileType::Json,
                }
            })
        });

        self.has_unsaved_changes = false;
        self.status_message = Some(format!("Loaded {event_count} events, {room_count} rooms"));

        self.update_window_title(cx);
        self.update_menus(cx);
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn do_open(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let default_dir = self
            .current_path
            .as_ref()
            .and_then(|p| p.parent())
            .unwrap_or_else(|| std::path::Path::new("."));

        let Some(path) = rfd::FileDialog::new()
            .set_directory(default_dir)
            .add_filter("Schedule files", &["json", "xlsx"])
            .add_filter("JSON", &["json"])
            .add_filter("Excel Workbook", &["xlsx"])
            .add_filter("All files", &["*"])
            .pick_file()
        else {
            return;
        };

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "xlsx" && ext != "json" {
            self.status_message =
                Some("Unsupported file type. Please select .xlsx or .json".to_string());
            cx.notify();
            return;
        }

        let import_options = XlsxImportOptions {
            ..XlsxImportOptions::default()
        };

        cx.spawn(async move |this, cx| {
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

    fn do_save_as(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
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

        let current_ext = self
            .current_path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("json");

        let suggested_name = self
            .current_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(|stem| format!("{stem}.{current_ext}"))
            .unwrap_or_else(|| "schedule.json".to_string());

        let mut dialog = rfd::FileDialog::new()
            .set_directory(default_dir)
            .set_file_name(&suggested_name);

        if current_ext == "xlsx" {
            dialog = dialog
                .add_filter("Excel Workbook", &["xlsx"])
                .add_filter("JSON", &["json"]);
        } else {
            dialog = dialog
                .add_filter("JSON", &["json"])
                .add_filter("Excel Workbook", &["xlsx"]);
        }
        dialog = dialog.add_filter("All files", &["*"]);

        let Some(path) = dialog.save_file() else {
            return;
        };

        let schedule_clone = schedule.clone();

        cx.spawn(async move |this, cx| {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let (result, file_type) = if ext == "xlsx" {
                (
                    xlsx_export::export_to_xlsx(&schedule_clone, &path),
                    FileType::Xlsx,
                )
            } else {
                (
                    schedule_clone.save_json_with_mode(&path, JsonExportMode::Staff),
                    FileType::Json,
                )
            };

            cx.update(|cx| {
                this.update(cx, |editor, cx| match result {
                    Ok(()) => {
                        editor.current_path = Some(path.clone());
                        editor.current_file_type = Some(file_type);
                        editor.has_unsaved_changes = false;
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

        let selected_id = self.selected_event_id.clone();
        self.event_cards = events
            .iter()
            .map(|event| {
                let is_selected = selected_id.as_deref() == Some(event.id.as_str());
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
                let card = cx.new(|_cx| {
                    EventCard::new(event, room_name, panel_color, panel_type, is_selected)
                });
                cx.subscribe(
                    &card,
                    |this: &mut Self, _entity, event: &EventCardEvent, cx| {
                        let EventCardEvent::Clicked(id) = event;
                        this.open_detail_for_event(id.clone(), cx);
                    },
                )
                .detach();
                card
            })
            .collect();
    }

    fn push_undo_snapshot(&mut self) {
        if let Some(ref schedule) = self.schedule {
            if self.undo_stack.len() >= MAX_UNDO_STEPS {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push(schedule.events.clone());
            self.redo_stack.clear();
        }
    }

    fn do_undo(&mut self, _: &EditUndo, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.undo_stack.pop() else {
            return;
        };
        if let Some(ref mut schedule) = self.schedule {
            self.redo_stack.push(schedule.events.clone());
            schedule.events = snapshot;
            self.has_unsaved_changes = true;
        }
        self.selected_event_id = None;
        self.detail_pane = None;
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn do_redo(&mut self, _: &EditRedo, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.redo_stack.pop() else {
            return;
        };
        if let Some(ref mut schedule) = self.schedule {
            self.undo_stack.push(schedule.events.clone());
            schedule.events = snapshot;
            self.has_unsaved_changes = true;
        }
        self.selected_event_id = None;
        self.detail_pane = None;
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn open_detail_for_event(&mut self, event_id: String, cx: &mut Context<Self>) {
        let Some(ref schedule) = self.schedule else {
            return;
        };
        let Some(event) = schedule.events.iter().find(|e| e.id == event_id).cloned() else {
            return;
        };
        self.selected_event_id = Some(event_id);
        let rooms: Vec<(u32, String)> = schedule
            .sorted_rooms()
            .iter()
            .map(|r| (r.uid, r.long_name.clone()))
            .collect();
        let panel_types: Vec<(String, String)> = schedule
            .panel_types
            .iter()
            .map(|pt| (pt.effective_uid().to_string(), pt.kind.clone()))
            .collect();
        let presenter_names: Vec<String> =
            schedule.presenters.iter().map(|p| p.name.clone()).collect();
        let pane = cx.new(|cx| {
            DetailPane::new(
                DetailPaneMode::Editing(event),
                rooms,
                panel_types,
                presenter_names,
                cx,
            )
        });
        cx.subscribe(
            &pane,
            |this: &mut Self, _entity, event: &DetailPaneEvent, cx| match event {
                DetailPaneEvent::Save(updated) => {
                    this.apply_save(updated.clone(), cx);
                }
                DetailPaneEvent::Delete(id) => {
                    this.apply_delete(id.clone(), cx);
                }
                DetailPaneEvent::Cancel => {
                    this.selected_event_id = None;
                    this.detail_pane = None;
                    this.rebuild_event_cards(cx);
                    cx.notify();
                }
            },
        )
        .detach();
        self.detail_pane = Some(pane);
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn open_new_event(&mut self, _: &NewEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(ref schedule) = self.schedule else {
            return;
        };
        let day = self.days.get(self.selected_day_index).copied();
        let selected_event_end = self.selected_event_id.as_ref().and_then(|id| {
            schedule
                .events
                .iter()
                .find(|e| &e.id == id)
                .map(|e| e.end_time)
        });
        let default_room = self
            .selected_event_id
            .as_ref()
            .and_then(|id| schedule.events.iter().find(|e| &e.id == id))
            .and_then(|e| e.room_id)
            .or(self.selected_room);
        let default_panel_type = schedule
            .panel_types
            .iter()
            .find(|pt| !pt.is_hidden)
            .map(|pt| pt.effective_uid().to_string());
        let rooms: Vec<(u32, String)> = schedule
            .sorted_rooms()
            .iter()
            .map(|r| (r.uid, r.long_name.clone()))
            .collect();
        let panel_types: Vec<(String, String)> = schedule
            .panel_types
            .iter()
            .map(|pt| (pt.effective_uid().to_string(), pt.kind.clone()))
            .collect();
        let presenter_names: Vec<String> =
            schedule.presenters.iter().map(|p| p.name.clone()).collect();
        let pane = cx.new(|cx| {
            DetailPane::new(
                DetailPaneMode::Creating {
                    day,
                    default_start: selected_event_end,
                    default_room,
                    default_panel_type,
                },
                rooms,
                panel_types,
                presenter_names,
                cx,
            )
        });
        cx.subscribe(
            &pane,
            |this: &mut Self, _entity, event: &DetailPaneEvent, cx| match event {
                DetailPaneEvent::Save(new_event) => {
                    this.apply_save(new_event.clone(), cx);
                }
                DetailPaneEvent::Delete(_) => {}
                DetailPaneEvent::Cancel => {
                    this.selected_event_id = None;
                    this.detail_pane = None;
                    this.rebuild_event_cards(cx);
                    cx.notify();
                }
            },
        )
        .detach();
        self.selected_event_id = None;
        self.detail_pane = Some(pane);
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn apply_save(&mut self, mut updated: Event, cx: &mut Context<Self>) {
        if self.schedule.is_none() {
            return;
        }
        self.push_undo_snapshot();
        let Some(ref mut schedule) = self.schedule else {
            return;
        };
        if let Some(pos) = schedule.events.iter().position(|e| e.id == updated.id) {
            if updated.change_state == ChangeState::Unchanged {
                updated.change_state = ChangeState::Modified;
            }
            schedule.events[pos] = updated.clone();
        } else {
            let was_deleted = schedule
                .events
                .iter()
                .any(|e| e.id == updated.id && e.change_state == ChangeState::Deleted);
            updated.change_state = if was_deleted {
                ChangeState::Replaced
            } else {
                ChangeState::Added
            };
            schedule.events.push(updated.clone());
        }
        self.has_unsaved_changes = true;
        self.selected_event_id = Some(updated.id.clone());
        self.detail_pane = None;
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn apply_delete(&mut self, event_id: String, cx: &mut Context<Self>) {
        if self.schedule.is_none() {
            return;
        }
        self.push_undo_snapshot();
        let Some(ref mut schedule) = self.schedule else {
            return;
        };
        if let Some(event) = schedule.events.iter_mut().find(|e| e.id == event_id) {
            event.change_state = ChangeState::Deleted;
        }
        self.has_unsaved_changes = true;
        self.selected_event_id = None;
        self.detail_pane = None;
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn update_window_title(&self, _cx: &mut Context<Self>) {
        // Window title is updated in render via Window::set_window_title.
    }

    fn update_menus(&self, _cx: &mut Context<Self>) {
        // Menu state updates would go here when we implement dynamic menu updates
    }

    fn can_save(&self) -> bool {
        self.schedule.is_some()
            && self.current_path.is_some()
            && matches!(
                self.current_file_type,
                Some(FileType::Json) | Some(FileType::Xlsx)
            )
    }

    fn can_export(&self) -> bool {
        self.schedule.is_some()
    }

    fn window_title(&self) -> String {
        let app_title = "Cosplay America Schedule Editor";
        let file_name = self
            .current_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str());

        match file_name {
            Some(name) if self.has_unsaved_changes => {
                format!("{app_title} — {name} (modified)")
            }
            Some(name) => format!("{app_title} — {name}"),
            None => app_title.to_string(),
        }
    }

    fn edit_undo(&mut self, action: &EditUndo, window: &mut Window, cx: &mut Context<Self>) {
        self.do_undo(action, window, cx);
    }

    fn edit_redo(&mut self, action: &EditRedo, window: &mut Window, cx: &mut Context<Self>) {
        self.do_redo(action, window, cx);
    }

    // Action handlers
    fn file_open(&mut self, _: &FileOpen, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_open(_window, cx);
    }

    fn file_save(&mut self, _: &FileSave, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_save() {
            self.status_message = Some("Cannot save: No file loaded".to_string());
            cx.notify();
            return;
        }

        let Some(ref schedule) = self.schedule else {
            self.status_message = Some("No schedule to save".to_string());
            cx.notify();
            return;
        };

        let Some(ref path) = self.current_path else {
            self.status_message = Some("No file path available".to_string());
            cx.notify();
            return;
        };

        let file_type = self.current_file_type;
        let mut schedule_clone = schedule.clone();
        let path_clone = path.clone();

        cx.spawn(async move |this, cx| {
            let result = if file_type == Some(FileType::Xlsx) {
                let update_result = xlsx_update::update_xlsx(&schedule_clone, &path_clone);
                if update_result.is_ok() {
                    xlsx_update::post_save_cleanup(&mut schedule_clone);
                }
                update_result
            } else {
                schedule_clone.save_json_with_mode(&path_clone, JsonExportMode::Staff)
            };

            cx.update(|cx| {
                this.update(cx, |editor, cx| match result {
                    Ok(()) => {
                        if file_type == Some(FileType::Xlsx) {
                            if let Some(ref mut sched) = editor.schedule {
                                xlsx_update::post_save_cleanup(sched);
                            }
                        }
                        editor.has_unsaved_changes = false;
                        editor.status_message = Some(format!("Saved: {}", path_clone.display()));
                        editor.update_window_title(cx);
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

    fn file_save_as(&mut self, _: &FileSaveAs, _window: &mut Window, cx: &mut Context<Self>) {
        self.do_save_as(_window, cx);
    }

    fn file_export_public_json(
        &mut self,
        _: &FileExportPublicJson,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ref schedule) = self.schedule else {
            self.status_message = Some("No schedule to export".to_string());
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
            .map(|stem| format!("{}-public.json", stem))
            .unwrap_or_else(|| "schedule-public.json".to_string());

        let Some(path) = rfd::FileDialog::new()
            .set_directory(default_dir)
            .set_file_name(&suggested_name)
            .add_filter("JSON", &["json"])
            .add_filter("All files", &["*"])
            .save_file()
        else {
            return;
        };

        let schedule_clone = schedule.clone();

        cx.spawn(async move |this, cx| {
            let result = schedule_clone.save_json_with_mode(&path, JsonExportMode::Public);

            cx.update(|cx| {
                this.update(cx, |editor, cx| match result {
                    Ok(()) => {
                        editor.status_message =
                            Some(format!("Exported public schedule: {}", path.display()));
                        cx.notify();
                    }
                    Err(e) => {
                        editor.status_message = Some(format!("Export error: {e}"));
                        cx.notify();
                    }
                })
            })
            .ok();
        })
        .detach();
    }
}

impl Render for ScheduleEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title(&self.window_title());
        window.set_window_edited(self.has_unsaved_changes);

        let bg = rgb(0xF9FAFB);
        let title_color = rgb(0x111827);
        let subtitle_color = rgb(0x6B7280);
        let empty_color = rgb(0x9CA3AF);
        let status_color = rgb(0x059669);
        let has_schedule = self.schedule.is_some();

        let title = self
            .schedule
            .as_ref()
            .map(|s| s.meta.title.clone())
            .unwrap_or_else(|| "No schedule loaded".to_string());
        let title = SharedString::from(title);

        let event_count = self.schedule.as_ref().map(|s| s.events.len()).unwrap_or(0);
        let event_count_text = SharedString::from(format!("{event_count} events"));

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
            .relative()
            .flex()
            .flex_col()
            .track_focus(&self.focus_handle)
            .bg(rgb(0xFFFFFF))
            .on_action(cx.listener(Self::file_open))
            .on_action(cx.listener(Self::edit_undo))
            .on_action(cx.listener(Self::edit_redo));

        // Platform menu bar (Windows/Linux only; macOS uses native menus)
        #[cfg(not(target_os = "macos"))]
        {
            layout = layout.child(self.menu_bar.clone());
        }

        // Plus button (new event)
        let plus_btn = div()
            .id("new-event-btn")
            .px(px(12.0))
            .py(px(6.0))
            .bg(rgb(0x2563EB))
            .hover(|s| s.bg(rgb(0x1D4ED8)))
            .rounded(px(6.0))
            .text_sm()
            .text_color(rgb(0xFFFFFF))
            .font_weight(gpui::FontWeight::BOLD)
            .cursor_pointer()
            .child("+  New Event")
            .when(has_schedule, |this| {
                this.on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _ev, window, cx| {
                        this.open_new_event(&NewEvent, window, cx);
                    }),
                )
            });

        layout = layout
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
                    .child(plus_btn),
            );

        layout = layout.when(self.can_save(), |this| {
            this.on_action(cx.listener(Self::file_save))
        });
        layout = layout.when(self.can_export(), |this| {
            this.on_action(cx.listener(Self::file_save_as))
                .on_action(cx.listener(Self::file_export_public_json))
        });

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

        // Body: sidebar + content + optional detail pane
        let body = div()
            .flex()
            .flex_grow()
            .overflow_hidden()
            .child(self.sidebar.clone())
            .child(if let Some(ref pane) = self.detail_pane {
                h_resizable("editor-content-detail")
                    .child(
                        resizable_panel()
                            .size_range(px(200.0)..gpui::Pixels::MAX)
                            .child(content),
                    )
                    .child(
                        resizable_panel()
                            .size(px(380.0))
                            .size_range(px(250.0)..px(700.0))
                            .child(
                                div()
                                    .id("detail-pane-scroll")
                                    .h_full()
                                    .border_l_1()
                                    .border_color(rgb(0xE5E7EB))
                                    .overflow_y_scroll()
                                    .child(pane.clone()),
                            ),
                    )
                    .into_any_element()
            } else {
                content.into_any_element()
            });

        layout = layout.child(body);
        layout
    }
}

impl Focusable for ScheduleEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
