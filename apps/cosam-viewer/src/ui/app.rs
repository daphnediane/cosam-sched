/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Main viewer UI component.

use dioxus::prelude::*;

use crate::data::ScheduleDoc;
use crate::state::{Filters, Theme, ViewerState};

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

fn load_doc_from_bytes(
    bytes: Vec<u8>,
    name: Option<String>,
) -> anyhow::Result<(ScheduleDoc, Option<String>)> {
    let doc = ScheduleDoc::from_json(&bytes)?;
    Ok((doc, name))
}

// ---------------------------------------------------------------------------
// Root App component
// ---------------------------------------------------------------------------

#[component]
pub fn App() -> Element {
    let mut state: Signal<ViewerState> = use_signal(ViewerState::default);
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);

    // -----------------------------------------------------------------------
    // Derived data (read once to avoid repeated borrows)
    // -----------------------------------------------------------------------
    let (days, panels, filter_rooms, filter_types, filter_presenters, title) = {
        let s = state.read();
        let days = s.days.clone();
        let panels = s.panels_for_day();
        let filter_rooms = s
            .doc
            .as_ref()
            .map(|d| d.visible_rooms().into_iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
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
            filter_rooms,
            filter_types,
            filter_presenters,
            title,
        )
    };

    let theme_class = state.read().theme.css_class();
    let show_filter_panel = state.read().filters.show_filter_panel;
    let has_doc = state.read().doc.is_some();
    let selected_day_index = state.read().selected_day_index; // None = All Days
    let detail_panel = state.read().detail_panel();
    let active_presenter_filter = state.read().filters.presenter.clone();
    let time_groups = group_by_time(panels);

    // -----------------------------------------------------------------------
    // File open handler
    // -----------------------------------------------------------------------
    #[cfg(feature = "desktop")]
    let open_file = move |_| {
        spawn(async move {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("cosam JSON", &["json"])
                .add_filter("All files", &["*"])
                .pick_file()
                .await;

            if let Some(handle) = file {
                let name = handle.file_name();
                let bytes = handle.read().await;
                match load_doc_from_bytes(bytes, Some(name)) {
                    Ok((doc, fname)) => {
                        state.write().load_doc(doc, fname);
                        error_msg.set(None);
                    }
                    Err(e) => error_msg.set(Some(format!("Failed to load: {e}"))),
                }
            }
        });
    };

    #[cfg(not(feature = "desktop"))]
    let open_file = move |_| {
        error_msg.set(Some(
            "File open not yet implemented on this platform.".to_string(),
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
            // Top bar: toolbar controls + day tabs in one row
            // ---------------------------------------------------------------
            header { class: "topbar", role: "banner",
                // Left: open + title
                div { class: "topbar-start",
                    button {
                        class: "toolbar-btn toolbar-open",
                        onclick: open_file,
                        aria_label: "Open schedule file",
                        "Open"
                    }
                    span { class: "toolbar-title", "{title}" }
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
                            "Open a cosam widget JSON file to get started."
                        }
                        button { class: "btn-primary", onclick: open_file, "Open Schedule" }
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
                // Panel list (time-grouped)
                // -----------------------------------------------------------
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
