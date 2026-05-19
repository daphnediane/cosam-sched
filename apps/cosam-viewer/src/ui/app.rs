/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Main viewer UI component.

use dioxus::prelude::*;
#[cfg(feature = "desktop")]
use dioxus_desktop::{use_muda_event_handler, use_window};

use crate::data::ScheduleDoc;
use crate::state::{Filters, Theme, ViewMode, ViewerState};
use crate::ui::grid::GridView;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Group a flat panel list into (time-label, panels) buckets by start time.
fn group_by_time(
    panels: Vec<crate::state::PanelView>,
) -> Vec<(String, Vec<crate::state::PanelView>)> {
    let mut groups: Vec<(String, Vec<crate::state::PanelView>)> = vec![];
    for panel in panels {
        let key = panel
            .start_time
            .map(|dt| dt.format("%-I:%M %p").to_string())
            .unwrap_or_default();
        if let Some(last) = groups.last_mut() {
            if last.0 == key {
                last.1.push(panel);
                continue;
            }
        }
        groups.push((key, vec![panel]));
    }
    groups
}

#[cfg(feature = "desktop")]
fn load_doc_from_path(
    path: std::path::PathBuf,
    name: Option<String>,
) -> anyhow::Result<(ScheduleDoc, Option<String>)> {
    let doc = ScheduleDoc::from_path(&path)?;
    Ok((doc, name))
}

fn load_doc_from_url(url: String) -> anyhow::Result<(ScheduleDoc, Option<String>)> {
    let doc = ScheduleDoc::from_url(&url)?;
    Ok((doc, Some(url)))
}

// ---------------------------------------------------------------------------
// Root App component
// ---------------------------------------------------------------------------

