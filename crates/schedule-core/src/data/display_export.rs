/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};

use super::panel_type::PanelType;
use super::presenter::Presenter;
use super::schedule::{Meta, Schedule};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayPanel {
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
pub struct DisplaySchedule {
    pub meta: Meta,
    pub panels: Vec<DisplayPanel>,
    pub rooms: Vec<super::room::Room>,
    pub panel_types: indexmap::IndexMap<String, super::panel_type::PanelType>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub timeline: Vec<super::timeline::TimelineEntry>,
    pub presenters: Vec<Presenter>,
}

fn parse_local_datetime(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
}

fn is_overnight_break(gap_start: &NaiveDateTime, gap_end: &NaiveDateTime) -> bool {
    // Overnight if on different dates, or if the gap crosses 4 AM
    if gap_start.date() != gap_end.date() {
        return true;
    }
    let start_hour = gap_start.hour();
    let end_hour = gap_end.hour();
    start_hour < 4 && end_hour >= 4
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
    let mut used_groups: HashSet<String> = HashSet::new();

    // First pass: handle groups and always_grouped members
    for name in credited_presenters {
        if used_as_member.contains(name.as_str()) {
            continue;
        }
        if let Some(presenter) = presenter_lookup.get(name.as_str()) {
            if presenter.is_group() {
                // For always_shown groups, check if we should show "Member of Group" format
                if presenter.always_shown() {
                    let unique_members: std::collections::HashSet<_> =
                        presenter.members().iter().collect();
                    let credited_members_in_group: Vec<&String> = unique_members
                        .iter()
                        .filter(|member| credited_presenters.contains(member))
                        .copied()
                        .collect();

                    if !used_groups.contains(name) {
                        // If not all members are present (partial attendance), check format
                        if credited_members_in_group.len() < unique_members.len() {
                            if credited_members_in_group.is_empty() {
                                // No members credited: show just the group name
                                credits.push(name.clone());
                            } else if credited_members_in_group.len() == 1 {
                                // Single member: "Member of Group"
                                credits
                                    .push(format!("{} of {}", credited_members_in_group[0], name));
                            } else {
                                // Multiple members: "Group (Member1, Member2, ...)"
                                let member_names: Vec<String> = credited_members_in_group
                                    .iter()
                                    .map(|s| s.to_string())
                                    .collect();
                                credits.push(format!("{} ({})", name, member_names.join(", ")));
                            }
                            for member in &credited_members_in_group {
                                used_as_member.insert(member.as_str());
                            }
                            used_groups.insert(name.clone());
                        } else {
                            // All members present, show just the group name
                            credits.push(name.clone());
                            used_groups.insert(name.clone());
                            for member in presenter.members() {
                                used_as_member.insert(member.as_str());
                            }
                        }
                    }
                } else {
                    // Regular group logic
                    let show_as_group = presenter
                        .members()
                        .iter()
                        .all(|member| credited_presenters.contains(member));

                    if show_as_group {
                        if !used_groups.contains(name) {
                            credits.push(name.clone());
                            used_groups.insert(name.clone());
                            for member in presenter.members() {
                                used_as_member.insert(member.as_str());
                            }
                        }
                    } else {
                        // Group is not always_shown and not all members present, show members individually
                        for member in presenter.members() {
                            if credited_presenters.contains(member)
                                && !used_as_member.contains(member.as_str())
                            {
                                credits.push(member.clone());
                                used_as_member.insert(member.as_str());
                            }
                        }
                    }
                }
            } else if presenter.always_grouped() && !presenter.groups().is_empty() {
                // This member should always appear under their group name
                for group_name in presenter.groups() {
                    if let Some(group) = presenter_lookup.get(group_name.as_str()) {
                        // Check if this group should be shown and handle partial membership
                        let show_as_group = group.always_shown()
                            || group
                                .members()
                                .iter()
                                .all(|member| credited_presenters.contains(member));

                        if !used_groups.contains(group_name) {
                            if show_as_group {
                                // For always_shown groups, check if we should show "Member of Group" format
                                if group.always_shown() {
                                    let credited_members_in_group: Vec<&String> = group
                                        .members()
                                        .iter()
                                        .filter(|member| credited_presenters.contains(member))
                                        .collect();

                                    // If not all members are present (partial attendance), show "Member of Group" for each
                                    if credited_members_in_group.len() < group.members().len() {
                                        // Show each credited member as "Member of Group"
                                        for member in &credited_members_in_group {
                                            credits.push(format!("{} of {}", member, group_name));
                                            used_as_member.insert(member.as_str());
                                        }
                                        used_groups.insert(group_name.clone());
                                    } else {
                                        // All members present, show just the group name
                                        credits.push(group_name.clone());
                                        used_groups.insert(group_name.clone());
                                        for member in group.members() {
                                            used_as_member.insert(member.as_str());
                                        }
                                    }
                                } else {
                                    // Regular group, show just the group name
                                    credits.push(group_name.clone());
                                    used_groups.insert(group_name.clone());
                                    for member in group.members() {
                                        used_as_member.insert(member.as_str());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Second pass: handle remaining individual presenters and check for implicit groups
    for name in credited_presenters {
        if used_as_member.contains(name.as_str()) {
            continue;
        }
        if let Some(presenter) = presenter_lookup.get(name.as_str()) {
            if !presenter.is_group() && !presenter.always_grouped() {
                // Check if this presenter belongs to any groups that should be shown
                let mut group_shown = false;
                for group_name in presenter.groups() {
                    if !used_groups.contains(group_name) {
                        if let Some(group) = presenter_lookup.get(group_name.as_str()) {
                            let show_as_group = group.always_shown()
                                || group
                                    .members()
                                    .iter()
                                    .all(|member| credited_presenters.contains(member));

                            if show_as_group {
                                // For always_shown groups, check if we should show "Member of Group" format
                                if group.always_shown() {
                                    let credited_members_in_group: Vec<&String> = group
                                        .members()
                                        .iter()
                                        .filter(|member| credited_presenters.contains(member))
                                        .collect();

                                    // If not all members are present (partial attendance), show "Member of Group" for each
                                    if credited_members_in_group.len() < group.members().len() {
                                        // Show each credited member as "Member of Group"
                                        for member in &credited_members_in_group {
                                            credits.push(format!("{} of {}", member, group_name));
                                            used_as_member.insert(member.as_str());
                                        }
                                        used_groups.insert(group_name.clone());
                                    } else {
                                        // All members present, show just the group name
                                        credits.push(group_name.clone());
                                        used_groups.insert(group_name.clone());
                                        for member in group.members() {
                                            used_as_member.insert(member.as_str());
                                        }
                                    }
                                } else {
                                    // Regular group, show just the group name
                                    credits.push(group_name.clone());
                                    used_groups.insert(group_name.clone());
                                    for member in group.members() {
                                        used_as_member.insert(member.as_str());
                                    }
                                }
                                group_shown = true;
                                break;
                            }
                        }
                    }
                }

                if !group_shown {
                    // Show as individual presenter
                    credits.push(name.clone());
                }
            }
        } else {
            // Presenter not found in lookup, show as-is
            credits.push(name.clone());
        }
    }

    credits
}

impl Schedule {
    pub fn export_display_json_string(&self) -> Result<String> {
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

        let mut flat_panels: Vec<DisplayPanel> = Vec::new();

        for panel in self.panels.values() {
            if let Some(ref pt_uid) = panel.panel_type {
                if excluded_type_uids.contains(pt_uid) {
                    continue;
                }
            }

            for part in &panel.parts {
                for session in &part.sessions {
                    // Skip unscheduled panels from display export
                    if !session.is_scheduled() {
                        continue;
                    }
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

                    flat_panels.push(DisplayPanel {
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

        // Generate baked-in implicit breaks (%IB / %NB)
        let visible_room_ids: Vec<u32> = self
            .rooms
            .iter()
            .filter(|r| !r.is_break)
            .map(|r| r.uid)
            .collect();

        let break_type_uids: HashSet<&String> = self
            .panel_types
            .iter()
            .filter(|(_, pt)| pt.is_break)
            .map(|(prefix, _)| prefix)
            .collect();

        // Collect scheduled non-break panels for gap detection
        let scheduled: Vec<&DisplayPanel> = flat_panels
            .iter()
            .filter(|p| {
                p.start_time.is_some()
                    && p.end_time.is_some()
                    && !p
                        .panel_type
                        .as_ref()
                        .is_some_and(|pt| break_type_uids.contains(pt))
            })
            .collect();

        let mut implicit_breaks: Vec<DisplayPanel> = Vec::new();
        let mut has_ib = false;
        let mut has_nb = false;

        if scheduled.len() > 1 {
            let mut latest_end = parse_local_datetime(scheduled[0].end_time.as_ref().unwrap());
            let mut latest_end_str = scheduled[0].end_time.clone();

            for panel in &scheduled[1..] {
                let start_str = panel.start_time.as_ref().unwrap();
                if let (Some(latest), Some(next_start)) =
                    (latest_end, parse_local_datetime(start_str))
                {
                    let gap_minutes = (next_start - latest).num_minutes();
                    // Match widget heuristic: gaps > 3 hours become implicit breaks
                    if gap_minutes > 180 {
                        let is_overnight = is_overnight_break(&latest, &next_start);
                        let prefix = if is_overnight { "%NB" } else { "%IB" };
                        let id = format!("{}{:03}", prefix, implicit_breaks.len() + 1);
                        let duration = gap_minutes.max(0) as u32;

                        if is_overnight {
                            has_nb = true;
                        } else {
                            has_ib = true;
                        }

                        implicit_breaks.push(DisplayPanel {
                            id: id.clone(),
                            base_id: id,
                            part_num: None,
                            session_num: None,
                            name: if is_overnight {
                                "Overnight Break".to_string()
                            } else {
                                "Break".to_string()
                            },
                            panel_type: Some(prefix.to_string()),
                            room_ids: visible_room_ids.clone(),
                            start_time: latest_end_str.clone(),
                            end_time: Some(start_str.clone()),
                            duration,
                            description: None,
                            note: None,
                            prereq: None,
                            cost: None,
                            capacity: None,
                            difficulty: None,
                            ticket_url: None,
                            is_free: true,
                            is_full: false,
                            is_kids: false,
                            credits: Vec::new(),
                            presenters: Vec::new(),
                        });
                    }
                }

                // Update latest end time if this panel ends later
                if let Some(next_end) = parse_local_datetime(panel.end_time.as_ref().unwrap()) {
                    if latest_end.map_or(true, |le| next_end > le) {
                        latest_end = Some(next_end);
                        latest_end_str = panel.end_time.clone();
                    }
                }
            }
        }

        // Merge implicit breaks and re-sort
        if !implicit_breaks.is_empty() {
            flat_panels.extend(implicit_breaks);
            flat_panels.sort_by(|a, b| match (&a.start_time, &b.start_time) {
                (Some(a_time), Some(b_time)) => a_time.cmp(b_time),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.id.cmp(&b.id),
            });
        }

        let mut visible_panel_types: indexmap::IndexMap<String, _> = self
            .panel_types
            .iter()
            .filter(|(_, pt)| !pt.is_hidden && !pt.is_private && !pt.is_timeline)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Add synthetic panel types for baked-in breaks
        if has_ib {
            visible_panel_types.insert(
                "%IB".to_string(),
                PanelType {
                    prefix: "%IB".to_string(),
                    kind: "Implicit Break".to_string(),
                    colors: indexmap::indexmap! { "color".to_string() => "#F5F5F5".to_string() },
                    is_break: true,
                    is_cafe: false,
                    is_workshop: false,
                    is_hidden: false,
                    is_room_hours: false,
                    is_timeline: false,
                    is_private: false,
                    metadata: None,
                    source: None,
                    change_state: Default::default(),
                },
            );
        }
        if has_nb {
            visible_panel_types.insert(
                "%NB".to_string(),
                PanelType {
                    prefix: "%NB".to_string(),
                    kind: "Overnight Break".to_string(),
                    colors: indexmap::indexmap! { "color".to_string() => "#F5F5F5".to_string() },
                    is_break: true,
                    is_cafe: false,
                    is_workshop: false,
                    is_hidden: false,
                    is_room_hours: false,
                    is_timeline: false,
                    is_private: false,
                    metadata: None,
                    source: None,
                    change_state: Default::default(),
                },
            );
        }

        let mut meta = self.meta.clone();
        meta.version = Some(7);
        meta.variant = Some("display".to_string());
        meta.generated = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        meta.generator = Some(format!("cosam-editor {}", env!("CARGO_PKG_VERSION")));

        // Remove private Excel metadata fields for public format
        meta.creator = None;
        meta.last_modified_by = None;
        // Keep modified field as it's public in v6

        let display = DisplaySchedule {
            meta,
            panels: flat_panels,
            rooms: self.rooms.clone(),
            panel_types: visible_panel_types,
            timeline: timeline_entries,
            presenters: self.presenters.clone(),
        };

        serde_json::to_string_pretty(&display)
            .context("Failed to serialize display schedule to JSON")
    }

    pub fn export_display(&self, path: &Path) -> Result<()> {
        let json = self.export_display_json_string()?;
        std::fs::write(path, json.as_bytes())
            .with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::data::panel::{Panel, PanelPart, PanelSession};
    use crate::data::presenter::{Presenter, PresenterGroup, PresenterMember, PresenterRank};
    use crate::data::room::Room;
    use crate::data::schedule::{Meta, Schedule};
    use indexmap::IndexMap;

    #[test]
    fn test_compute_credits_enhanced_logic() {
        let presenters = vec![
            // Regular presenter
            Presenter {
                id: None,
                name: "John Doe".to_string(),
                rank: PresenterRank::from_str("fan_panelist"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            // Always grouped member
            Presenter {
                id: None,
                name: "Jane Smith".to_string(),
                rank: PresenterRank::from_str("fan_panelist"),
                is_member: PresenterMember::IsMember(
                    {
                        let mut groups = std::collections::BTreeSet::new();
                        groups.insert("Test Group".to_string());
                        groups
                    },
                    true,
                ),
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            // Regular group member
            Presenter {
                id: None,
                name: "Bob Johnson".to_string(),
                rank: PresenterRank::from_str("fan_panelist"),
                is_member: PresenterMember::IsMember(
                    {
                        let mut groups = std::collections::BTreeSet::new();
                        groups.insert("Test Group".to_string());
                        groups
                    },
                    false,
                ),
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            // Always shown group
            Presenter {
                id: None,
                name: "Test Group".to_string(),
                rank: PresenterRank::from_str("guest"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::IsGroup(
                    {
                        let mut members = std::collections::BTreeSet::new();
                        members.insert("Jane Smith".to_string());
                        members.insert("Bob Johnson".to_string());
                        members
                    },
                    true,
                ),
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            // Regular group
            Presenter {
                id: None,
                name: "Regular Group".to_string(),
                rank: PresenterRank::from_str("guest"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::IsGroup(
                    {
                        let mut members = std::collections::BTreeSet::new();
                        members.insert("Alice Brown".to_string());
                        members.insert("Charlie Wilson".to_string());
                        members
                    },
                    false,
                ),
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            // Regular group members
            Presenter {
                id: None,
                name: "Alice Brown".to_string(),
                rank: PresenterRank::from_str("fan_panelist"),
                is_member: PresenterMember::IsMember(
                    {
                        let mut groups = std::collections::BTreeSet::new();
                        groups.insert("Regular Group".to_string());
                        groups
                    },
                    false,
                ),
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            Presenter {
                id: None,
                name: "Charlie Wilson".to_string(),
                rank: PresenterRank::from_str("fan_panelist"),
                is_member: PresenterMember::IsMember(
                    {
                        let mut groups = std::collections::BTreeSet::new();
                        groups.insert("Regular Group".to_string());
                        groups
                    },
                    false,
                ),
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
        ];

        // Test 1: Always shown group with partial membership should show "Member of Group" format
        let credits = super::compute_credits(
            false,
            None,
            &["Jane Smith".to_string()], // Only one member of always_shown group
            &presenters,
        );
        assert_eq!(credits, vec!["Jane Smith of Test Group"]);

        // Test 2: Regular group with all members present should show group name
        let credits = super::compute_credits(
            false,
            None,
            &["Alice Brown".to_string(), "Charlie Wilson".to_string()],
            &presenters,
        );
        assert_eq!(credits, vec!["Regular Group"]);

        // Test 3: Regular group with partial membership should show individual names
        let credits = super::compute_credits(
            false,
            None,
            &["Alice Brown".to_string()], // Only one member of regular group
            &presenters,
        );
        assert_eq!(credits, vec!["Alice Brown"]);

        // Test 4: Always grouped member should show "Member of Group" format when group is always_shown
        let credits = super::compute_credits(false, None, &["Jane Smith".to_string()], &presenters);
        assert_eq!(credits, vec!["Jane Smith of Test Group"]);

        // Test 5: Regular presenter should show individual name
        let credits = super::compute_credits(false, None, &["John Doe".to_string()], &presenters);
        assert_eq!(credits, vec!["John Doe"]);

        // Test 6: Mixed scenario - always shown group, regular group, and individual
        let credits = super::compute_credits(
            false,
            None,
            &[
                "John Doe".to_string(),
                "Jane Smith".to_string(),
                "Alice Brown".to_string(),
            ],
            &presenters,
        );
        // Should show: Jane Smith of Test Group (always_shown partial), John Doe (individual), Alice Brown (partial regular group)
        assert_eq!(
            credits,
            vec!["Jane Smith of Test Group", "John Doe", "Alice Brown"]
        );
    }

    #[test]
    fn test_compute_credits_hide_alt_panelist() {
        let presenters = vec![
            Presenter {
                id: None,
                name: "John Doe".to_string(),
                rank: PresenterRank::from_str("fan_panelist"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
            Presenter {
                id: None,
                name: "Test Group".to_string(),
                rank: PresenterRank::from_str("guest"),
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::IsGroup(
                    {
                        let mut members = std::collections::BTreeSet::new();
                        members.insert("John Doe".to_string());
                        members.insert("Jane Doe".to_string());
                        members
                    },
                    false,
                ),
                metadata: None,
                source: None,
                change_state: Default::default(),
            },
        ];

        // Test normal case
        let credits = super::compute_credits(false, None, &["John Doe".to_string()], &presenters);
        assert_eq!(credits, vec!["John Doe"]);

        // Test hide_panelist
        let credits = super::compute_credits(true, None, &["John Doe".to_string()], &presenters);
        assert_eq!(credits, Vec::<String>::new());

        // Test alt_panelist
        let credits = super::compute_credits(
            false,
            Some("Mystery Guest"),
            &["John Doe".to_string()],
            &presenters,
        );
        assert_eq!(credits, vec!["Mystery Guest"]);

        // Test precedence: hide_panelist overrides alt_panelist
        let credits = super::compute_credits(
            true,
            Some("Mystery Guest"),
            &["John Doe".to_string()],
            &presenters,
        );
        assert_eq!(credits, Vec::<String>::new());
    }

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
                start_time: Some("2023-01-01T10:00:00".to_string()),
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
                start_time: Some("2023-01-01T10:00:00".to_string()),
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
                start_time: Some("2023-01-01T10:00:00".to_string()),
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
                    start_time: Some("2023-01-01T10:00:00".to_string()),
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
                    start_time: Some("2023-01-01T10:00:00".to_string()),
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

        let json_result = schedule.export_display_json_string().unwrap();

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
