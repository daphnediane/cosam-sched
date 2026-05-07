/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{actions, div, px, rgb, Context, FocusHandle, Focusable, SharedString, Window};
use schedule_core::edit::context::EditContext;
use schedule_core::tables::PanelEntityType;

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
    ctx: Option<EditContext>,
    current_path: Option<PathBuf>,
    status_message: Option<String>,
    #[cfg(not(target_os = "macos"))]
    menu_bar: gpui::Entity<crate::menu::WindowsMenuBar>,
}

impl ScheduleEditor {
    pub fn new(ctx: Option<EditContext>, path: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        #[cfg(not(target_os = "macos"))]
        let menu_bar = cx.new(|cx| crate::menu::WindowsMenuBar::new(cx));

        Self {
            focus_handle: cx.focus_handle(),
            ctx,
            current_path: path,
            status_message: None,
            #[cfg(not(target_os = "macos"))]
            menu_bar,
        }
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

    fn handle_file_save(&mut self, _: &FileSave, _window: &mut Window, _cx: &mut Context<Self>) {
        eprintln!("FileSave: not yet implemented (deferred to EDITOR-033)");
    }

    fn handle_file_save_as(
        &mut self,
        _: &FileSaveAs,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        eprintln!("FileSaveAs: not yet implemented (deferred to EDITOR-033)");
    }

    fn handle_file_export_public_json(
        &mut self,
        _: &FileExportPublicJson,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        eprintln!("FileExportPublicJson: not yet implemented (deferred to EDITOR-033)");
    }

    fn handle_file_export_embed(
        &mut self,
        _: &FileExportEmbed,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        eprintln!("FileExportEmbed: not yet implemented (deferred to EDITOR-033)");
    }

    fn handle_file_export_test(
        &mut self,
        _: &FileExportTest,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        eprintln!("FileExportTest: not yet implemented (deferred to EDITOR-033)");
    }

    fn handle_edit_undo(&mut self, _: &EditUndo, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ref mut ctx) = self.ctx {
            match ctx.undo() {
                Ok(()) => {
                    self.status_message = Some("Undo".to_string());
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
        eprintln!("NewEvent: not yet implemented (deferred to EDITOR-033)");
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

        // Body
        let body = if let Some(ref ctx) = self.ctx {
            let schedule = ctx.schedule();
            let schedule_id = format!("{}", schedule.metadata.schedule_id);
            let panel_count = schedule.entity_count::<PanelEntityType>();

            div()
                .flex()
                .flex_col()
                .flex_grow()
                .justify_center()
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .text_base()
                        .text_color(text_primary)
                        .child("Schedule loaded"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(text_muted)
                        .child(SharedString::from(format!("ID: {schedule_id}"))),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(text_muted)
                        .child(SharedString::from(format!("Panels: {panel_count}"))),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x9CA3AF))
                        .child("(Grid view coming in EDITOR-033)"),
                )
        } else {
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
                )
        };

        root.child(body)
    }
}
