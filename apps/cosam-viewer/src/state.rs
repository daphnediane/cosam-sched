/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Application state for the cosam-viewer.

use std::collections::HashSet;

use chrono::NaiveDate;

use crate::data::ScheduleDoc;

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Cosam,
    Light,
    Dark,
    HighContrast,
}

impl Theme {
    pub fn css_class(self) -> &'static str {
        match self {
            Theme::Cosam => "theme-cosam",
            Theme::Light => "theme-light",
            Theme::Dark => "theme-dark",
            Theme::HighContrast => "theme-high-contrast",
        }
    }

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            Theme::Cosam => "Default",
            Theme::Light => "Light",
            Theme::Dark => "Dark",
            Theme::HighContrast => "High Contrast",
        }
    }

    #[allow(dead_code)]
    pub const ALL: &'static [Theme] = &[
        Theme::Cosam,
        Theme::Light,
        Theme::Dark,
        Theme::HighContrast,
    ];
}

// ---------------------------------------------------------------------------
// View mode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    List,
    // Grid view is a future work item (FEATURE-116a).
}

// ---------------------------------------------------------------------------
// Filters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct Filters {
    pub search: String,
    /// Empty = all rooms shown.
    pub rooms: HashSet<u32>,
    /// Empty = all types shown.
    pub types: HashSet<String>,
    /// None = all presenters shown.
    pub presenter: Option<String>,
    pub show_filter_panel: bool,
}

impl Filters {
    pub fn is_default(&self) -> bool {
        self.search.is_empty()
            && self.rooms.is_empty()
            && self.types.is_empty()
            && self.presenter.is_none()
    }
}

// ---------------------------------------------------------------------------
// Derived panel info for display
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PanelView {
    pub id: String,
    pub name: String,
    pub panel_type: String,
    pub type_color: Option<String>,
    pub room_names: Vec<String>,
    pub start_time: Option<chrono::NaiveDateTime>,
    #[allow(dead_code)] // reserved for grid view (FEATURE-116a)
    pub end_time: Option<chrono::NaiveDateTime>,
    pub time_str: String,
    pub description: Option<String>,
    pub note: Option<String>,
    pub prereq: Option<String>,
    pub cost: Option<String>,
    pub capacity: Option<String>,
    pub difficulty: Option<String>,
    pub ticket_url: Option<String>,
    pub is_premium: bool,
    pub is_full: bool,
    pub is_kids: bool,
    pub is_workshop: bool,
    pub is_break: bool,
    /// Formatted credit strings for display (may include group names).
    pub credits: Vec<String>,
    /// Raw individual presenter names for search/filter matching.
    /// Not rendered directly — credits is used for display.
    #[allow(dead_code)]
    pub presenter_names: Vec<String>,
}

// ---------------------------------------------------------------------------
// ViewerState
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct ViewerState {
    pub doc: Option<ScheduleDoc>,
    pub file_name: Option<String>,

    pub theme: Theme,
    #[allow(dead_code)] // reserved for grid view toggle (FEATURE-116a)
    pub view_mode: ViewMode,
    pub filters: Filters,

    /// All unique scheduled days in the document, sorted.
    pub days: Vec<NaiveDate>,
    pub selected_day_index: usize,

    /// Panel currently shown in detail modal.
    pub detail_panel_id: Option<String>,
}

impl ViewerState {
    pub fn load_doc(&mut self, doc: ScheduleDoc, file_name: Option<String>) {
        self.days = collect_days(&doc);
        self.selected_day_index = 0;
        self.filters = Filters::default();
        self.detail_panel_id = None;
        self.file_name = file_name;
        self.doc = Some(doc);
    }

