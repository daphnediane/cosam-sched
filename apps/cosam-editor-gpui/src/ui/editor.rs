/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::PathBuf;

use chrono::NaiveDate;
use gpui::prelude::*;
use gpui::{actions, div, px, rgb, Context, Entity, FocusHandle, Focusable, SharedString, Window};
use schedule_core::edit::context::EditContext;
use schedule_core::tables::{EventRoomId, PanelId};
use schedule_core::value::{FieldValue, FieldValueItem};

use crate::ui::day_tabs::{DayTabEvent, DayTabs};
use crate::ui::detail_pane::{DetailPane, DetailPaneEvent};
use crate::ui::panel_card::{PanelCard, PanelCardEvent};
use crate::ui::schedule_data::{all_days, all_rooms, panels_for, RoomDisplayInfo};
use crate::ui::sidebar::{Sidebar, SidebarEvent};

actions!(
    schedule_editor,
    [
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
    pub ctx: Option<EditContext>,
    pub current_path: Option<PathBuf>,
    status_message: Option<String>,

    // Grid state
    days: Vec<NaiveDate>,
    rooms: Vec<RoomDisplayInfo>,
    selected_day_index: usize,
    selected_room: Option<EventRoomId>,
    selected_panel_id: Option<PanelId>,
    panel_cards: Vec<Entity<PanelCard>>,
    day_tabs: Option<Entity<DayTabs>>,
    sidebar: Option<Entity<Sidebar>>,
    detail_pane: Option<Entity<DetailPane>>,

    #[cfg(not(target_os = "macos"))]
    menu_bar: Entity<crate::menu::WindowsMenuBar>,
}

impl ScheduleEditor {
    pub fn new(ctx: Option<EditContext>, path: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        #[cfg(not(target_os = "macos"))]
        let menu_bar = cx.new(|cx| crate::menu::WindowsMenuBar::new(cx));

        let mut editor = Self {
            focus_handle: cx.focus_handle(),
            ctx,
            current_path: path,
            status_message: None,
            days: Vec::new(),
            rooms: Vec::new(),
            selected_day_index: 0,
            selected_room: None,
            selected_panel_id: None,
            panel_cards: Vec::new(),
            day_tabs: None,
            sidebar: None,
            detail_pane: None,
            #[cfg(not(target_os = "macos"))]
            menu_bar,
        };

        if editor.ctx.is_some() {
            editor.load_schedule_data(cx);
        }

        editor
    }

    pub fn set_schedule(
        &mut self,
        ctx: EditContext,
        path: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        self.ctx = Some(ctx);
        self.current_path = path;
        self.selected_day_index = 0;
        self.selected_room = None;
        self.selected_panel_id = None;
        self.detail_pane = None;
        self.load_schedule_data(cx);
        cx.notify();
    }

    fn window_title(&self) -> String {
        match &self.current_path {
            Some(p) => p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("cosam-editor")
                .to_string(),
            None => "cosam-editor".to_string(),
        }
    }

    fn load_schedule_data(&mut self, cx: &mut Context<Self>) {
        let Some(ref ctx) = self.ctx else { return };
        let schedule = ctx.schedule();

        self.days = all_days(schedule);
        self.rooms = all_rooms(schedule);
        if self.selected_day_index >= self.days.len() {
            self.selected_day_index = 0;
        }

        // Create day tabs entity
        let days = self.days.clone();
        let day_tabs = cx.new(|_cx| DayTabs::new(days));
        cx.subscribe(
            &day_tabs,
            |this, _entity, event: &DayTabEvent, cx| match event {
                DayTabEvent::Selected(idx) => {
                    this.selected_day_index = *idx;
                    this.selected_panel_id = None;
                    this.detail_pane = None;
                    this.rebuild_panel_cards(cx);
                    cx.notify();
                }
            },
        )
        .detach();
        self.day_tabs = Some(day_tabs);

        // Create sidebar entity
        let rooms = self.rooms.clone();
        let sidebar = cx.new(|_cx| Sidebar::new(rooms));
        cx.subscribe(
            &sidebar,
            |this, _entity, event: &SidebarEvent, cx| match event {
                SidebarEvent::RoomSelected(room) => {
                    this.selected_room = *room;
                    this.selected_panel_id = None;
                    this.detail_pane = None;
                    this.rebuild_panel_cards(cx);
                    cx.notify();
                }
            },
        )
        .detach();
        self.sidebar = Some(sidebar);

        self.rebuild_panel_cards(cx);
    }

    fn rebuild_panel_cards(&mut self, cx: &mut Context<Self>) {
        let Some(ref ctx) = self.ctx else {
            self.panel_cards = Vec::new();
            return;
        };
        let Some(day) = self.days.get(self.selected_day_index).copied() else {
            self.panel_cards = Vec::new();
            return;
        };

        let panels = panels_for(ctx.schedule(), day, self.selected_room);
        let selected_id = self.selected_panel_id;

        self.panel_cards = panels
            .into_iter()
            .map(|info| {
                let is_selected = selected_id == Some(info.panel_id);
                let card = cx.new(|_cx| PanelCard::new(info, is_selected));
                cx.subscribe(
                    &card,
                    |this, _entity, event: &PanelCardEvent, cx| match event {
                        PanelCardEvent::Clicked(panel_id) => {
                            this.open_detail_pane(*panel_id, cx);
                        }
                    },
                )
                .detach();
                card
            })
            .collect();
    }

    fn open_detail_pane(&mut self, panel_id: PanelId, cx: &mut Context<Self>) {
        let Some(ref ctx) = self.ctx else { return };
        let Some(day) = self.days.get(self.selected_day_index).copied() else {
            return;
        };

        let panels = panels_for(ctx.schedule(), day, self.selected_room);
        let Some(info) = panels.into_iter().find(|p| p.panel_id == panel_id) else {
            return;
        };

        self.selected_panel_id = Some(panel_id);

        if let Some(ref pane) = self.detail_pane {
            pane.update(cx, |pane, _cx| pane.update_info(info));
        } else {
            let pane = cx.new(|cx| DetailPane::new(info, cx));
            cx.subscribe(
                &pane,
                |this, _entity, event: &DetailPaneEvent, cx| match event {
                    DetailPaneEvent::Close => {
                        this.selected_panel_id = None;
                        this.detail_pane = None;
                        this.rebuild_panel_cards(cx);
                        cx.notify();
                    }
                    DetailPaneEvent::SaveRequested => {
                        this.handle_save_from_pane(cx);
                    }
                },
            )
            .detach();
            self.detail_pane = Some(pane);
        }

        self.rebuild_panel_cards(cx);
        cx.notify();
    }

    fn handle_save_from_pane(&mut self, cx: &mut Context<Self>) {
        let Some(panel_id) = self.selected_panel_id else {
            return;
        };
        let Some(ref pane) = self.detail_pane else {
            return;
        };

        let new_name = pane.read(cx).info.name.clone(); // read current name from info as fallback
                                                        // Get actual edited value from InputState if available
        let new_name = if let Some(ref input) = pane.read(cx).name_input.clone() {
            input.read(cx).value().to_string()
        } else {
            new_name
        };

        let new_name = new_name.trim().to_string();
        if new_name.is_empty() {
            self.status_message = Some("Name cannot be empty".to_string());
            cx.notify();
            return;
        }

        if let Some(ref mut ctx) = self.ctx {
            match ctx.update_field_cmd(
                panel_id,
                "name",
                FieldValue::Single(FieldValueItem::String(new_name.clone())),
            ) {
                Ok(cmd) => match ctx.apply(cmd) {
                    Ok(()) => {
                        self.status_message = Some(format!("Saved: {new_name}"));
                        self.selected_panel_id = None;
                        self.detail_pane = None;
                        self.rebuild_panel_cards(cx);
                        cx.notify();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Save failed: {e}"));
                        cx.notify();
                    }
                },
                Err(e) => {
                    self.status_message = Some(format!("Save failed: {e}"));
                    cx.notify();
                }
            }
        }
    }

    fn handle_file_save(&mut self, _: &FileSave, _window: &mut Window, _cx: &mut Context<Self>) {
        eprintln!("FileSave: not yet implemented (deferred to EDITOR-034)");
    }

    fn handle_file_save_as(
        &mut self,
        _: &FileSaveAs,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        eprintln!("FileSaveAs: not yet implemented (deferred to EDITOR-034)");
    }

    fn handle_file_export_public_json(
        &mut self,
        _: &FileExportPublicJson,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        eprintln!("FileExportPublicJson: not yet implemented (deferred to EDITOR-034)");
    }

    fn handle_file_export_embed(
        &mut self,
        _: &FileExportEmbed,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        eprintln!("FileExportEmbed: not yet implemented (deferred to EDITOR-034)");
    }

    fn handle_file_export_test(
        &mut self,
        _: &FileExportTest,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        eprintln!("FileExportTest: not yet implemented (deferred to EDITOR-034)");
    }

    fn handle_edit_undo(&mut self, _: &EditUndo, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ref mut ctx) = self.ctx {
            match ctx.undo() {
                Ok(()) => {
                    self.status_message = Some("Undo".to_string());
                    self.selected_panel_id = None;
                    self.detail_pane = None;
                    self.rebuild_panel_cards(cx);
                    cx.notify();
                }
                Err(e) => {
                    self.status_message = Some(format!("Nothing to undo: {e}"));
                    cx.notify();
                }
            }
        }
    }

    fn handle_edit_redo(&mut self, _: &EditRedo, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ref mut ctx) = self.ctx {
            match ctx.redo() {
                Ok(()) => {
                    self.status_message = Some("Redo".to_string());
                    self.selected_panel_id = None;
                    self.detail_pane = None;
                    self.rebuild_panel_cards(cx);
                    cx.notify();
                }
                Err(e) => {
                    self.status_message = Some(format!("Nothing to redo: {e}"));
                    cx.notify();
                }
            }
        }
    }

    fn handle_new_event(&mut self, _: &NewEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        eprintln!("NewEvent: not yet implemented (deferred to EDITOR-034)");
    }
}

