/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Widget JSON format structures.
//!
//! This module provides the widget JSON display format structures documented in
//! `docs/widget-json-format.md`.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Top-level metadata for widget JSON export.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPanel {
    pub id: String,
    pub base_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_num: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_num: Option<i32>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub room_ids: Vec<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    pub duration: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prereq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket_url: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_premium: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_full: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_kids: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credits: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presenters: Vec<String>,
}

/// Room entry in widget JSON format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetTimeline {
    pub id: String,
    pub start_time: String,
    pub description: String,
    pub panel_type: Option<String>,
    pub note: Option<String>,
}

/// Presenter entry in widget JSON format (DisplayPresenter).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPresenter {
    pub name: String,
    pub rank: String,
    pub sort_key: i32,
    pub is_group: bool,
    pub members: Vec<String>,
    pub groups: Vec<String>,
    pub panel_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub subsumes_members: bool,
}

/// Complete widget JSON export structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WidgetExport {
    pub meta: WidgetMeta,
    pub panels: Vec<WidgetPanel>,
    pub rooms: Vec<WidgetRoom>,
    pub panel_types: BTreeMap<String, WidgetPanelType>,
    pub timeline: Vec<WidgetTimeline>,
    pub presenters: Vec<WidgetPresenter>,
}

fn is_false(b: &bool) -> bool {
    !b
}