#[component]
pub fn App() -> Element {
    let mut state: Signal<ViewerState> = use_signal(ViewerState::default);
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);
    let mut url_input: Signal<String> = use_signal(String::new);
    #[cfg(feature = "desktop")]
    let mut trigger_open_file: Signal<bool> = use_signal(|| false);
    #[cfg(feature = "desktop")]
    let mut trigger_open_folder: Signal<bool> = use_signal(|| false);

    // -----------------------------------------------------------------------
    // Derived data (read once to avoid repeated borrows)
    // -----------------------------------------------------------------------
    let (days, panels, grid_rooms, filter_rooms, filter_types, filter_presenters, title) = {
        use std::collections::HashSet;
        let s = state.read();
        let days = s.days.clone();
        let panels = s.panels_for_day();
        let visible_rooms = s
            .doc
            .as_ref()
            .map(|d| d.visible_rooms().into_iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        // Grid shows only rooms that have at least one non-break panel on the
        // selected day (empty columns would be confusing).
        let panel_room_ids: HashSet<i32> = panels
            .iter()
            .filter(|p| !p.is_break)
            .flat_map(|p| p.room_ids.iter().copied())
            .collect();
        let grid_rooms = visible_rooms
            .iter()
            .filter(|r| panel_room_ids.contains(&r.uid))
            .cloned()
            .collect::<Vec<_>>();
        let filter_rooms = visible_rooms;
        let filter_types = s
            .doc
            .as_ref()
            .map(|d| {
                d.visible_types()
                    .into_iter()
                    .map(|(k, _)| k.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let filter_presenters = s.presenter_names_for_filter();
        let title = s
            .doc
            .as_ref()
            .map(|d| d.meta.title.clone())
            .unwrap_or_else(|| "cosam-viewer".to_string());
        (
            days,
            panels,
            grid_rooms,
            filter_rooms,
            filter_types,
            filter_presenters,
            title,
        )
    };

    // -----------------------------------------------------------------------
    // Desktop-only hooks: menu events + window title
    // -----------------------------------------------------------------------
    #[cfg(feature = "desktop")]
    use_muda_event_handler(move |event| {
        let id = event.id().0.as_str();
        match id {
            crate::ID_FILE_OPEN => trigger_open_file.set(true),
            crate::ID_FILE_OPEN_FOLDER => trigger_open_folder.set(true),
            _ => {}
        }
    });

    #[cfg(feature = "desktop")]
    {
        let title_for_effect = title.clone();
        use_effect(move || {
            use_window().window.set_title(&title_for_effect);
        });
    }

    // Trigger file open dialog (from menu or empty-state button)
    #[cfg(feature = "desktop")]
    use_effect(move || {
        if !*trigger_open_file.read() {
            return;
        }
        trigger_open_file.set(false);
        spawn(async move {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("Schedule files", &["json", "xlsx", "cosam"])
                .add_filter("Widget JSON", &["json"])
                .add_filter("XLSX spreadsheet", &["xlsx"])
                .add_filter("Binary schedule", &["cosam"])
                .add_filter("All files", &["*"])
                .pick_file()
                .await;
            if let Some(handle) = file {
                let name = handle.file_name();
                let path = handle.path().to_path_buf();
                match tokio::task::spawn_blocking(move || load_doc_from_path(path, Some(name)))
                    .await
                {
                    Ok(Ok((doc, fname))) => {
                        state.write().load_doc(doc, fname);
                        error_msg.set(None);
                    }
                    Ok(Err(e)) => error_msg.set(Some(format!("Failed to load: {e}"))),
                    Err(e) => error_msg.set(Some(format!("Load error: {e}"))),
                }
            }
        });
    });

    // Trigger folder open dialog (from menu or empty-state button)
    #[cfg(feature = "desktop")]
    use_effect(move || {
        if !*trigger_open_folder.read() {
            return;
        }
        trigger_open_folder.set(false);
        spawn(async move {
            let folder = rfd::AsyncFileDialog::new().pick_folder().await;
            if let Some(handle) = folder {
                let name = handle.file_name();
                let path = handle.path().to_path_buf();
                match tokio::task::spawn_blocking(move || load_doc_from_path(path, Some(name)))
                    .await
                {
                    Ok(Ok((doc, fname))) => {
                        state.write().load_doc(doc, fname);
                        error_msg.set(None);
                    }
                    Ok(Err(e)) => error_msg.set(Some(format!("Failed to load folder: {e}"))),
                    Err(e) => error_msg.set(Some(format!("Load error: {e}"))),
                }
            }
        });
    });

    let theme_class = state.read().theme.css_class();
    let show_filter_panel = state.read().filters.show_filter_panel;
    let has_doc = state.read().doc.is_some();
    let view_mode = state.read().view_mode;
    let selected_day_index = state.read().selected_day_index; // None = All Days
    let detail_panel = state.read().detail_panel();
    let active_presenter_filter = state.read().filters.presenter.clone();
    let time_groups = group_by_time(panels.clone());

    // -----------------------------------------------------------------------
    // File / folder / URL open handlers
    // -----------------------------------------------------------------------
    // Desktop: delegate to trigger signals so menu and button share the same path.
    #[cfg(feature = "desktop")]
    let open_file = move |_| trigger_open_file.set(true);

    #[cfg(feature = "desktop")]
    let open_folder = move |_| trigger_open_folder.set(true);

    let trigger_load_url = move || {
        let url = url_input.read().trim().to_string();
        if url.is_empty() {
            return;
        }
        spawn(async move {
            match tokio::task::spawn_blocking(move || load_doc_from_url(url)).await {
                Ok(Ok((doc, fname))) => {
                    state.write().load_doc(doc, fname);
                    error_msg.set(None);
                    url_input.set(String::new());
                }
                Ok(Err(e)) => error_msg.set(Some(format!("Failed to load URL: {e}"))),
                Err(e) => error_msg.set(Some(format!("Load error: {e}"))),
            }
        });
    };

    #[cfg(not(feature = "desktop"))]
    let open_file = move |_| {
        error_msg.set(Some(
            "File open not yet implemented on this platform.".to_string(),
        ));
    };

    #[cfg(not(feature = "desktop"))]
    let open_folder = move |_| {
        error_msg.set(Some(
            "Folder open not yet implemented on this platform.".to_string(),
        ));
    };

    // -----------------------------------------------------------------------
    // Render
    // -----------------------------------------------------------------------
    rsx! {
        style { {include_str!("../style.css")} }

        div { class: "viewer-root {theme_class}",
            a { class: "skip-link", href: "#main-content", "Skip to content" }

            // ---------------------------------------------------------------
            // Top bar: view icons (left) + day tabs (center) + filter/theme (right)
            // ---------------------------------------------------------------
            header { class: "topbar", role: "banner",
                // Left: list/grid icon buttons (only when a schedule is loaded)
                div { class: "topbar-start",
                    if has_doc {
                        div { class: "toolbar-view-toggle", role: "group", aria_label: "View mode",
                            // List view icon (horizontal lines)
                            button {
                                class: if view_mode == ViewMode::List { "toolbar-icon-btn active" } else { "toolbar-icon-btn" },
                                aria_pressed: if view_mode == ViewMode::List { "true" } else { "false" },
                                aria_label: "List view",
                                title: "List view",
                                onclick: move |_| state.write().view_mode = ViewMode::List,
                                dangerous_inner_html: r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/></svg>"#,
                            }
                            // Grid view icon (2×2 grid)
                            button {
                                class: if view_mode == ViewMode::Grid { "toolbar-icon-btn active" } else { "toolbar-icon-btn" },
                                aria_pressed: if view_mode == ViewMode::Grid { "true" } else { "false" },
                                aria_label: "Grid view",
                                title: "Grid view",
                                onclick: move |_| {
                                    let mut s = state.write();
                                    s.view_mode = ViewMode::Grid;
                                    // Grid view requires a specific day — auto-select day 0.
                                    if s.selected_day_index.is_none() && !s.days.is_empty() {
                                        s.selected_day_index = Some(0);
                                    }
                                },
                                dangerous_inner_html: r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/></svg>"#,
                            }
                        }
                    }
                }

                // Centre: day tabs (only when a schedule is loaded)
                if has_doc {
                    nav { class: "topbar-days", aria_label: "Convention days",
                        // All Days tab
                        {
                            let is_active = selected_day_index.is_none();
                            rsx! {
                                button {
                                    class: if is_active { "day-tab day-tab-active" } else { "day-tab" },
                                    aria_selected: if is_active { "true" } else { "false" },
                                    onclick: move |_| {
                                        state.write().selected_day_index = None;
                                        state.write().detail_panel_id = None;
                                    },
                                    "All Days"
                                }
                            }
                        }
                        // Per-day tabs
                        for (i, day) in days.iter().enumerate() {
                            {
                                let label = day.format("%a %-d").to_string();
                                let is_active = selected_day_index == Some(i);
                                rsx! {
                                    button {
                                        class: if is_active { "day-tab day-tab-active" } else { "day-tab" },
                                        aria_selected: if is_active { "true" } else { "false" },
                                        onclick: move |_| {
                                            state.write().selected_day_index = Some(i);
                                            state.write().detail_panel_id = None;
                                        },
                                        "{label}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Right: filter toggle + theme picker
                div { class: "topbar-end",
                    if has_doc {
                        button {
                            class: if show_filter_panel { "toolbar-btn active" } else { "toolbar-btn" },
                            aria_expanded: if show_filter_panel { "true" } else { "false" },
                            aria_label: "Toggle filters",
                            onclick: move |_| {
                                state.write().filters.show_filter_panel = !show_filter_panel;
                            },
                            "Filter"
                        }
                    }
                    label { class: "sr-only", r#for: "theme-select", "Theme" }
                    select {
                        id: "theme-select",
                        class: "toolbar-select",
                        aria_label: "Select theme",
                        onchange: move |e| {
                            let theme = match e.value().as_str() {
                                "light" => Theme::Light,
                                "dark" => Theme::Dark,
                                "high-contrast" => Theme::HighContrast,
                                _ => Theme::Cosam,
                            };
                            state.write().theme = theme;
                        },
                        option { value: "cosam", "Default" }
                        option { value: "light", "Light" }
                        option { value: "dark", "Dark" }
                        option { value: "high-contrast", "High Contrast" }
                    }
                }
            }

            // ---------------------------------------------------------------
            // Error banner
            // ---------------------------------------------------------------
            if let Some(ref msg) = *error_msg.read() {
                div { class: "error-banner", role: "alert",
                    span { "{msg}" }
                    button {
                        class: "error-dismiss",
                        aria_label: "Dismiss error",
                        onclick: move |_| error_msg.set(None),
                        "×"
                    }
                }
            }

            // ---------------------------------------------------------------
            // Empty state
            // ---------------------------------------------------------------
            if !has_doc {
                main { id: "main-content", class: "empty-state",
                    div { class: "empty-state-inner",
                        h1 { class: "empty-title", "cosam Schedule Viewer" }
                        p { class: "empty-sub",
                            "Open a schedule to get started. Supported: widget JSON, XLSX, binary .cosam, or a CSV directory."
                        }
                        div { class: "empty-actions",
                            button { class: "btn-primary", onclick: open_file, "Open File" }
                            button { class: "btn-secondary", onclick: open_folder, "Open Folder (CSV)" }
                        }
                        div { class: "empty-url-section",
                            p { class: "empty-sub", "Or load from a webpage URL:" }
                            div { class: "url-input-row",
                                label { class: "sr-only", r#for: "url-input", "Schedule URL" }
                                input {
                                    id: "url-input",
                                    class: "url-input",
                                    r#type: "url",
                                    placeholder: "https://example.com/schedule",
                                    value: "{url_input.read()}",
                                    oninput: move |e| url_input.set(e.value()),
                                    onkeydown: move |e| {
                                        if e.key() == Key::Enter {
                                            trigger_load_url();
                                        }
                                    },
                                    aria_label: "Schedule webpage URL",
                                }
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| trigger_load_url(),
                                    disabled: url_input.read().trim().is_empty(),
                                    "Load URL"
                                }
                            }
                        }
                    }
                }
            } else {
                // -----------------------------------------------------------
                // Filter panel
                // -----------------------------------------------------------
                if show_filter_panel {
                    section { class: "filter-panel", aria_label: "Filters",
                        // Search
                        div { class: "filter-section",
                            label { class: "filter-label", r#for: "search-input", "Search" }
                            input {
                                id: "search-input",
                                class: "filter-search",
                                r#type: "search",
                                placeholder: "Name, description, presenter…",
                                value: "{state.read().filters.search}",
                                oninput: move |e| state.write().filters.search = e.value(),
                                aria_label: "Search panels",
                            }
                        }

                        // Presenter dropdown
                        if !filter_presenters.is_empty() {
                            div { class: "filter-section",
                                label {
                                    class: "filter-label",
                                    r#for: "presenter-select",
                                    "Presenter"
                                }
                                select {
                                    id: "presenter-select",
                                    class: "filter-select",
                                    aria_label: "Filter by presenter",
                                    onchange: move |e| {
                                        let v = e.value();
                                        state.write().filters.presenter =
                                            if v.is_empty() { None } else { Some(v) };
                                    },
                                    option { value: "", "— All Presenters —" }
                                    for name in &filter_presenters {
                                        {
                                            let selected =
                                                active_presenter_filter.as_deref() == Some(name.as_str());
                                            rsx! {
                                                option {
                                                    value: "{name}",
                                                    selected: "{selected}",
                                                    "{name}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Room chips
                        if !filter_rooms.is_empty() {
                            fieldset { class: "filter-section",
                                legend { class: "filter-label", "Rooms" }
                                div { class: "filter-chips",
                                    for room in &filter_rooms {
                                        {
                                            let uid = room.uid;
                                            let name = room.long_name.clone();
                                            let checked = state.read().filters.rooms.contains(&uid);
                                            let cb_id = format!("room-{uid}");
                                            rsx! {
                                                label {
                                                    class: if checked { "chip chip-active" } else { "chip" },
                                                    r#for: "{cb_id}",
                                                    input {
                                                        id: "{cb_id}",
                                                        r#type: "checkbox",
                                                        class: "chip-check",
                                                        checked: "{checked}",
                                                        onchange: move |e| {
                                                            if e.checked() {
                                                                state.write().filters.rooms.insert(uid);
                                                            } else {
                                                                state.write().filters.rooms.remove(&uid);
                                                            }
                                                        },
                                                    }
                                                    "{name}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Event type chips
                        if !filter_types.is_empty() {
                            fieldset { class: "filter-section",
                                legend { class: "filter-label", "Event Types" }
                                div { class: "filter-chips",
                                    for type_key in &filter_types {
                                        {
                                            let key = type_key.clone();
                                            let checked = state.read().filters.types.contains(&key);
                                            let cb_id = format!("type-{}", key.to_lowercase());
                                            rsx! {
                                                label {
                                                    class: if checked { "chip chip-active" } else { "chip" },
                                                    r#for: "{cb_id}",
                                                    input {
                                                        id: "{cb_id}",
                                                        r#type: "checkbox",
                                                        class: "chip-check",
                                                        checked: "{checked}",
                                                        onchange: move |e| {
                                                            if e.checked() {
                                                                state.write().filters.types.insert(key.clone());
                                                            } else {
                                                                state.write().filters.types.remove(&key);
                                                            }
                                                        },
                                                    }
                                                    "{key}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Clear filters
                        if !state.read().filters.is_default() {
                            button {
                                class: "btn-secondary",
                                onclick: move |_| {
                                    let mut s = state.write();
                                    let show = s.filters.show_filter_panel;
                                    s.filters = Filters::default();
                                    s.filters.show_filter_panel = show;
                                },
                                "Clear Filters"
                            }
                        }
                    }
                }

                // -----------------------------------------------------------
                // Main content area — list view or grid view
                // -----------------------------------------------------------
                if view_mode == ViewMode::Grid {
                    main { id: "main-content", class: "grid-main-area",
                        GridView {
                            panels,
                            rooms: grid_rooms,
                            on_panel_click: move |id: String| {
                                state.write().detail_panel_id = Some(id);
                            },
                        }
                    }
                } else {
                main { id: "main-content", class: "panel-list-area",
                    if time_groups.is_empty() {
                        div { class: "empty-state-inline",
                            "No panels match the current filters."
                        }
                    } else {
                        for (time_label, group_panels) in &time_groups {
                            section {
                                class: "time-group",
                                aria_label: "Events at {time_label}",

                                // Sticky time header — only shown when at least
                                // one non-break panel is in the group.
                                if group_panels.iter().any(|p| !p.is_break) {
                                    div { class: "time-header", aria_hidden: "true",
                                        span { class: "time-label", "{time_label}" }
                                    }
                                }

                                div { class: "panel-cards",
                                    for panel in group_panels {
                                        if panel.is_break {
                                            // -------------------------------------------------------
                                            // Break banner
                                            // -------------------------------------------------------
                                            {
                                                let dur = panel.end_time
                                                    .zip(panel.start_time)
                                                    .map(|(e, s)| (e - s).num_minutes());
                                                let dur_str = match dur {
                                                    Some(m) if m >= 60 => {
                                                        let h = m / 60;
                                                        let min = m % 60;
                                                        if min == 0 {
                                                            format!("{h}h break")
                                                        } else {
                                                            format!("{h}h {min}m break")
                                                        }
                                                    }
                                                    Some(m) => format!("{m}m break"),
                                                    None => "Break".to_string(),
                                                };
                                                rsx! {
                                                    div {
                                                        class: "break-banner",
                                                        role: "separator",
                                                        aria_label: "{dur_str}",
                                                        span { class: "break-label", "{dur_str}" }
                                                    }
                                                }
                                            }
                                        } else {
                                            // -------------------------------------------------------
                                            // Regular panel card
                                            // -------------------------------------------------------
                                            {
                                                let pid = panel.id.clone();
                                                let pid2 = pid.clone();
                                                let pname = panel.name.clone();
                                                let time_str = panel.time_str.clone();
                                                let rooms = panel.room_names.join(", ");
                                                let type_key = panel.panel_type.clone();
                                                let color = panel.type_color.clone();
                                                let is_workshop = panel.is_workshop;
                                                let is_premium = panel.is_premium;
                                                let is_full = panel.is_full;
                                                let is_kids = panel.is_kids;
                                                let credits = panel.credits.clone();
                                                rsx! {
                                                    article {
                                                        class: "panel-card",
                                                        "data-panel-type": "{type_key}",
                                                        tabindex: "0",
                                                        role: "button",
                                                        aria_label: "View details for {pname}",
                                                        onclick: move |_| {
                                                            state.write().detail_panel_id =
                                                                Some(pid.clone());
                                                        },
                                                        onkeydown: move |e| {
                                                            if e.key() == Key::Enter
                                                                || e.key()
                                                                    == Key::Character(
                                                                        " ".to_string(),
                                                                    )
                                                            {
                                                                state.write().detail_panel_id =
                                                                    Some(pid2.clone());
                                                            }
                                                        },

                                                        div {
                                                            class: "card-color-bar",
                                                            style: if let Some(ref c) = color {
                                                                format!("background:{c}")
                                                            } else {
                                                                String::new()
                                                            },
                                                        }

                                                        div { class: "card-body",
                                                            div { class: "card-header-row",
                                                                h3 { class: "card-name", "{pname}" }
                                                                div { class: "card-badges",
                                                                    if is_workshop {
                                                                        span {
                                                                            class: "badge badge-workshop",
                                                                            "Workshop"
                                                                        }
                                                                    }
                                                                    if is_premium {
                                                                        span {
                                                                            class: "badge badge-paid",
                                                                            "Paid"
                                                                        }
                                                                    }
                                                                    if is_full {
                                                                        span {
                                                                            class: "badge badge-full",
                                                                            "Full"
                                                                        }
                                                                    }
                                                                    if is_kids {
                                                                        span {
                                                                            class: "badge badge-kids",
                                                                            "Kids"
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            div { class: "card-meta",
                                                                span { class: "card-time", "{time_str}" }
                                                                if !rooms.is_empty() {
                                                                    span { class: "card-sep", " · " }
                                                                    span { class: "card-rooms", "{rooms}" }
                                                                }
                                                            }
                                                            if !credits.is_empty() {
                                                                div { class: "card-credits",
                                                                    "{credits.join(\", \")}"
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
                        }
                    }
                }
                } // end else (list view)

                // -----------------------------------------------------------
                // Detail modal
                // -----------------------------------------------------------
                if let Some(ref panel) = detail_panel {
                    {
                        let pname = panel.name.clone();
                        let time_str = panel.time_str.clone();
                        let rooms = panel.room_names.join(", ");
                        let desc = panel.description.clone();
                        let note = panel.note.clone();
                        let prereq = panel.prereq.clone();
                        let cost = panel.cost.clone();
                        let capacity = panel.capacity.clone();
                        let difficulty = panel.difficulty.clone();
                        let ticket_url = panel.ticket_url.clone();
                        let credits = panel.credits.clone();
                        let is_premium = panel.is_premium;
                        let is_kids = panel.is_kids;
                        rsx! {
                            div {
                                class: "modal-backdrop",
                                role: "dialog",
                                aria_modal: "true",
                                aria_label: "Panel details: {pname}",
                                onclick: move |_| state.write().detail_panel_id = None,

                                div {
                                    class: "modal-content",
                                    onclick: move |e| e.stop_propagation(),

                                    div { class: "modal-header",
                                        h2 { class: "modal-title", "{pname}" }
                                        button {
                                            class: "modal-close",
                                            aria_label: "Close panel details",
                                            onclick: move |_| state.write().detail_panel_id = None,
                                            "×"
                                        }
                                    }

                                    div { class: "modal-meta",
                                        if !time_str.is_empty() {
                                            div { class: "modal-field",
                                                span { class: "modal-label", "Time: " }
                                                span { "{time_str}" }
                                            }
                                        }
                                        if !rooms.is_empty() {
                                            div { class: "modal-field",
                                                span { class: "modal-label", "Room: " }
                                                span { "{rooms}" }
                                            }
                                        }
                                        if !credits.is_empty() {
                                            div { class: "modal-field",
                                                span { class: "modal-label", "Presenters: " }
                                                span { "{credits.join(\", \")}" }
                                            }
                                        }
                                        if is_premium {
                                            if let Some(ref c) = cost {
                                                div { class: "modal-field",
                                                    span { class: "modal-label", "Cost: " }
                                                    span { "{c}" }
                                                }
                                            }
                                        }
                                        if let Some(ref cap) = capacity {
                                            div { class: "modal-field",
                                                span { class: "modal-label", "Capacity: " }
                                                span { "{cap}" }
                                            }
                                        }
                                        if let Some(ref diff) = difficulty {
                                            div { class: "modal-field",
                                                span { class: "modal-label", "Difficulty: " }
                                                span { "{diff}" }
                                            }
                                        }
                                        if is_kids {
                                            div { class: "modal-field",
                                                span { class: "badge badge-kids", "Kids programming" }
                                            }
                                        }
                                    }

                                    if let Some(ref d) = desc {
                                        div { class: "modal-section",
                                            h3 { class: "modal-section-title", "Description" }
                                            p { class: "modal-text", "{d}" }
                                        }
                                    }
                                    if let Some(ref p) = prereq {
                                        div { class: "modal-section",
                                            h3 { class: "modal-section-title", "Prerequisites" }
                                            p { class: "modal-text", "{p}" }
                                        }
                                    }
                                    if let Some(ref n) = note {
                                        div { class: "modal-section",
                                            h3 { class: "modal-section-title", "Notes" }
                                            p { class: "modal-text", "{n}" }
                                        }
                                    }
                                    if let Some(ref url) = ticket_url {
                                        div { class: "modal-section",
                                            a {
                                                class: "modal-ticket-link",
                                                href: "{url}",
                                                target: "_blank",
                                                rel: "noopener noreferrer",
                                                "Buy Tickets / Register"
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
    }
}
