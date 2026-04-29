/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget JSON export functionality.
//!
//! Converts from the internal CRDT/field-system format to the widget JSON display format
//! documented in `docs/widget-json-format.md`.

use crate::schedule::Schedule;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Widget JSON Structures ───────────────────────────────────────────────────────

/// Top-level metadata for widget JSON export.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetMeta {
    pub title: String,
    pub version: i32,
    pub variant: String,
    pub generator: String,
    pub generated: String,
    pub modified: String,
    pub start_time: String,
    pub end_time: String,
}

/// Panel entry in widget JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPanel {
    pub id: String,
    pub base_id: String,
    pub part_num: Option<i32>,
    pub session_num: Option<i32>,
    pub name: String,
    pub panel_type: Option<String>,
    pub room_ids: Vec<i32>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub duration: i32,
    pub description: Option<String>,
    pub note: Option<String>,
    pub prereq: Option<String>,
    pub cost: Option<String>,
    pub capacity: Option<String>,
    pub difficulty: Option<String>,
    pub ticket_url: Option<String>,
    pub is_free: bool,
    pub is_full: bool,
    pub is_kids: bool,
    pub credits: Vec<String>,
    pub presenters: Vec<String>,
}

/// Room entry in widget JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetRoom {
    pub uid: i32,
    pub short_name: String,
    pub long_name: String,
    pub hotel_room: String,
    pub sort_key: i32,
    pub is_break: bool,
}

/// Panel type entry in widget JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPanelType {
    pub kind: String,
    pub colors: HashMap<String, String>,
    pub is_break: bool,
    pub is_cafe: bool,
    pub is_workshop: bool,
    pub is_hidden: bool,
    pub is_room_hours: bool,
    pub is_timeline: bool,
    pub is_private: bool,
}

/// Timeline entry in widget JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetTimeline {
    pub id: String,
    pub start_time: String,
    pub description: String,
    pub panel_type: Option<String>,
    pub note: Option<String>,
}

/// Presenter entry in widget JSON format (DisplayPresenter).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPresenter {
    pub name: String,
    pub rank: String,
    pub sort_key: i32,
    pub is_group: bool,
    pub members: Vec<String>,
    pub groups: Vec<String>,
    pub always_grouped: bool,
    pub always_shown: bool,
    pub panel_ids: Vec<String>,
}

/// Complete widget JSON export structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetExport {
    pub meta: WidgetMeta,
    pub panels: Vec<WidgetPanel>,
    pub rooms: Vec<WidgetRoom>,
    pub panel_types: HashMap<String, WidgetPanelType>,
    pub timeline: Vec<WidgetTimeline>,
    pub presenters: Vec<WidgetPresenter>,
}

// ── Export Function ───────────────────────────────────────────────────────────────

/// Export schedule data to widget JSON format.
///
/// This function converts from the internal CRDT/field-system format to the
/// widget JSON display format, including:
/// - Credit formatting with group resolution
/// - Break synthesis (implicit breaks between panels)
/// - Presenter bidirectional group membership
pub fn export_to_widget_json(
    schedule: &Schedule,
    title: &str,
) -> Result<WidgetExport, ExportError> {
    let now = Utc::now();
    let meta = WidgetMeta {
        title: title.to_string(),
        version: 0,
        variant: "display".to_string(),
        generator: "cosam-convert 0.1.0".to_string(),
        generated: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        modified: schedule
            .metadata
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        start_time: schedule
            .metadata
            .created_at
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        end_time: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    };

    // Export panels with credit formatting and break synthesis
    let panels = export_panels(schedule)?;

    // Export rooms
    let rooms = export_rooms(schedule)?;

    // Export panel types
    let panel_types = export_panel_types(schedule)?;

    // Export timeline
    let timeline = export_timeline(schedule)?;

    // Export presenters with bidirectional group membership
    let presenters = export_presenters(schedule, &panels)?;

    Ok(WidgetExport {
        meta,
        panels,
        rooms,
        panel_types,
        timeline,
        presenters,
    })
}

/// Export panels with credit formatting and break synthesis.
fn export_panels(_schedule: &Schedule) -> Result<Vec<WidgetPanel>, ExportError> {
    // TODO: Implement panel export with credit formatting and break synthesis
    // This requires:
    // - Converting PanelInternalData to WidgetPanel
    // - Credit resolution logic from v9/v10 (hidePanelist, altPanelist, group resolution)
    // - Break synthesis (add %IB and %NB panels for time gaps)
    // For now, return empty list
    Ok(Vec::new())
}

/// Export rooms.
fn export_rooms(_schedule: &Schedule) -> Result<Vec<WidgetRoom>, ExportError> {
    // TODO: Implement room export
    // This requires converting EventRoomInternalData and HotelRoomInternalData to WidgetRoom
    Ok(Vec::new())
}

/// Export panel types.
fn export_panel_types(
    _schedule: &Schedule,
) -> Result<HashMap<String, WidgetPanelType>, ExportError> {
    // TODO: Implement panel type export
    // This requires converting PanelTypeInternalData to WidgetPanelType
    Ok(HashMap::new())
}

/// Export timeline.
fn export_timeline(_schedule: &Schedule) -> Result<Vec<WidgetTimeline>, ExportError> {
    // TODO: Implement timeline export
    // Timeline entries are panels with isTimeline: true
    Ok(Vec::new())
}

/// Export presenters with bidirectional group membership.
fn export_presenters(
    _schedule: &Schedule,
    _panels: &[WidgetPanel],
) -> Result<Vec<WidgetPresenter>, ExportError> {
    // TODO: Implement presenter export with bidirectional group membership
    // This requires:
    // - Converting PresenterInternalData to WidgetPresenter
    // - Bidirectional group membership logic (individual → group, group → individual)
    // - Building presenter-to-panel mapping for filtering
    Ok(Vec::new())
}

// ── Error Types ─────────────────────────────────────────────────────────────────

/// Errors that can occur during widget JSON export.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Failed to access entity: {0}")]
    EntityAccess(String),

    #[error("Failed to format credits: {0}")]
    CreditFormatting(String),

    #[error("Failed to synthesize breaks: {0}")]
    BreakSynthesis(String),

    #[error("Failed to resolve group membership: {0}")]
    GroupResolution(String),
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Schedule;

    #[test]
    fn test_export_creates_valid_structure() {
        let schedule = Schedule::new();
        let result = export_to_widget_json(&schedule, "Test Schedule");
        assert!(result.is_ok());
        let export = result.unwrap();
        assert_eq!(export.meta.version, 0);
        assert_eq!(export.meta.variant, "display");
    }
}
