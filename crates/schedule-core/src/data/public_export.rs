/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use super::presenter::Presenter;
use super::schedule::{Meta, Schedule};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicPanel {
    pub id: String,
    pub base_id: String,
    pub part_num: Option<u32>,
    pub session_num: Option<u32>,
    pub name: String,
    pub panel_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub room_ids: Vec<u32>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub duration: u32,
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
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_free: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_full: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_kids: bool,
    pub credits: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub presenters: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSchedule {
    pub meta: Meta,
    pub panels: Vec<PublicPanel>,
    pub rooms: Vec<super::room::Room>,
    pub panel_types: Vec<super::panel_type::PanelType>,
    pub time_types: Vec<super::timeline::TimeType>,
    pub timeline: Vec<super::timeline::TimelineEntry>,
    pub presenters: Vec<Presenter>,
}

fn join_parts(parts: &[Option<&str>]) -> Option<String> {
    let joined: Vec<&str> = parts
        .iter()
        .filter_map(|p| *p)
        .filter(|s| !s.is_empty())
        .collect();
    if joined.is_empty() {
        None
    } else {
        Some(joined.join(" "))
    }
}

fn effective_alt_panelist(
    session_val: Option<&str>,
    part_val: Option<&str>,
    base_val: Option<&str>,
) -> Option<String> {
    session_val.or(part_val).or(base_val).map(|s| s.to_string())
}

fn compute_credits(
    hide_panelist: bool,
    alt_panelist: Option<&str>,
    credited_presenters: &[String],
    all_presenters: &[Presenter],
) -> Vec<String> {
    if hide_panelist {
        return Vec::new();
    }

    if let Some(alt) = alt_panelist {
        return vec![alt.to_string()];
    }

    if credited_presenters.is_empty() {
        return Vec::new();
    }

    let presenter_lookup: std::collections::HashMap<&str, &Presenter> = all_presenters
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    let mut credits: Vec<String> = Vec::new();
    let mut used_as_member: HashSet<&str> = HashSet::new();

    for name in credited_presenters {
        if used_as_member.contains(name.as_str()) {
            continue;
        }
        if let Some(presenter) = presenter_lookup.get(name.as_str()) {
            if presenter.is_group {
                credits.push(name.clone());
                for member in &presenter.members {
                    used_as_member.insert(member.as_str());
                }
            } else if presenter.always_grouped && !presenter.groups.is_empty() {
                for group_name in &presenter.groups {
                    if !credits.contains(group_name)
                        && !used_as_member.contains(group_name.as_str())
                    {
                        credits.push(group_name.clone());
                        if let Some(group) = presenter_lookup.get(group_name.as_str()) {
                            for member in &group.members {
                                used_as_member.insert(member.as_str());
                            }
                        }
                    }
                }
            } else {
                credits.push(name.clone());
            }
        } else {
            credits.push(name.clone());
        }
    }

    credits
}

impl Schedule {
    pub fn export_public_json_string(&self) -> Result<String> {
        let hidden_type_uids: HashSet<String> = self
            .panel_types
            .iter()
            .filter(|pt| pt.is_hidden)
            .map(|pt| pt.effective_uid())
            .collect();

        let mut flat_panels: Vec<PublicPanel> = Vec::new();

        for panel in self.panels.values() {
            if let Some(ref pt_uid) = panel.panel_type {
                if hidden_type_uids.contains(pt_uid) {
                    continue;
                }
            }

            for part in &panel.parts {
                for session in &part.sessions {
                    let description = join_parts(&[
                        panel.description.as_deref(),
                        part.description.as_deref(),
                        session.description.as_deref(),
                    ]);
                    let note = join_parts(&[
                        panel.note.as_deref(),
                        part.note.as_deref(),
                        session.note.as_deref(),
                    ]);
                    let prereq = join_parts(&[
                        panel.prereq.as_deref(),
                        part.prereq.as_deref(),
                        session.prereq.as_deref(),
                    ]);

                    let alt_panelist = effective_alt_panelist(
                        session.alt_panelist.as_deref(),
                        part.alt_panelist.as_deref(),
                        panel.alt_panelist.as_deref(),
                    );

                    let capacity = session
                        .capacity
                        .as_ref()
                        .or(panel.capacity.as_ref())
                        .cloned();
                    let ticket_url = session
                        .ticket_url
                        .as_ref()
                        .or(panel.ticket_url.as_ref())
                        .cloned();

                    let mut all_credited: Vec<String> = Vec::new();
                    for name in &panel.credited_presenters {
                        if !all_credited.contains(name) {
                            all_credited.push(name.clone());
                        }
                    }
                    for name in &part.credited_presenters {
                        if !all_credited.contains(name) {
                            all_credited.push(name.clone());
                        }
                    }
                    for name in &session.credited_presenters {
                        if !all_credited.contains(name) {
                            all_credited.push(name.clone());
                        }
                    }

                    let credits = compute_credits(
                        session.hide_panelist,
                        alt_panelist.as_deref(),
                        &all_credited,
                        &self.presenters,
                    );

                    let mut all_presenters: Vec<String> = Vec::new();
                    for name in &all_credited {
                        if !all_presenters.contains(name) {
                            all_presenters.push(name.clone());
                        }
                    }
                    for name in &panel.uncredited_presenters {
                        if !all_presenters.contains(name) {
                            all_presenters.push(name.clone());
                        }
                    }
                    for name in &part.uncredited_presenters {
                        if !all_presenters.contains(name) {
                            all_presenters.push(name.clone());
                        }
                    }
                    for name in &session.uncredited_presenters {
                        if !all_presenters.contains(name) {
                            all_presenters.push(name.clone());
                        }
                    }

                    flat_panels.push(PublicPanel {
                        id: session.id.clone(),
                        base_id: panel.id.clone(),
                        part_num: part.part_num,
                        session_num: session.session_num,
                        name: panel.name.clone(),
                        panel_type: panel.panel_type.clone(),
                        room_ids: session.room_ids.clone(),
                        start_time: session.start_time.clone(),
                        end_time: session.end_time.clone(),
                        duration: session.duration,
                        description,
                        note,
                        prereq,
                        cost: panel.cost.clone(),
                        capacity,
                        difficulty: panel.difficulty.clone(),
                        ticket_url,
                        is_free: panel.is_free,
                        is_full: session.is_full,
                        is_kids: panel.is_kids,
                        credits,
                        presenters: all_presenters,
                    });
                }
            }
        }

        flat_panels.sort_by(|a, b| match (&a.start_time, &b.start_time) {
            (Some(a_time), Some(b_time)) => a_time.cmp(b_time),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.id.cmp(&b.id),
        });

        let visible_panel_types: Vec<_> = self
            .panel_types
            .iter()
            .filter(|pt| !pt.is_hidden)
            .cloned()
            .collect();

        let mut meta = self.meta.clone();
        meta.version = Some(5);
        meta.variant = Some("public".to_string());
        meta.generated = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        meta.generator = Some(format!("cosam-editor {}", env!("CARGO_PKG_VERSION")));

        let public = PublicSchedule {
            meta,
            panels: flat_panels,
            rooms: self.rooms.clone(),
            panel_types: visible_panel_types,
            time_types: self.time_types.clone(),
            timeline: self.timeline.clone(),
            presenters: self.presenters.clone(),
        };

        serde_json::to_string_pretty(&public).context("Failed to serialize public schedule to JSON")
    }

    pub fn export_public(&self, path: &Path) -> Result<()> {
        let json = self.export_public_json_string()?;
        std::fs::write(path, json.as_bytes())
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }
}
