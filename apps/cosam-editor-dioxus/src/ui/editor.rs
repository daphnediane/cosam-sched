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
use schedule_core::tables::{EventRoomId, PanelId};
use schedule_core::value::{FieldValue, FieldValueItem};
use schedule_core::xlsx::{import_xlsx, XlsxImportOptions};
use schedule_core::ChangeState;

use crate::menu;
use crate::ui::schedule_data::{all_days, all_rooms, panels_for, PanelDisplayInfo};

fn load_schedule(path: &PathBuf) -> anyhow::Result<Schedule> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(FILE_MAGIC) {
        Schedule::load_from_file(&bytes).map_err(|e| anyhow::anyhow!("{e}"))
    } else {
        import_xlsx(path, &XlsxImportOptions::default()).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

fn change_state_class(state: &ChangeState) -> &'static str {
    match state {
        ChangeState::Added => "state-added",
        ChangeState::Modified => "state-modified",
        ChangeState::Deleted => "state-deleted",
        ChangeState::Unchanged => "",
    }
}

#[component]
pub fn App() -> Element {
    let mut ctx: Signal<Option<EditContext>> = use_signal(|| None);
    let mut current_path: Signal<Option<PathBuf>> = use_signal(|| None);
    let mut status: Signal<Option<String>> = use_signal(|| None);
    let mut selected_day_index: Signal<usize> = use_signal(|| 0);
    let mut selected_room: Signal<Option<EventRoomId>> = use_signal(|| None);
    let mut selected_panel_id: Signal<Option<PanelId>> = use_signal(|| None);
    let mut editing_name: Signal<String> = use_signal(String::new);

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
                                selected_day_index.set(0);
                                selected_room.set(None);
                                selected_panel_id.set(None);
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
                selected_day_index.set(0);
                selected_room.set(None);
                selected_panel_id.set(None);
            }
            menu::ID_EDIT_UNDO => {
                if let Some(ref mut c) = *ctx.write() {
                    match c.undo() {
                        Ok(()) => {
                            status.set(Some("Undo".to_string()));
                            selected_panel_id.set(None);
                        }
                        Err(e) => status.set(Some(format!("Nothing to undo: {e}"))),
                    }
                }
            }
            menu::ID_EDIT_REDO => {
                if let Some(ref mut c) = *ctx.write() {
                    match c.redo() {
                        Ok(()) => {
                            status.set(Some("Redo".to_string()));
                            selected_panel_id.set(None);
                        }
                        Err(e) => status.set(Some(format!("Nothing to redo: {e}"))),
                    }
                }
            }
            menu::ID_FILE_SAVE | menu::ID_FILE_SAVE_AS => {
                eprintln!("{id}: not yet implemented (deferred to EDITOR-034)");
            }
            menu::ID_FILE_EXPORT_JSON | menu::ID_FILE_EXPORT_EMBED | menu::ID_FILE_EXPORT_TEST => {
                eprintln!("{id}: not yet implemented (deferred to EDITOR-034)");
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
                        selected_day_index.set(0);
                        selected_room.set(None);
                        selected_panel_id.set(None);
                    }
                    Err(e) => status.set(Some(format!("Error: {e}"))),
                }
            }
        });
    });

    let _ = use_global_shortcut("CmdOrControl+S", move |_| {
        eprintln!("FileSave: not yet implemented (deferred to EDITOR-034)");
    });

    let _ = use_global_shortcut("CmdOrControl+Z", move |_| {
        if let Some(ref mut c) = *ctx.write() {
            match c.undo() {
                Ok(()) => {
                    status.set(Some("Undo".to_string()));
                    selected_panel_id.set(None);
                }
                Err(e) => status.set(Some(format!("Nothing to undo: {e}"))),
            }
        }
    });

    let _ = use_global_shortcut("CmdOrControl+Shift+Z", move |_| {
        if let Some(ref mut c) = *ctx.write() {
            match c.redo() {
                Ok(()) => {
                    status.set(Some("Redo".to_string()));
                    selected_panel_id.set(None);
                }
                Err(e) => status.set(Some(format!("Nothing to redo: {e}"))),
            }
        }
    });

    // Derive view data from ctx
    let ctx_read = ctx.read();
    let (days, rooms, panels) = if let Some(ref c) = *ctx_read {
        let sched = c.schedule();
        let days = all_days(sched);
        let rooms = all_rooms(sched);
        let day_idx = *selected_day_index.read();
        let panels = days
            .get(day_idx)
            .copied()
            .map(|d| panels_for(sched, d, *selected_room.read()))
            .unwrap_or_default();
        (days, rooms, panels)
    } else {
        (vec![], vec![], vec![])
    };
    drop(ctx_read);

    let window_title = current_path
        .read()
        .as_ref()
        .and_then(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "cosam-editor".to_string());

    // Detail pane info
    let detail_info: Option<PanelDisplayInfo> = selected_panel_id
        .read()
        .and_then(|pid| panels.iter().find(|p| p.panel_id == pid).cloned());

    rsx! {
        style { {include_str!("../style.css")} }
        div { class: "app",
            // Status bar
            if let Some(msg) = status.read().as_ref() {
                div { class: "status-bar", "{msg}" }
            }
            // Header
            div { class: "header",
                span { class: "header-title", "{window_title}" }
            }

            if ctx.read().is_none() {
                // No schedule loaded
                div { class: "empty-state-full",
                    p { class: "empty-state-title", "No schedule loaded" }
                    p { class: "empty-state-sub",
                        "Use File \u{203A} Open to load a .cosam or .xlsx file"
                    }
                }
            } else {
                // Day tabs
                if !days.is_empty() {
                    div { class: "day-tabs",
                        for (i, day) in days.iter().enumerate() {
                            {
                                let label = day.format("%A, %b %d").to_string();
                                let is_active = i == *selected_day_index.read();
                                rsx! {
                                    button {
                                        class: if is_active { "day-tab active" } else { "day-tab" },
                                        onclick: move |_| {
                                            selected_day_index.set(i);
                                            selected_panel_id.set(None);
                                        },
                                        "{label}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Content row
                div { class: "content-row",
                    // Sidebar
                    if !rooms.is_empty() {
                        div { class: "sidebar",
                            div { class: "sidebar-header", "Rooms" }
                            button {
                                class: if selected_room.read().is_none() { "sidebar-item active" } else { "sidebar-item" },
                                onclick: move |_| {
                                    selected_room.set(None);
                                    selected_panel_id.set(None);
                                },
                                "All Rooms"
                            }
                            for room in &rooms {
                                {
                                    let rid = room.room_id;
                                    let name = room.display_name.clone();
                                    let is_active = *selected_room.read() == Some(rid);
                                    rsx! {
                                        button {
                                            class: if is_active { "sidebar-item active" } else { "sidebar-item" },
                                            onclick: move |_| {
                                                selected_room.set(Some(rid));
                                                selected_panel_id.set(None);
                                            },
                                            "{name}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Panel list
                    div { class: "panel-list",
                        if panels.is_empty() {
                            div { class: "empty-state",
                                if days.is_empty() {
                                    "No scheduled panels found"
                                } else {
                                    "No panels for this selection"
                                }
                            }
                        } else {
                            for panel in &panels {
                                {
                                    let pid = panel.panel_id;
                                    let is_selected = *selected_panel_id.read() == Some(pid);
                                    let name = panel.name.clone();
                                    let time_room = format!(
                                        "{} · {}",
                                        panel.time_range_str,
                                        if panel.room_names.is_empty() { "—".to_string() }
                                        else { panel.room_names.join(", ") }
                                    );
                                    let code = panel.code.clone();
                                    let state_class = change_state_class(&panel.change_state);
                                    let is_deleted = panel.change_state == ChangeState::Deleted;
                                    let card_class = format!(
                                        "panel-card {state_class}{}",
                                        if is_selected { " selected" } else { "" }
                                    );
                                    let pname = panel.name.clone();
                                    rsx! {
                                        div {
                                            class: "{card_class}",
                                            style: if is_deleted { "opacity: 0.55" } else { "" },
                                            onclick: move |_| {
                                                selected_panel_id.set(Some(pid));
                                                editing_name.set(pname.clone());
                                            },
                                            div { class: "card-body",
                                                div { class: "card-header-row",
                                                    span {
                                                        class: if is_deleted { "card-name deleted" } else { "card-name" },
                                                        "{name}"
                                                    }
                                                    span { class: "card-code", "{code}" }
                                                }
                                                div { class: "card-meta", "{time_room}" }
                                                if state_class != "" {
                                                    div { class: "card-badge {state_class}",
                                                        {match panel.change_state {
                                                            ChangeState::Added => "Added",
                                                            ChangeState::Modified => "Modified",
                                                            ChangeState::Deleted => "Deleted",
                                                            ChangeState::Unchanged => "",
                                                        }}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Detail pane
                    if let Some(ref info) = detail_info {
                        {
                            let pid = info.panel_id;
                            let code = info.code.clone();
                            let time_str = info.time_range_str.clone();
                            let rooms_str = if info.room_names.is_empty() {
                                "—".to_string()
                            } else {
                                info.room_names.join(", ")
                            };
                            let desc = info.description.clone();
                            rsx! {
                                div { class: "detail-pane",
                                    div { class: "detail-header",
                                        span { class: "detail-code", "{code}" }
                                        button {
                                            class: "detail-close",
                                            onclick: move |_| selected_panel_id.set(None),
                                            "×"
                                        }
                                    }
                                    div { class: "detail-field",
                                        label { class: "detail-label", "Name" }
                                        input {
                                            class: "detail-input",
                                            r#type: "text",
                                            value: "{editing_name}",
                                            oninput: move |e| editing_name.set(e.value()),
                                        }
                                    }
                                    div { class: "detail-field",
                                        label { class: "detail-label", "Time" }
                                        div { class: "detail-value", "{time_str}" }
                                    }
                                    div { class: "detail-field",
                                        label { class: "detail-label", "Room" }
                                        div { class: "detail-value", "{rooms_str}" }
                                    }
                                    if let Some(ref d) = desc {
                                        div { class: "detail-field",
                                            label { class: "detail-label", "Description" }
                                            div { class: "detail-value detail-desc", "{d}" }
                                        }
                                    }
                                    button {
                                        class: "save-btn",
                                        onclick: move |_| {
                                            let new_name = editing_name.read().trim().to_string();
                                            if new_name.is_empty() {
                                                status.set(Some("Name cannot be empty".to_string()));
                                                return;
                                            }
                                            if let Some(ref mut c) = *ctx.write() {
                                                match c.update_field_cmd(
                                                    pid,
                                                    "name",
                                                    FieldValue::Single(FieldValueItem::String(new_name.clone())),
                                                ) {
                                                    Ok(cmd) => match c.apply(cmd) {
                                                        Ok(()) => {
                                                            status.set(Some(format!("Saved: {new_name}")));
                                                            selected_panel_id.set(None);
                                                        }
                                                        Err(e) => status.set(Some(format!("Save failed: {e}"))),
                                                    },
                                                    Err(e) => status.set(Some(format!("Save failed: {e}"))),
                                                }
                                            }
                                        },
                                        "Save Name"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
