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
use super::presenter::{Presenter, PresenterRank};
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
    pub duration: Option<u32>,
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

/// Public-facing presenter for display JSON.
///
/// Contains only the fields relevant to consumers (schedule viewers,
/// guest pages).  Internal fields like `PresenterMember`, `PresenterGroup`,
/// metadata, source info, and change state are omitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayPresenter {
    pub name: String,
    pub rank: PresenterRank,
    /// Sequential ordering key (0-based) computed from the internal
    /// `PresenterSortRank`.  Lower values sort first.
    pub sort_key: u32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_group: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub always_grouped: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub always_shown: bool,
    /// Panel IDs where this presenter/group should appear.
    /// Includes direct panel references and indirect references through group membership.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub panel_ids: Vec<String>,
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
    pub presenters: Vec<DisplayPresenter>,
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
        for ps in self.panel_sets.values() {
            for panel in &ps.panels {
                let is_timeline_panel = panel
                    .panel_type
                    .as_ref()
                    .map(|pt| timeline_type_uids.contains(pt))
                    .unwrap_or(false);
                if is_timeline_panel {
                    timeline_entries.push(super::timeline::TimelineEntry {
                        id: panel.id.clone(),
                        start_time: panel.timing.start_time(),
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
        timeline_entries.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        // Pre-compute per-PanelSet stats for title suffix logic.
        // `multi_part_bases`: base_ids that have > 1 distinct part_num.
        // `multi_session_keys`: (base_id, part_num) combos with > 1 panel.
        use std::collections::HashMap;
        let mut part_nums_per_base: HashMap<&str, std::collections::HashSet<Option<u32>>> =
            HashMap::new();
        let mut panels_per_part: HashMap<(&str, Option<u32>), usize> = HashMap::new();
        for ps in self.panel_sets.values() {
            for panel in &ps.panels {
                if panel.is_scheduled() {
                    part_nums_per_base
                        .entry(panel.base_id.as_str())
                        .or_default()
                        .insert(panel.part_num);
                    *panels_per_part
                        .entry((panel.base_id.as_str(), panel.part_num))
                        .or_insert(0) += 1;
                }
            }
        }

        let mut flat_panels: Vec<DisplayPanel> = Vec::new();

        for ps in self.panel_sets.values() {
            for panel in &ps.panels {
                if let Some(ref pt_uid) = panel.panel_type {
                    if excluded_type_uids.contains(pt_uid) {
                        continue;
                    }
                }
                if !panel.is_scheduled() {
                    continue;
                }

                let multi_part = part_nums_per_base
                    .get(panel.base_id.as_str())
                    .map_or(false, |s| s.len() > 1);
                let multi_session = panels_per_part
                    .get(&(panel.base_id.as_str(), panel.part_num))
                    .map_or(false, |&c| c > 1);

                let mut panel_name = panel.name.clone();
                if multi_part || multi_session {
                    let mut suffix_parts = Vec::new();
                    if let Some(pn) = panel.part_num.filter(|_| multi_part) {
                        suffix_parts.push(format!("Part {}", pn));
                    }
                    if let Some(sn) = panel.session_num.filter(|_| multi_session) {
                        suffix_parts.push(format!("Session {}", sn));
                    }
                    if !suffix_parts.is_empty() {
                        panel_name = format!("{} ({})", panel_name, suffix_parts.join(", "));
                    }
                }

                let credits = compute_credits(
                    panel.hide_panelist,
                    panel.alt_panelist.as_deref(),
                    &panel.credited_presenters,
                    &self.presenters,
                );

                let mut all_presenters = panel.credited_presenters.clone();
                for name in &panel.uncredited_presenters {
                    if !all_presenters.contains(name) {
                        all_presenters.push(name.clone());
                    }
                }

                flat_panels.push(DisplayPanel {
                    id: panel.id.clone(),
                    base_id: panel.base_id.clone(),
                    part_num: panel.part_num,
                    session_num: panel.session_num,
                    name: panel_name,
                    panel_type: panel.panel_type.clone(),
                    room_ids: panel.room_ids.clone(),
                    start_time: panel.timing.start_time_str(),
                    end_time: panel.timing.end_time_str(),
                    duration: panel.effective_duration_minutes(),
                    description: panel.description.clone(),
                    note: panel.note.clone(),
                    prereq: panel.prereq.clone(),
                    cost: panel.cost.clone(),
                    capacity: panel.capacity.clone(),
                    difficulty: panel.difficulty.clone(),
                    ticket_url: panel.ticket_url.clone(),
                    is_free: panel.is_free,
                    is_full: panel.is_full,
                    is_kids: panel.is_kids,
                    credits,
                    presenters: all_presenters,
                });
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
                            duration: Some(duration),
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

        // Build display presenters with bidirectional group membership and panel IDs.
        //
        // Logic:
        // 1. Start with directly referenced presenters from panels
        // 2. Add groups that contain any directly referenced presenter (individual → group)
        // 3. Add members of any directly referenced group (group → individual)
        // 4. For transitive group traversal, only follow group links (not member links)
        // 5. For each included presenter/group, collect all panel IDs where they should appear
        let presenter_lookup: std::collections::HashMap<&str, &Presenter> = self
            .presenters
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect();

        // Step 1: Find all directly referenced presenters
        let mut directly_referenced: HashSet<String> = HashSet::new();
        for dp in &flat_panels {
            for name in &dp.presenters {
                directly_referenced.insert(name.clone());
            }
        }

        // Step 2: Find all related presenters through bidirectional group membership
        let mut included_presenters: HashSet<String> = directly_referenced.clone();
        let mut to_check_groups: Vec<String> = Vec::new();
        let mut to_check_members: Vec<String> = Vec::new();

        // Initialize traversal sets
        for name in &directly_referenced {
            if let Some(presenter) = presenter_lookup.get(name.as_str()) {
                if presenter.is_group() {
                    to_check_members.push(name.clone());
                } else {
                    to_check_groups.push(name.clone());
                }
            }
        }

        // Traverse groups: individual → group → group → ... (transitive groups only)
        while let Some(presenter_name) = to_check_groups.pop() {
            if let Some(presenter) = presenter_lookup.get(presenter_name.as_str()) {
                for group_name in presenter.groups() {
                    if !included_presenters.contains(group_name) {
                        included_presenters.insert(group_name.clone());
                        to_check_groups.push(group_name.clone()); // Continue group traversal
                    }
                }
            }
        }

        // Traverse members: group → individual (direct members only, no further group traversal)
        while let Some(group_name) = to_check_members.pop() {
            if let Some(group) = presenter_lookup.get(group_name.as_str()) {
                for member_name in group.members() {
                    if !included_presenters.contains(member_name) {
                        included_presenters.insert(member_name.clone());
                        // Don't add to to_check_groups - we don't traverse groups from members
                    }
                }
            }
        }

        // Step 3: For each included presenter, collect panel IDs where they should appear
        let mut presenter_to_panels: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        // Initialize empty panel lists for all included presenters
        for name in &included_presenters {
            presenter_to_panels.insert(name.clone(), Vec::new());
        }

        // For each panel, add its ID to all presenters that should appear on it
        for dp in &flat_panels {
            let panel_id = &dp.id;

            // Start with directly referenced presenters on this panel
            let mut panel_presenters: HashSet<String> = HashSet::new();
            for name in &dp.presenters {
                panel_presenters.insert(name.clone());
            }

            // Expand bidirectional:
            // - For individuals: add their groups (transitive)
            // - For groups: add their members (direct only)
            let mut to_expand_groups: Vec<String> = Vec::new();
            let mut to_expand_members: Vec<String> = Vec::new();

            for name in &panel_presenters {
                if let Some(presenter) = presenter_lookup.get(name.as_str()) {
                    if presenter.is_group() {
                        to_expand_members.push(name.clone());
                    } else {
                        to_expand_groups.push(name.clone());
                    }
                }
            }

            // Transitive group traversal from individuals
            while let Some(presenter_name) = to_expand_groups.pop() {
                if let Some(presenter) = presenter_lookup.get(presenter_name.as_str()) {
                    for group_name in presenter.groups() {
                        if !panel_presenters.contains(group_name) {
                            panel_presenters.insert(group_name.clone());
                            to_expand_groups.push(group_name.clone()); // Continue group traversal
                        }
                    }
                }
            }

            // Direct member traversal from groups
            while let Some(group_name) = to_expand_members.pop() {
                if let Some(group) = presenter_lookup.get(group_name.as_str()) {
                    for member_name in group.members() {
                        if !panel_presenters.contains(member_name) {
                            panel_presenters.insert(member_name.clone());
                            // Don't traverse groups from members
                        }
                    }
                }
            }

            // Add this panel ID to all presenters that should appear on it
            for presenter_name in &panel_presenters {
                if let Some(panel_ids) = presenter_to_panels.get_mut(presenter_name) {
                    if !panel_ids.contains(panel_id) {
                        panel_ids.push(panel_id.clone());
                    }
                }
            }
        }

        // Step 4: Build DisplayPresenter objects
        let mut display_presenters: Vec<&Presenter> = self
            .presenters
            .iter()
            .filter(|p| included_presenters.contains(&p.name))
            .collect();
        display_presenters.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));

        let display_presenters: Vec<DisplayPresenter> = display_presenters
            .iter()
            .enumerate()
            .map(|(idx, p)| DisplayPresenter {
                name: p.name.clone(),
                rank: p.rank.clone(),
                sort_key: idx as u32,
                is_group: p.is_group(),
                members: p.members().iter().cloned().collect(),
                groups: p.groups().iter().cloned().collect(),
                always_grouped: p.always_grouped(),
                always_shown: p.always_shown(),
                panel_ids: presenter_to_panels
                    .get(&p.name)
                    .cloned()
                    .unwrap_or_default(),
            })
            .collect();

        let mut meta = self.meta.clone();
        meta.version = Some(9);
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
            presenters: display_presenters,
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
    use crate::data::panel::Panel;
    use crate::data::panel_set::PanelSet;
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
                sort_rank: None,
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
                sort_rank: None,
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
                sort_rank: None,
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
                sort_rank: None,
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
                sort_rank: None,
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
                sort_rank: None,
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
                sort_rank: None,
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
                sort_rank: None,
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
                sort_rank: None,
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

    fn make_scheduled_panel(
        id: &str,
        base_id: &str,
        part_num: Option<u32>,
        session_num: Option<u32>,
        name: &str,
    ) -> Panel {
        let mut p = Panel::new(id, base_id);
        p.name = name.to_string();
        p.part_num = part_num;
        p.session_num = session_num;
        p.room_ids = vec![1];
        p.set_start_time_from_str("2023-01-01T10:00:00");
        p.set_duration_minutes(60);
        p
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
            panel_sets: IndexMap::new(),
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

        // Test case 1: Single-part, single-session panel — no numbering
        let mut ps1 = PanelSet::new("panel1");
        ps1.panels.push(make_scheduled_panel(
            "panel1",
            "panel1",
            Some(1),
            Some(1),
            "Single Panel",
        ));
        schedule.panel_sets.insert("panel1".to_string(), ps1);

        // Test case 2: Multi-part panel — show part numbers
        let mut ps2 = PanelSet::new("panel2");
        ps2.panels.push(make_scheduled_panel(
            "panel2P1S1",
            "panel2",
            Some(1),
            Some(1),
            "Multi Part Panel",
        ));
        ps2.panels.push(make_scheduled_panel(
            "panel2P2S1",
            "panel2",
            Some(2),
            Some(1),
            "Multi Part Panel",
        ));
        schedule.panel_sets.insert("panel2".to_string(), ps2);

        // Test case 3: Single-part, multi-session panel — show session numbers
        let mut ps3 = PanelSet::new("panel3");
        ps3.panels.push(make_scheduled_panel(
            "panel3P1S1",
            "panel3",
            Some(1),
            Some(1),
            "Multi Session Panel",
        ));
        ps3.panels.push(make_scheduled_panel(
            "panel3P1S2",
            "panel3",
            Some(1),
            Some(2),
            "Multi Session Panel",
        ));
        schedule.panel_sets.insert("panel3".to_string(), ps3);

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
