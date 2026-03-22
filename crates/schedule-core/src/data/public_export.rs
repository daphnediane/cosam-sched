/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::presenter::Presenter;
use super::schedule::{Meta, Schedule};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSchedule {
    pub meta: Meta,
    pub panels: Vec<PublicPanel>,
    pub rooms: Vec<super::room::Room>,
    pub panel_types: indexmap::IndexMap<String, super::panel_type::PanelType>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
        let excluded_type_uids: HashSet<String> = self
            .panel_types
            .iter()
            .filter(|(_, pt)| pt.is_hidden || pt.is_private || pt.is_timeline)
            .map(|(prefix, _)| prefix.clone())
            .collect();

        // Build timeline entries from is_timeline panels
        let timeline_type_uids: HashSet<String> = self
            .panel_types
            .iter()
            .filter(|(_, pt)| pt.is_timeline)
            .map(|(prefix, _)| prefix.clone())
            .collect();

        let mut timeline_entries: Vec<super::timeline::TimelineEntry> = self.timeline.clone();
        for panel in self.panels.values() {
            let is_timeline_panel = panel
                .panel_type
                .as_ref()
                .map(|pt| timeline_type_uids.contains(pt))
                .unwrap_or(false);
            if is_timeline_panel {
                for part in &panel.parts {
                    for session in &part.sessions {
                        timeline_entries.push(super::timeline::TimelineEntry {
                            id: session.id.clone(),
                            start_time: session.start_time.clone().unwrap_or_default(),
                            description: panel.name.clone(),
                            panel_type: panel.panel_type.clone(),
                            note: panel.note.clone(),
                            metadata: None,
                            source: None,
                            change_state: Default::default(),
                        });
                    }
                }
            }
        }
        timeline_entries.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        let mut flat_panels: Vec<PublicPanel> = Vec::new();

        for panel in self.panels.values() {
            if let Some(ref pt_uid) = panel.panel_type {
                if excluded_type_uids.contains(pt_uid) {
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

                    let mut panel_name = panel.name.clone();

                    // Add part/session suffixes for multi-part or multi-session panels
                    // Only add part number if there are multiple parts
                    let should_show_part_num = part.part_num.is_some() && panel.parts.len() > 1;
                    // Only add session number if there are multiple sessions in this part
                    let should_show_session_num =
                        session.session_num.is_some() && part.sessions.len() > 1;

                    if should_show_part_num || should_show_session_num {
                        let mut suffix_parts = Vec::new();
                        if let Some(part_num) = part.part_num.filter(|_| should_show_part_num) {
                            suffix_parts.push(format!("Part {}", part_num));
                        }
                        if let Some(session_num) =
                            session.session_num.filter(|_| should_show_session_num)
                        {
                            suffix_parts.push(format!("Session {}", session_num));
                        }
                        if !suffix_parts.is_empty() {
                            panel_name = format!("{} ({})", panel_name, suffix_parts.join(", "));
                        }
                    }

                    flat_panels.push(PublicPanel {
                        id: session.id.clone(),
                        base_id: panel.id.clone(),
                        part_num: part.part_num,
                        session_num: session.session_num,
                        name: panel_name,
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

        let visible_panel_types: indexmap::IndexMap<String, _> = self
            .panel_types
            .iter()
            .filter(|(_, pt)| !pt.is_hidden && !pt.is_private && !pt.is_timeline)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let mut meta = self.meta.clone();
        meta.version = Some(7);
        meta.variant = Some("display".to_string());
        meta.generated = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        meta.generator = Some(format!("cosam-editor {}", env!("CARGO_PKG_VERSION")));

        // Remove private Excel metadata fields for public format
        meta.creator = None;
        meta.last_modified_by = None;
        // Keep modified field as it's public in v6

        let public = PublicSchedule {
            meta,
            panels: flat_panels,
            rooms: self.rooms.clone(),
            panel_types: visible_panel_types,
            timeline: timeline_entries,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::panel::{Panel, PanelPart, PanelSession};
    use crate::data::room::Room;
    use crate::data::schedule::{Meta, Schedule};
    use indexmap::IndexMap;

    #[test]
    fn test_part_session_numbering_in_titles() {
        let mut schedule = Schedule {
            conflicts: Vec::new(),
            meta: Meta {
                title: "Test Schedule".to_string(),
                generated: "2023-01-01T00:00:00Z".to_string(),
                version: Some(2),
                variant: None,
                generator: None,
                start_time: None,
                end_time: None,
                next_presenter_id: None,
                creator: None,
                last_modified_by: None,
                modified: None,
            },
            timeline: Vec::new(),
            panels: IndexMap::new(),
            events: Vec::new(),
            rooms: vec![Room {
                uid: 1,
                short_name: "Room 1".to_string(),
                long_name: "Room 1".to_string(),
                hotel_room: "Room 1".to_string(),
                sort_key: 1,
                is_break: false,
                metadata: None,
                source: None,
                change_state: Default::default(),
            }],
            panel_types: IndexMap::new(),
            time_types: Vec::new(),
            presenters: Vec::new(),
            imported_sheets: Default::default(),
        };

        // Test case 1: Single part, single session - no numbering
        let mut panel1 = Panel::new("panel1".to_string());
        panel1.name = "Single Panel".to_string();
        panel1.parts.push(PanelPart {
            part_num: Some(1),
            description: None,
            note: None,
            prereq: None,
            alt_panelist: None,
            credited_presenters: Vec::new(),
            uncredited_presenters: Vec::new(),
            sessions: vec![PanelSession {
                id: "session1".to_string(),
                session_num: Some(1),
                description: None,
                note: None,
                prereq: None,
                alt_panelist: None,
                room_ids: vec![1],
                start_time: None,
                end_time: None,
                duration: 60,
                is_full: false,
                capacity: None,
                seats_sold: None,
                pre_reg_max: None,
                ticket_url: None,
                simple_tix_event: None,
                hide_panelist: false,
                credited_presenters: Vec::new(),
                uncredited_presenters: Vec::new(),
                notes_non_printing: None,
                workshop_notes: None,
                power_needs: None,
                sewing_machines: false,
                av_notes: None,
                conflicts: Vec::new(),
                metadata: IndexMap::new(),
                source: None,
                change_state: Default::default(),
            }],
            change_state: Default::default(),
        });
        schedule.panels.insert("panel1".to_string(), panel1);

        // Test case 2: Multiple parts, single session each - show part numbers
        let mut panel2 = Panel::new("panel2".to_string());
        panel2.name = "Multi Part Panel".to_string();
        panel2.parts.push(PanelPart {
            part_num: Some(1),
            description: None,
            note: None,
            prereq: None,
            alt_panelist: None,
            credited_presenters: Vec::new(),
            uncredited_presenters: Vec::new(),
            sessions: vec![PanelSession {
                id: "session2a".to_string(),
                session_num: Some(1),
                description: None,
                note: None,
                prereq: None,
                alt_panelist: None,
                room_ids: vec![1],
                start_time: None,
                end_time: None,
                duration: 60,
                is_full: false,
                capacity: None,
                seats_sold: None,
                pre_reg_max: None,
                ticket_url: None,
                simple_tix_event: None,
                hide_panelist: false,
                credited_presenters: Vec::new(),
                uncredited_presenters: Vec::new(),
                notes_non_printing: None,
                workshop_notes: None,
                power_needs: None,
                sewing_machines: false,
                av_notes: None,
                conflicts: Vec::new(),
                metadata: IndexMap::new(),
                source: None,
                change_state: Default::default(),
            }],
            change_state: Default::default(),
        });
        panel2.parts.push(PanelPart {
            part_num: Some(2),
            description: None,
            note: None,
            prereq: None,
            alt_panelist: None,
            credited_presenters: Vec::new(),
            uncredited_presenters: Vec::new(),
            sessions: vec![PanelSession {
                id: "session2b".to_string(),
                session_num: Some(1),
                description: None,
                note: None,
                prereq: None,
                alt_panelist: None,
                room_ids: vec![1],
                start_time: None,
                end_time: None,
                duration: 60,
                is_full: false,
                capacity: None,
                seats_sold: None,
                pre_reg_max: None,
                ticket_url: None,
                simple_tix_event: None,
                hide_panelist: false,
                credited_presenters: Vec::new(),
                uncredited_presenters: Vec::new(),
                notes_non_printing: None,
                workshop_notes: None,
                power_needs: None,
                sewing_machines: false,
                av_notes: None,
                conflicts: Vec::new(),
                metadata: IndexMap::new(),
                source: None,
                change_state: Default::default(),
            }],
            change_state: Default::default(),
        });
        schedule.panels.insert("panel2".to_string(), panel2);

        // Test case 3: Single part, multiple sessions - show session numbers
        let mut panel3 = Panel::new("panel3".to_string());
        panel3.name = "Multi Session Panel".to_string();
        panel3.parts.push(PanelPart {
            part_num: Some(1),
            description: None,
            note: None,
            prereq: None,
            alt_panelist: None,
            credited_presenters: Vec::new(),
            uncredited_presenters: Vec::new(),
            sessions: vec![
                PanelSession {
                    id: "session3a".to_string(),
                    session_num: Some(1),
                    description: None,
                    note: None,
                    prereq: None,
                    alt_panelist: None,
                    room_ids: vec![1],
                    start_time: None,
                    end_time: None,
                    duration: 60,
                    is_full: false,
                    capacity: None,
                    seats_sold: None,
                    pre_reg_max: None,
                    ticket_url: None,
                    simple_tix_event: None,
                    hide_panelist: false,
                    credited_presenters: Vec::new(),
                    uncredited_presenters: Vec::new(),
                    notes_non_printing: None,
                    workshop_notes: None,
                    power_needs: None,
                    sewing_machines: false,
                    av_notes: None,
                    conflicts: Vec::new(),
                    metadata: IndexMap::new(),
                    source: None,
                    change_state: Default::default(),
                },
                PanelSession {
                    id: "session3b".to_string(),
                    session_num: Some(2),
                    description: None,
                    note: None,
                    prereq: None,
                    alt_panelist: None,
                    room_ids: vec![1],
                    start_time: None,
                    end_time: None,
                    duration: 60,
                    is_full: false,
                    capacity: None,
                    seats_sold: None,
                    pre_reg_max: None,
                    ticket_url: None,
                    simple_tix_event: None,
                    hide_panelist: false,
                    credited_presenters: Vec::new(),
                    uncredited_presenters: Vec::new(),
                    notes_non_printing: None,
                    workshop_notes: None,
                    power_needs: None,
                    sewing_machines: false,
                    av_notes: None,
                    conflicts: Vec::new(),
                    metadata: IndexMap::new(),
                    source: None,
                    change_state: Default::default(),
                },
            ],
            change_state: Default::default(),
        });
        schedule.panels.insert("panel3".to_string(), panel3);

        let json_result = schedule.export_public_json_string().unwrap();

        // Verify the titles by checking the JSON directly
        assert!(
            json_result.contains("\"Single Panel\""),
            "Should not add numbering for single part/session. JSON: {}",
            json_result
        );

        // Multiple parts should show part numbers
        assert!(
            json_result.contains("\"Multi Part Panel (Part 1)\""),
            "Should show part number for multi-part panel. JSON: {}",
            json_result
        );
        assert!(
            json_result.contains("\"Multi Part Panel (Part 2)\""),
            "Should show part number for multi-part panel. JSON: {}",
            json_result
        );

        // Multiple sessions should show session numbers
        assert!(
            json_result.contains("\"Multi Session Panel (Session 1)\""),
            "Should show session number for multi-session panel. JSON: {}",
            json_result
        );
        assert!(
            json_result.contains("\"Multi Session Panel (Session 2)\""),
            "Should show session number for multi-session panel. JSON: {}",
            json_result
        );

        // Ensure we don't have unwanted numbering
        assert!(
            !json_result.contains("\"Single Panel (Part 1)\""),
            "Should not add part number for single part. JSON: {}",
            json_result
        );
        assert!(
            !json_result.contains("\"Single Panel (Session 1)\""),
            "Should not add session number for single session. JSON: {}",
            json_result
        );
    }
}