impl Focusable for ScheduleEditor {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ScheduleEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = rgb(0xF3F4F6);
        let text_primary = rgb(0x111827);
        let text_muted = rgb(0x6B7280);

        let title = SharedString::from(self.window_title());

        let mut root = div()
            .track_focus(&self.focus_handle)
            .key_context("ScheduleEditor")
            .on_action(cx.listener(Self::handle_file_save))
            .on_action(cx.listener(Self::handle_file_save_as))
            .on_action(cx.listener(Self::handle_file_export_public_json))
            .on_action(cx.listener(Self::handle_file_export_embed))
            .on_action(cx.listener(Self::handle_file_export_test))
            .on_action(cx.listener(Self::handle_edit_undo))
            .on_action(cx.listener(Self::handle_edit_redo))
            .on_action(cx.listener(Self::handle_new_event))
            .flex()
            .flex_col()
            .size_full()
            .bg(bg);

        #[cfg(not(target_os = "macos"))]
        {
            root = root.child(self.menu_bar.clone());
        }

        // Status bar
        if let Some(ref msg) = self.status_message {
            root = root.child(
                div()
                    .px(px(12.0))
                    .py(px(4.0))
                    .text_sm()
                    .text_color(text_muted)
                    .border_b_1()
                    .border_color(rgb(0xE5E7EB))
                    .bg(rgb(0xFFFBEB))
                    .child(SharedString::from(msg.clone())),
            );
        }

