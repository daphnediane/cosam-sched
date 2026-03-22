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
use indexmap::IndexMap;

use crate::data::source_info::ChangeState;
use crate::data::xlsx_export;
use crate::data::xlsx_import::XlsxImportOptions;
use crate::data::xlsx_update;
use crate::data::{Panel, Schedule};
use crate::ui::day_tabs::{DayTabEvent, DayTabs};
use crate::ui::detail_pane::{DetailPane, DetailPaneEvent};
use crate::ui::event_card::{EventCard, EventCardEvent};
use crate::ui::panel_edit_window::{PanelEditWindow, PanelEditWindowEvent};
use crate::ui::sidebar::{RoomEntry, Sidebar, SidebarEvent};
use crate::ui::web_preview;

const MAX_UNDO_STEPS: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq)]
enum FileType {
    Json,
    Xlsx,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    ListView,
    WebPreview,
}

actions!(
    schedule_editor,
    [
        FileOpen,
        FileSave,
        FileSaveAs,
        FileExportPublicJson,
        FileExportEmbed,
        FileExportTest,
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
    undo_stack: Vec<IndexMap<String, Panel>>,
    redo_stack: Vec<IndexMap<String, Panel>>,
    active_view: ViewMode,
    preview_open_in_browser: bool,
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
                    _ => FileType::Json,
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
            active_view: ViewMode::ListView,
            preview_open_in_browser: false,
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

        let panel_count = schedule.panels.len();
        let room_count = schedule.rooms.len();
        self.schedule = Some(schedule);
        self.current_path = path.clone();

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
        self.status_message = Some(format!("Loaded {panel_count} panels, {room_count} rooms"));

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

        let mut schedule_clone = schedule.clone();

        // Update Excel metadata when saving
        let current_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| "Unknown User".to_string());

        schedule_clone.meta.last_modified_by = Some(username.clone());
        schedule_clone.meta.modified = Some(current_time);

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
                (schedule_clone.save_json(&path), FileType::Json)
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

        let mut sessions = schedule.sessions_for_day(day);

        if let Some(room_uid) = self.selected_room {
            sessions.retain(|s| s.room_ids.contains(&room_uid));
        }

        let selected_id = self.selected_event_id.clone();
        self.event_cards = sessions
            .iter()
            .map(|session| {
                let is_selected = selected_id.as_deref() == Some(session.session_id.as_str());
                let room_name = session
                    .room_ids
                    .first()
                    .and_then(|rid| schedule.room_by_id(*rid))
                    .map(|r| r.long_name.as_str())
                    .unwrap_or("—");
                let panel_type = session
                    .panel_type
                    .as_ref()
                    .and_then(|pt_uid| schedule.panel_types.get(pt_uid));
                let panel_color = panel_type.and_then(|pt| pt.color());
                let card = cx.new(|_cx| {
                    EventCard::from_session(
                        session,
                        room_name,
                        panel_color,
                        panel_type,
                        is_selected,
                    )
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
            self.undo_stack.push(schedule.panels.clone());
            self.redo_stack.clear();
        }
    }

    fn do_undo(&mut self, _: &EditUndo, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.undo_stack.pop() else {
            return;
        };
        if let Some(ref mut schedule) = self.schedule {
            self.redo_stack.push(schedule.panels.clone());
            schedule.panels = snapshot;
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
            self.undo_stack.push(schedule.panels.clone());
            schedule.panels = snapshot;
            self.has_unsaved_changes = true;
        }
        self.selected_event_id = None;
        self.detail_pane = None;
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn open_detail_for_event(&mut self, session_id: String, cx: &mut Context<Self>) {
        let Some(ref schedule) = self.schedule else {
            return;
        };

        let mut found_base_id: Option<String> = None;
        'outer: for (base_id, panel) in schedule.panels.iter() {
            for part in &panel.parts {
                for session in &part.sessions {
                    if session.id == session_id {
                        found_base_id = Some(base_id.clone());
                        break 'outer;
                    }
                }
            }
        }

        let Some(base_id) = found_base_id else {
            return;
        };
        let Some(panel) = schedule.panels.get(&base_id).cloned() else {
            return;
        };

        self.selected_event_id = Some(session_id.clone());

        let rooms: Vec<(u32, String)> = schedule
            .sorted_rooms()
            .iter()
            .map(|r| (r.uid, r.long_name.clone()))
            .collect();
        let panel_types: Vec<(String, String)> = schedule
            .panel_types
            .iter()
            .map(|(prefix, pt)| (prefix.clone(), pt.kind.clone()))
            .collect();

        let pane = cx.new(|_cx| DetailPane::new(&panel, &rooms, &panel_types, &session_id));
        cx.subscribe(
            &pane,
            |this: &mut Self, _entity, event: &DetailPaneEvent, cx| match event {
                DetailPaneEvent::Close => {
                    this.selected_event_id = None;
                    this.detail_pane = None;
                    this.rebuild_event_cards(cx);
                    cx.notify();
                }
                DetailPaneEvent::OpenEdit {
                    base_id,
                    session_id,
                } => {
                    this.open_edit_window(base_id.clone(), session_id.clone(), cx);
                }
            },
        )
        .detach();
        self.detail_pane = Some(pane);
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn open_edit_window(&mut self, base_id: String, session_id: String, cx: &mut Context<Self>) {
        let Some(ref schedule) = self.schedule else {
            return;
        };
        let Some(panel) = schedule.panels.get(&base_id).cloned() else {
            return;
        };
        let rooms: Vec<(u32, String)> = schedule
            .sorted_rooms()
            .iter()
            .map(|r| (r.uid, r.long_name.clone()))
            .collect();
        let panel_types: Vec<(String, String)> = schedule
            .panel_types
            .iter()
            .map(|(prefix, pt)| (prefix.clone(), pt.kind.clone()))
            .collect();
        let presenter_names: Vec<String> =
            schedule.presenters.iter().map(|p| p.name.clone()).collect();

        let edit_entity = cx.new(|cx| {
            PanelEditWindow::new(panel, &session_id, rooms, panel_types, presenter_names, cx)
        });
        cx.subscribe(
            &edit_entity,
            |this: &mut Self, _entity, event: &PanelEditWindowEvent, cx| match event {
                PanelEditWindowEvent::Save(panel) => {
                    this.apply_panel_save(panel.clone(), cx);
                }
                PanelEditWindowEvent::SessionDeleted {
                    base_id,
                    session_id,
                } => {
                    this.apply_session_delete(base_id.clone(), session_id.clone(), cx);
                }
            },
        )
        .detach();

        let entity_for_window = edit_entity;
        if cx
            .open_window(gpui::WindowOptions::default(), move |window, cx| {
                window.focus(&entity_for_window.focus_handle(cx));
                cx.new(|cx| gpui_component::Root::new(entity_for_window.clone(), window, cx))
            })
            .is_err()
        {
            self.status_message = Some("Failed to open edit window".to_string());
            cx.notify();
        }
    }

    fn open_new_event(&mut self, _: &NewEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(ref schedule) = self.schedule else {
            self.status_message = Some("No schedule loaded".to_string());
            cx.notify();
            return;
        };

        let new_id = format!("new-{}", chrono::Utc::now().timestamp_millis());
        let panel = Panel::new(new_id);

        let rooms: Vec<(u32, String)> = schedule
            .sorted_rooms()
            .iter()
            .map(|r| (r.uid, r.long_name.clone()))
            .collect();
        let panel_types: Vec<(String, String)> = schedule
            .panel_types
            .iter()
            .map(|(prefix, pt)| (prefix.clone(), pt.kind.clone()))
            .collect();
        let presenter_names: Vec<String> =
            schedule.presenters.iter().map(|p| p.name.clone()).collect();

        let edit_entity =
            cx.new(|cx| PanelEditWindow::new(panel, "", rooms, panel_types, presenter_names, cx));
        cx.subscribe(
            &edit_entity,
            |this: &mut Self, _entity, event: &PanelEditWindowEvent, cx| match event {
                PanelEditWindowEvent::Save(panel) => {
                    this.apply_panel_save(panel.clone(), cx);
                }
                PanelEditWindowEvent::SessionDeleted {
                    base_id,
                    session_id,
                } => {
                    this.apply_session_delete(base_id.clone(), session_id.clone(), cx);
                }
            },
        )
        .detach();

        let entity_for_window = edit_entity;
        if cx
            .open_window(gpui::WindowOptions::default(), move |window, cx| {
                window.focus(&entity_for_window.focus_handle(cx));
                cx.new(|cx| gpui_component::Root::new(entity_for_window.clone(), window, cx))
            })
            .is_err()
        {
            self.status_message = Some("Failed to open new panel window".to_string());
            cx.notify();
        }
    }

    pub fn apply_panel_save(&mut self, mut panel: Panel, cx: &mut Context<Self>) {
        self.push_undo_snapshot();
        if let Some(ref mut schedule) = self.schedule {
            let is_existing = schedule.panels.contains_key(&panel.id);
            if panel.change_state == ChangeState::Unchanged {
                panel.change_state = if is_existing {
                    ChangeState::Modified
                } else {
                    ChangeState::Added
                };
            }
            schedule.panels.insert(panel.id.clone(), panel);
            self.has_unsaved_changes = true;
        }
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    pub fn apply_session_delete(
        &mut self,
        base_id: String,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        self.push_undo_snapshot();
        if let Some(ref mut schedule) = self.schedule {
            if let Some(panel) = schedule.panels.get_mut(&base_id) {
                for part in &mut panel.parts {
                    part.sessions.retain(|s| s.id != session_id);
                }
                panel.parts.retain(|p| !p.sessions.is_empty());
            }
            self.has_unsaved_changes = true;
        }
        if self.selected_event_id.as_deref() == Some(session_id.as_str()) {
            self.selected_event_id = None;
            self.detail_pane = None;
        }
        self.rebuild_event_cards(cx);
        cx.notify();
    }

    fn update_window_title(&self, _cx: &mut Context<Self>) {
        // Window title is updated in render via Window::set_window_title.
    }

    fn update_menus(&self, _cx: &mut Context<Self>) {
        // Menu state updates would go here when we implement dynamic menu updates
    }

    fn switch_to_list_view(&mut self, cx: &mut Context<Self>) {
        self.active_view = ViewMode::ListView;
        self.status_message = Some("Switched to list view".to_string());
        cx.notify();
    }

    fn switch_to_web_preview(&mut self, cx: &mut Context<Self>) {
        self.active_view = ViewMode::WebPreview;
        self.refresh_web_preview(cx);
    }

    fn refresh_web_preview(&mut self, cx: &mut Context<Self>) {
        let Some(ref schedule) = self.schedule else {
            self.status_message = Some("No schedule to preview".to_string());
            cx.notify();
            return;
        };

        match web_preview::write_preview(schedule) {
            Ok(path) => {
                if let Err(err) = web_preview::open_preview_in_browser(&path) {
                    self.status_message = Some(format!("Failed to open browser: {err}"));
                    cx.notify();
                    return;
                }
                self.preview_open_in_browser = true;
                self.status_message = Some("Preview updated".to_string());
            }
            Err(err) => {
                self.status_message = Some(format!("Preview error: {err}"));
            }
        }
        cx.notify();
    }

    fn reopen_preview_in_browser(&mut self, cx: &mut Context<Self>) {
        let path = web_preview::preview_file_path();
        if !path.exists() {
            self.refresh_web_preview(cx);
            return;
        }
        match web_preview::open_preview_in_browser(&path) {
            Ok(()) => {
                self.preview_open_in_browser = true;
                self.status_message = Some("Reopened preview in browser".to_string());
            }
            Err(err) => {
                self.status_message = Some(format!("Failed to open browser: {err}"));
            }
        }
        cx.notify();
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

        // Update Excel metadata when saving
        let current_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .or_else(|_| std::env::var("LOGNAME"))
            .unwrap_or_else(|_| "Unknown User".to_string());

        schedule_clone.meta.last_modified_by = Some(username.clone());
        schedule_clone.meta.modified = Some(current_time);

        cx.spawn(async move |this, cx| {
            let result = if file_type == Some(FileType::Xlsx) {
                let update_result = xlsx_update::update_xlsx(&schedule_clone, &path_clone);
                if update_result.is_ok() {
                    xlsx_update::post_save_cleanup(&mut schedule_clone);
                }
                update_result
            } else {
                schedule_clone.save_json(&path_clone)
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
            let result = schedule_clone.export_public(&path);

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

    fn file_export_embed(
        &mut self,
        _: &FileExportEmbed,
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
            .map(|stem| format!("{}-embed.html", stem))
            .unwrap_or_else(|| "schedule-embed.html".to_string());

        let Some(path) = rfd::FileDialog::new()
            .set_directory(default_dir)
            .set_file_name(&suggested_name)
            .add_filter("HTML", &["html", "htm"])
            .add_filter("All files", &["*"])
            .save_file()
        else {
            return;
        };

        let schedule_clone = schedule.clone();

        cx.spawn(async move |this, cx| {
            // Convert schedule to JSON string
            let json_data = match schedule_clone.export_public_json_string() {
                Ok(json) => json,
                Err(e) => {
                    cx.update(|cx| {
                        this.update(cx, |editor, cx| {
                            editor.status_message =
                                Some(format!("Failed to serialize schedule: {e}"));
                            cx.notify();
                        })
                    })
                    .ok();
                    return;
                }
            };

            // Create widget sources (using built-in defaults)
            let sources = schedule_core::data::WidgetSources::default();

            let result = schedule_core::data::write_embed_html(
                &path, &json_data, &sources, true, // minified
                None, // style_page
            );

            cx.update(|cx| {
                this.update(cx, |editor, cx| match result {
                    Ok(()) => {
                        editor.status_message =
                            Some(format!("Exported embedded widget: {}", path.display()));
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

    fn file_export_test(
        &mut self,
        _: &FileExportTest,
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
            .map(|stem| format!("{}-test.html", stem))
            .unwrap_or_else(|| "schedule-test.html".to_string());

        let Some(path) = rfd::FileDialog::new()
            .set_directory(default_dir)
            .set_file_name(&suggested_name)
            .add_filter("HTML", &["html", "htm"])
            .add_filter("All files", &["*"])
            .save_file()
        else {
            return;
        };

        let schedule_clone = schedule.clone();

        cx.spawn(async move |this, cx| {
            // Convert schedule to JSON string
            let json_data = match schedule_clone.export_public_json_string() {
                Ok(json) => json,
                Err(e) => {
                    cx.update(|cx| {
                        this.update(cx, |editor, cx| {
                            editor.status_message =
                                Some(format!("Failed to serialize schedule: {e}"));
                            cx.notify();
                        })
                    })
                    .ok();
                    return;
                }
            };

            // Create widget sources (using built-in defaults)
            let sources = schedule_core::data::WidgetSources::default();

            // Extract title from filename
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Schedule");

            let result = schedule_core::data::write_test_html(
                &path, &json_data, title, &sources, true, // minified
                None, // style_page
            );

            cx.update(|cx| {
                this.update(cx, |editor, cx| match result {
                    Ok(()) => {
                        editor.status_message =
                            Some(format!("Exported test page: {}", path.display()));
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

        let panel_count = self.schedule.as_ref().map(|s| s.panels.len()).unwrap_or(0);
        let panel_count_text = SharedString::from(format!("{panel_count} panels"));

        let status_bar = self.status_message.as_ref().map(|msg| {
            div()
                .px(px(16.0))
                .py(px(4.0))
                .bg(rgb(0xECFDF5))
                .text_xs()
                .text_color(status_color)
                .child(SharedString::from(msg.clone()))
        });

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
        } else if self.active_view == ViewMode::WebPreview {
            let refresh_btn = div()
                .id("refresh-preview-btn")
                .px(px(16.0))
                .py(px(8.0))
                .bg(rgb(0x6B21A8))
                .hover(|s| s.bg(rgb(0x581C87)))
                .rounded(px(6.0))
                .text_sm()
                .text_color(rgb(0xFFFFFF))
                .font_weight(gpui::FontWeight::BOLD)
                .cursor_pointer()
                .child("Refresh Preview")
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _ev, _window, cx| {
                        this.refresh_web_preview(cx);
                    }),
                );

            let reopen_btn = div()
                .id("reopen-preview-btn")
                .px(px(16.0))
                .py(px(8.0))
                .bg(rgb(0xF3F4F6))
                .hover(|s| s.bg(rgb(0xE5E7EB)))
                .rounded(px(6.0))
                .text_sm()
                .text_color(rgb(0x374151))
                .font_weight(gpui::FontWeight::BOLD)
                .cursor_pointer()
                .child("Reopen in Browser")
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _ev, _window, cx| {
                        this.reopen_preview_in_browser(cx);
                    }),
                );

            content = content.child(
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .items_center()
                    .py(px(80.0))
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_lg()
                            .text_color(title_color)
                            .child("Preview is open in your browser"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(subtitle_color)
                            .child("The browser auto-reloads when data changes (Safari/Firefox)"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(12.0))
                            .child(refresh_btn)
                            .child(reopen_btn),
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
                    .child("No panels for this selection"),
            );
        } else {
            for card in &self.event_cards {
                content = content.child(card.clone());
            }
        }

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

        #[cfg(not(target_os = "macos"))]
        {
            layout = layout.child(self.menu_bar.clone());
        }

        let active_view = self.active_view;

        let view_toggle_btn =
            |id: &'static str, label: &'static str, _mode: ViewMode, is_active: bool| {
                let mut btn = div()
                    .id(id)
                    .px(px(10.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .text_xs()
                    .cursor_pointer();
                if is_active {
                    btn = btn.bg(rgb(0x6B21A8)).text_color(rgb(0xFFFFFF));
                } else {
                    btn = btn
                        .bg(rgb(0xF3F4F6))
                        .text_color(rgb(0x374151))
                        .hover(|s| s.bg(rgb(0xE5E7EB)));
                }
                btn.child(label)
            };

        let list_btn = view_toggle_btn(
            "view-list-btn",
            "List View",
            ViewMode::ListView,
            active_view == ViewMode::ListView,
        )
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|this, _ev, _window, cx| {
                this.switch_to_list_view(cx);
            }),
        );

        let preview_btn = view_toggle_btn(
            "view-preview-btn",
            "Web Preview",
            ViewMode::WebPreview,
            active_view == ViewMode::WebPreview,
        )
        .when(has_schedule, |this| {
            this.on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _ev, _window, cx| {
                    this.switch_to_web_preview(cx);
                }),
            )
        });

        let view_selector = div()
            .flex()
            .gap(px(4.0))
            .items_center()
            .child(list_btn)
            .child(preview_btn);

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
            .child("+  New Panel")
            .when(has_schedule, |this| {
                this.on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _ev, window, cx| {
                        this.open_new_event(&NewEvent, window, cx);
                    }),
                )
            });

        let header_right = div()
            .flex()
            .gap(px(12.0))
            .items_center()
            .child(view_selector)
            .child(plus_btn);

        layout = layout.child(
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
                                .child(panel_count_text),
                        ),
                )
                .child(header_right),
        );

        layout = layout.when(self.can_save(), |this| {
            this.on_action(cx.listener(Self::file_save))
        });
        layout = layout.when(self.can_export(), |this| {
            this.on_action(cx.listener(Self::file_save_as))
                .on_action(cx.listener(Self::file_export_public_json))
                .on_action(cx.listener(Self::file_export_embed))
                .on_action(cx.listener(Self::file_export_test))
        });

        if let Some(status) = status_bar {
            layout = layout.child(status);
        }

        layout = layout.child(
            div()
                .border_b_1()
                .border_color(rgb(0xE5E7EB))
                .child(self.day_tabs.clone()),
        );

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
                                    .id("detail-pane-wrapper")
                                    .h_full()
                                    .border_l_1()
                                    .border_color(rgb(0xE5E7EB))
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

impl Drop for ScheduleEditor {
    fn drop(&mut self) {
        web_preview::cleanup_preview();
    }
}

impl Focusable for ScheduleEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