    /// Return panels for the currently selected day, applying active filters.
    pub fn panels_for_day(&self) -> Vec<PanelView> {
        let doc = match &self.doc {
            Some(d) => d,
            None => return vec![],
        };
        let selected_day = match self.days.get(self.selected_day_index) {
            Some(d) => *d,
            None => return vec![],
        };

        let mut panels: Vec<PanelView> = doc
            .panels
            .iter()
            .filter_map(|p| {
                // Only show panels with a start time on the selected day.
                let start = parse_datetime(p.start_time.as_deref())?;
                if start.date() != selected_day {
                    return None;
                }

                // Look up panel type — skip hidden and timeline panels.
                let pt = doc.panel_types.get(&p.panel_type);
                if let Some(pt) = pt {
                    if pt.is_hidden || pt.is_timeline {
                        return None;
                    }
                }

                let is_break = pt.map(|t| t.is_break).unwrap_or(false);

                // Break panels bypass content filters (room, type, presenter,
                // search) — they represent schedule gaps across all rooms and
                // should always be visible when present.
                if !is_break {
                    // Room filter.
                    if !self.filters.rooms.is_empty()
                        && !p.room_ids.iter().any(|r| self.filters.rooms.contains(r))
                    {
                        return None;
                    }

                    // Type filter.
                    if !self.filters.types.is_empty()
                        && !self.filters.types.contains(&p.panel_type)
                    {
                        return None;
                    }

                    // Presenter filter.
                    if let Some(ref presenter_filter) = self.filters.presenter {
                        if !p
                            .presenters
                            .iter()
                            .any(|name| name == presenter_filter)
                        {
                            return None;
                        }
                    }

                    // Search filter — name, description, presenter names.
                    if !self.filters.search.is_empty() {
                        let needle = self.filters.search.to_lowercase();
                        let hay = format!(
                            "{} {} {}",
                            p.name.to_lowercase(),
                            p.description.as_deref().unwrap_or("").to_lowercase(),
                            p.presenters.join(" ").to_lowercase(),
                        );
                        if !hay.contains(&needle) {
                            return None;
                        }
                    }
                }

                let end = parse_datetime(p.end_time.as_deref());
                let time_str = format_time_range(Some(start), end);
                let room_names = p
                    .room_ids
                    .iter()
                    .filter_map(|uid| doc.room_by_uid(*uid).map(|r| r.long_name.clone()))
                    .collect();
                let type_color = pt
                    .and_then(|t| t.colors.as_ref())
                    .and_then(|c| c.color.clone());

                Some(PanelView {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    panel_type: p.panel_type.clone(),
                    type_color,
                    room_names,
                    start_time: Some(start),
                    end_time: end,
                    time_str,
                    description: p.description.clone(),
                    note: p.note.clone(),
                    prereq: p.prereq.clone(),
                    cost: p.cost.clone(),
                    capacity: p.capacity.map(|c| c.to_string()),
                    difficulty: p.difficulty.clone(),
                    ticket_url: p.ticket_url.clone(),
                    is_premium: p.is_premium,
                    is_full: p.is_full,
                    is_kids: p.is_kids,
                    is_workshop: pt.map(|t| t.is_workshop).unwrap_or(false),
                    is_break,
                    credits: p.credits.clone(),
                    presenter_names: p.presenters.clone(),
                })
            })
            .collect();

        panels.sort_by_key(|p| p.start_time);
        panels
    }

    /// All individual (non-group) presenter names that appear on at least one
    /// panel, sorted for display in the filter dropdown.
    pub fn presenter_names_for_filter(&self) -> Vec<String> {
        let doc = match &self.doc {
            Some(d) => d,
            None => return vec![],
        };
        let mut names: Vec<String> = doc
            .presenters
            .iter()
            .filter(|p| !p.is_group && !p.panel_ids.is_empty())
            .map(|p| p.name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    pub fn detail_panel(&self) -> Option<PanelView> {
        let id = self.detail_panel_id.as_deref()?;
        self.panels_for_day()
            .into_iter()
            .find(|p| p.id == id)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn collect_days(doc: &ScheduleDoc) -> Vec<NaiveDate> {
    use std::collections::BTreeSet;
    let mut dates: BTreeSet<NaiveDate> = BTreeSet::new();
    for panel in &doc.panels {
        if let Some(dt) = parse_datetime(panel.start_time.as_deref()) {
            dates.insert(dt.date());
        }
    }
    dates.into_iter().collect()
}

/// Parse an ISO-8601 / RFC 3339 datetime string (widget JSON uses e.g. "2026-07-03T10:00:00").
pub fn parse_datetime(s: Option<&str>) -> Option<chrono::NaiveDateTime> {
    let s = s?;
    // Try naive datetime first, then strip tz offset.
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt);
    }
    // Fallback: truncate timezone and retry.
    let trimmed = s.get(..19)?;
    chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S").ok()
}

fn format_time_range(
    start: Option<chrono::NaiveDateTime>,
    end: Option<chrono::NaiveDateTime>,
) -> String {
    match (start, end) {
        (Some(s), Some(e)) => {
            format!(
                "{} – {}",
                s.format("%-I:%M %p"),
                e.format("%-I:%M %p"),
            )
        }
        (Some(s), None) => s.format("%-I:%M %p").to_string(),
        _ => String::new(),
    }
}