        // Header
        root = root.child(
            div()
                .flex()
                .items_center()
                .px(px(16.0))
                .py(px(10.0))
                .border_b_1()
                .border_color(rgb(0xE5E7EB))
                .bg(rgb(0xFFFFFF))
                .child(
                    div()
                        .text_base()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(text_primary)
                        .child(title),
                ),
        );

        if self.ctx.is_none() {
            // No file loaded
            return root.child(
                div()
                    .flex()
                    .flex_col()
                    .flex_grow()
                    .justify_center()
                    .items_center()
                    .py(px(80.0))
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_lg()
                            .text_color(text_muted)
                            .child("No schedule loaded"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x9CA3AF))
                            .child("Use File > Open to load a .cosam or .xlsx file"),
                    ),
            );
        }

        // Day tabs
        if let Some(ref day_tabs) = self.day_tabs {
            root = root.child(day_tabs.clone());
        }

        // Content row: sidebar + card list + detail pane
        let mut content_row = div().flex().flex_row().flex_grow().overflow_hidden();

        // Sidebar
        if let Some(ref sidebar) = self.sidebar {
            content_row = content_row.child(sidebar.clone());
        }

        // Panel card list
        let mut card_list = div()
            .id("panel-list")
            .flex()
            .flex_col()
            .flex_grow()
            .overflow_y_scroll()
            .p(px(12.0))
            .gap(px(2.0))
            .bg(bg);

        if self.panel_cards.is_empty() {
            card_list =
                card_list.child(
                    div()
                        .flex()
                        .flex_grow()
                        .justify_center()
                        .items_center()
                        .py(px(60.0))
                        .child(div().text_sm().text_color(text_muted).child(
                            if self.days.is_empty() {
                                "No scheduled panels found"
                            } else {
                                "No panels for this selection"
                            },
                        )),
                );
        } else {
            for card in &self.panel_cards {
                card_list = card_list.child(card.clone());
            }
        }

        content_row = content_row.child(card_list);

        // Detail pane
        if let Some(ref pane) = self.detail_pane {
            content_row = content_row.child(pane.clone());
        }

        root.child(content_row)
    }
}
