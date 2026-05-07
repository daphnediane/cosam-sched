/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::path::PathBuf;

use dioxus::prelude::*;
use dioxus_desktop::{use_global_shortcut, use_muda_event_handler};
use schedule_core::edit::context::EditContext;
use schedule_core::schedule::{Schedule, FILE_MAGIC};
use schedule_core::tables::PanelEntityType;
use schedule_core::xlsx::{import_xlsx, XlsxImportOptions};

use crate::menu;

fn load_schedule(path: &PathBuf) -> anyhow::Result<Schedule> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(FILE_MAGIC) {
        Schedule::load_from_file(&bytes).map_err(|e| anyhow::anyhow!("{e}"))
    } else {
        import_xlsx(path, &XlsxImportOptions::default()).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

#[component]
pub fn App() -> Element {
    let mut ctx: Signal<Option<EditContext>> = use_signal(|| None);
    let mut current_path: Signal<Option<PathBuf>> = use_signal(|| None);
    let mut status: Signal<Option<String>> = use_signal(|| None);

    // Menu event handler
    use_muda_event_handler(move |event| {
        let id = event.id().0.as_str();
        match id {
            menu::ID_FILE_OPEN => {
                spawn(async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("Schedule files", &["cosam", "xlsx"])
                        .add_filter("cosam schedule", &["cosam"])
                        .add_filter("Excel Workbook", &["xlsx"])
                        .add_filter("All files", &["*"])
                        .pick_file()
                        .await;

                    if let Some(handle) = file {
                        let path = handle.path().to_path_buf();
                        match load_schedule(&path) {
                            Ok(schedule) => {
                                ctx.set(Some(EditContext::new(schedule)));
                                current_path.set(Some(path));
                                status.set(None);
                            }
                            Err(e) => {
                                status.set(Some(format!("Error loading file: {e}")));
                            }
                        }
                    }
                });
            }
            menu::ID_FILE_NEW_SCHEDULE => {
                ctx.set(Some(EditContext::new(Schedule::new())));
                current_path.set(None);
                status.set(None);
            }
            menu::ID_EDIT_UNDO => {
                if let Some(ref mut c) = *ctx.write() {
                    match c.undo() {
                        Ok(()) => status.set(Some("Undo".to_string())),
                        Err(e) => status.set(Some(format!("Nothing to undo: {e}"))),
                    }
                }
            }
            menu::ID_EDIT_REDO => {
                if let Some(ref mut c) = *ctx.write() {
                    match c.redo() {
                        Ok(()) => status.set(Some("Redo".to_string())),
                        Err(e) => status.set(Some(format!("Nothing to redo: {e}"))),
                    }
                }
            }
            menu::ID_FILE_SAVE | menu::ID_FILE_SAVE_AS => {
                eprintln!("{id}: not yet implemented (deferred to EDITOR-033)");
            }
            menu::ID_FILE_EXPORT_JSON | menu::ID_FILE_EXPORT_EMBED | menu::ID_FILE_EXPORT_TEST => {
                eprintln!("{id}: not yet implemented (deferred to EDITOR-033)");
            }
            _ => {}
        }
    });

    // Keyboard shortcuts
    let _ = use_global_shortcut("CmdOrControl+O", move |_| {
        spawn(async move {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("Schedule files", &["cosam", "xlsx"])
                .add_filter("All files", &["*"])
                .pick_file()
                .await;

            if let Some(handle) = file {
                let path = handle.path().to_path_buf();
                match load_schedule(&path) {
                    Ok(schedule) => {
                        ctx.set(Some(EditContext::new(schedule)));
                        current_path.set(Some(path));
                        status.set(None);
                    }
                    Err(e) => {
                        status.set(Some(format!("Error: {e}")));
                    }
                }
            }
        });
    });

    let _ = use_global_shortcut("CmdOrControl+S", move |_| {
        eprintln!("FileSave: not yet implemented (deferred to EDITOR-033)");
    });

    let _ = use_global_shortcut("CmdOrControl+Shift+S", move |_| {
        eprintln!("FileSaveAs: not yet implemented (deferred to EDITOR-033)");
    });

    let _ = use_global_shortcut("CmdOrControl+Z", move |_| {
        if let Some(ref mut c) = *ctx.write() {
            match c.undo() {
                Ok(()) => status.set(Some("Undo".to_string())),
                Err(e) => status.set(Some(format!("Nothing to undo: {e}"))),
            }
        }
    });

    let _ = use_global_shortcut("CmdOrControl+Shift+Z", move |_| {
        if let Some(ref mut c) = *ctx.write() {
            match c.redo() {
                Ok(()) => status.set(Some("Redo".to_string())),
                Err(e) => status.set(Some(format!("Nothing to redo: {e}"))),
            }
        }
    });

    let window_title = current_path
        .read()
        .as_ref()
        .and_then(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "cosam-editor".to_string());

    rsx! {
        style { {include_str!("../style.css")} }
        div { class: "app",
            if let Some(msg) = status.read().as_ref() {
                div { class: "status-bar", "{msg}" }
            }
            div { class: "header",
                span { class: "header-title", "{window_title}" }
            }
            div { class: "body",
                if let Some(ref c) = *ctx.read() {
                    {
                        let schedule = c.schedule();
                        let schedule_id = format!("{}", schedule.metadata.schedule_id);
                        let panel_count = schedule.entity_count::<PanelEntityType>();
                        rsx! {
                            div { class: "placeholder",
                                p { class: "placeholder-title", "Schedule loaded" }
                                p { class: "placeholder-detail", "ID: {schedule_id}" }
                                p { class: "placeholder-detail", "Panels: {panel_count}" }
                                p { class: "placeholder-hint", "(Grid view coming in EDITOR-033)" }
                            }
                        }
                    }
                } else {
                    div { class: "placeholder",
                        p { class: "placeholder-title", "No schedule loaded" }
                        p { class: "placeholder-detail",
                            "Use File > Open to load a .cosam or .xlsx file"
                        }
                    }
                }
            }
        }
    }
}
