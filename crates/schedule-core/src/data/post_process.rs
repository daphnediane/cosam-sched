/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

use std::collections::{HashMap, HashSet};

use super::event::EventConflict;
use super::panel_type::PanelType;
use super::presenter::Presenter;
use super::schedule::{ConflictEventRef, Schedule, ScheduleConflict};

const GROUP_SUFFIX_PATTERNS: [&str; 1] = ["staff"];

pub fn apply_schedule_parity(schedule: &mut Schedule) {
    resolve_session_conflicts(schedule);
    generate_credits(schedule);
    detect_conflicts(schedule);
    detect_panel_conflicts(schedule);
    schedule.update_schedule_bounds();
}

fn generate_credits(schedule: &mut Schedule) {
    let _presenter_lookup: HashMap<&str, &Presenter> = schedule
        .presenters
        .iter()
        .map(|presenter| (presenter.name.as_str(), presenter))
        .collect();

    // Generate credits for panels (though panels don't have credits field in current structure)
    // This function might need to be rethought for the panel-based structure
    // For now, we'll leave it as a no-op since credits are handled differently in panels
}

fn detect_conflicts(_schedule: &mut Schedule) {
    // This function was for event-based conflicts
    // Panel conflicts are handled in detect_panel_conflicts
    // For now, leave this empty
}

/// Detect conflicts in panel sessions (room and presenter overlaps)
fn detect_panel_conflicts(schedule: &mut Schedule) {
    let panel_type_lookup: HashMap<String, &PanelType> = schedule
        .panel_types
        .iter()
        .map(|(prefix, panel_type)| (prefix.clone(), panel_type))
        .collect();

    let mut panel_sessions: Vec<(
        String,
        String,
        chrono::NaiveDateTime,
        chrono::NaiveDateTime,
        Vec<u32>,
        Vec<String>,
    )> = Vec::new();
    let mut session_index_map: HashMap<String, usize> = HashMap::new();

    for ps in schedule.panel_sets.values() {
        for panel in &ps.panels {
            if let Some(ref pt_uid) = panel.panel_type {
                if is_break_event(Some(pt_uid), &panel_type_lookup) {
                    continue;
                }
            }

            if let (Some(start_time), Some(end_time)) =
                (panel.timing.start_time(), panel.effective_end_time())
            {
                let all_presenters: Vec<String> = panel
                    .credited_presenters
                    .iter()
                    .chain(panel.uncredited_presenters.iter())
                    .cloned()
                    .collect();

                panel_sessions.push((
                    panel.id.clone(),
                    panel.name.clone(),
                    start_time,
                    end_time,
                    panel.room_ids.clone(),
                    all_presenters,
                ));
                session_index_map.insert(panel.id.clone(), panel_sessions.len() - 1);
            }
        }
    }

    // Detect room conflicts
    let mut room_sessions: HashMap<u32, Vec<usize>> = HashMap::new();
    for (session_idx, (_, _, _, _, room_ids, _)) in panel_sessions.iter().enumerate() {
        for &room_id in room_ids {
            room_sessions.entry(room_id).or_default().push(session_idx);
        }
    }

    for (room_id, session_indexes) in room_sessions {
        if session_indexes.len() < 2 {
            continue;
        }

        let mut sorted_indexes = session_indexes;
        sorted_indexes.sort_by_key(|idx| panel_sessions[*idx].2); // sort by start time

        let overlap_groups = find_session_overlap_groups(&sorted_indexes, &panel_sessions);

        for overlap_group in overlap_groups {
            if overlap_group.len() < 2 {
                continue;
            }

            for first_pos in 0..(overlap_group.len() - 1) {
                for second_pos in (first_pos + 1)..overlap_group.len() {
                    let first_idx = overlap_group[first_pos];
                    let second_idx = overlap_group[second_pos];

                    add_panel_session_conflict(
                        schedule,
                        &panel_sessions[first_idx],
                        &panel_sessions[second_idx],
                        "room",
                        None,
                        Some(serde_json::json!(room_id)),
                    );
                }
            }
        }
    }

    // Detect presenter conflicts
    let mut presenter_sessions: HashMap<String, Vec<usize>> = HashMap::new();
    for (session_idx, (_, _, _, _, _, presenters)) in panel_sessions.iter().enumerate() {
        for presenter in presenters {
            presenter_sessions
                .entry(presenter.clone())
                .or_default()
                .push(session_idx);
        }
    }

    for (presenter_name, session_indexes) in presenter_sessions {
        if session_indexes.len() < 2 {
            continue;
        }

        let mut sorted_indexes = session_indexes;
        sorted_indexes.sort_by_key(|idx| panel_sessions[*idx].2); // sort by start time

        let overlap_groups = find_session_overlap_groups(&sorted_indexes, &panel_sessions);
        let group_presenter = is_group_presenter(&presenter_name, schedule);
        let conflict_type = if group_presenter {
            "group_presenter"
        } else {
            "presenter"
        };

        for overlap_group in overlap_groups {
            if overlap_group.len() < 2 {
                continue;
            }

            for first_pos in 0..(overlap_group.len() - 1) {
                for second_pos in (first_pos + 1)..overlap_group.len() {
                    let first_idx = overlap_group[first_pos];
                    let second_idx = overlap_group[second_pos];

                    add_panel_session_conflict(
                        schedule,
                        &panel_sessions[first_idx],
                        &panel_sessions[second_idx],
                        conflict_type,
                        Some(presenter_name.clone()),
                        None,
                    );
                }
            }
        }
    }
}

/// Find overlapping groups among panel sessions
fn find_session_overlap_groups(
    session_indexes: &[usize],
    sessions: &[(
        String,
        String,
        chrono::NaiveDateTime,
        chrono::NaiveDateTime,
        Vec<u32>,
        Vec<String>,
    )],
) -> Vec<Vec<usize>> {
    let mut overlap_groups: Vec<Vec<usize>> = Vec::new();
    let Some(first_index) = session_indexes.first().copied() else {
        return overlap_groups;
    };

    let mut current_group = vec![first_index];
    let mut current_end = sessions[first_index].3; // end_time

    for &session_index in session_indexes.iter().skip(1) {
        let session_start = sessions[session_index].2; // start_time
        if session_start < current_end {
            current_group.push(session_index);
            let session_end = sessions[session_index].3; // end_time
            if session_end > current_end {
                current_end = session_end;
            }
            continue;
        }

        overlap_groups.push(current_group);
        current_group = vec![session_index];
        current_end = sessions[session_index].3;
    }

    overlap_groups.push(current_group);
    overlap_groups
}

/// Add a conflict between two panel sessions
fn add_panel_session_conflict(
    schedule: &mut Schedule,
    first_session: &(
        String,
        String,
        chrono::NaiveDateTime,
        chrono::NaiveDateTime,
        Vec<u32>,
        Vec<String>,
    ),
    second_session: &(
        String,
        String,
        chrono::NaiveDateTime,
        chrono::NaiveDateTime,
        Vec<u32>,
        Vec<String>,
    ),
    conflict_type: &str,
    presenter_name: Option<String>,
    room_value: Option<serde_json::Value>,
) {
    schedule.conflicts.push(ScheduleConflict {
        event1: ConflictEventRef {
            id: first_session.0.clone(),
            name: first_session.1.clone(),
        },
        event2: ConflictEventRef {
            id: second_session.0.clone(),
            name: second_session.1.clone(),
        },
        presenter: presenter_name.clone(),
        room: room_value.clone(),
        conflict_type: conflict_type.to_string(),
    });

    // Helper closure: find a flat panel by id across all panel_sets
    let get_panel_id = |panels: &mut indexmap::IndexMap<String, super::panel_set::PanelSet>,
                        id: &str|
     -> Option<(String, usize)> {
        for (base_id, ps) in panels.iter() {
            if let Some(idx) = ps.panels.iter().position(|p| p.id == id) {
                return Some((base_id.clone(), idx));
            }
        }
        None
    };

    let make_details = |conflict_type: &str,
                        presenter_name: &Option<String>,
                        other_name: &str|
     -> Option<String> {
        match conflict_type {
            "group_presenter" => presenter_name
                .as_ref()
                .map(|name| format!("Group presenter overlap: {name} in multiple events")),
            "presenter" => presenter_name
                .as_ref()
                .map(|name| format!("Double-booked with: {} (presenter: {name})", other_name)),
            _ => Some(format!("Room conflict with: {}", other_name)),
        }
    };

    if let Some((base_id, idx)) = get_panel_id(&mut schedule.panel_sets, &first_session.0) {
        let details = make_details(conflict_type, &presenter_name, &second_session.1);
        schedule.panel_sets.get_mut(&base_id).unwrap().panels[idx]
            .conflicts
            .push(EventConflict {
                conflict_type: conflict_type.to_string(),
                details,
                conflict_event_id: Some(second_session.0.clone()),
            });
    }

    if let Some((base_id, idx)) = get_panel_id(&mut schedule.panel_sets, &second_session.0) {
        let details = make_details(conflict_type, &presenter_name, &first_session.1);
        schedule.panel_sets.get_mut(&base_id).unwrap().panels[idx]
            .conflicts
            .push(EventConflict {
                conflict_type: conflict_type.to_string(),
                details,
                conflict_event_id: Some(first_session.0.clone()),
            });
    }
}

fn is_break_event(
    panel_type_uid: Option<&str>,
    panel_type_lookup: &HashMap<String, &PanelType>,
) -> bool {
    let Some(uid) = panel_type_uid else {
        return false;
    };

    panel_type_lookup
        .get(uid)
        .map(|panel_type| panel_type.is_break)
        .unwrap_or(false)
}

fn is_group_presenter(presenter_name: &str, schedule: &Schedule) -> bool {
    if presenter_name.ends_with('=') {
        return true;
    }

    let presenter_lower = presenter_name.to_lowercase();
    if GROUP_SUFFIX_PATTERNS
        .iter()
        .any(|pattern| presenter_lower.ends_with(pattern))
    {
        return true;
    }

    schedule.relationships.is_group(presenter_name)
}

/// Resolve panel ID conflicts within each [`PanelSet`] by assigning unique
/// alpha suffixes.  Two flat panels in the same set that share the same `id`
/// AND `session_num` are considered duplicates; panels with the same `id` but
/// different `session_num` values are left as-is.
///
/// Alpha suffixes use A..Z, AA..AZ, … skipping P and S.
pub(crate) fn resolve_session_conflicts(schedule: &mut Schedule) {
    for ps in schedule.panel_sets.values_mut() {
        resolve_panelset_conflicts(ps);
    }
}

/// Resolve ID conflicts within a single [`PanelSet`].
fn resolve_panelset_conflicts(ps: &mut super::panel_set::PanelSet) {
    if ps.panels.len() <= 1 {
        return;
    }

    // Group panel indices by (id, session_num)
    let mut groups: HashMap<(String, Option<u32>), Vec<usize>> = HashMap::new();
    for (idx, panel) in ps.panels.iter().enumerate() {
        let suffix = extract_alpha_suffix(&panel.id);
        let key = (strip_alpha_suffix(&panel.id) + &suffix, panel.session_num);
        groups.entry(key).or_default().push(idx);
    }

    // Collect conflicting groups (same id AND session_num, > 1 panel)
    let conflicts: Vec<_> = groups
        .iter()
        .filter(|(_, indices)| indices.len() > 1)
        .map(|(key, indices)| (key.clone(), indices.clone()))
        .collect();

    if conflicts.is_empty() {
        return;
    }

    for ((conflicted_id, session_num), indices) in conflicts {
        let base_without_suffix = strip_alpha_suffix(&conflicted_id);
        let original_suffix = extract_alpha_suffix(&conflicted_id);

        // Collect all used (session_num, suffix) pairs in this PanelSet
        let mut used: HashSet<(Option<u32>, String)> = HashSet::new();
        for panel in &ps.panels {
            if panel.id.starts_with(&base_without_suffix) {
                used.insert((panel.session_num, extract_alpha_suffix(&panel.id)));
            }
        }

        let mut start_suffix = original_suffix;
        for &panel_idx in &indices {
            let new_suffix = generate_unique_suffix_for_session(session_num, &start_suffix, &used);
            let new_id = format!("{}{}", base_without_suffix, new_suffix);
            ps.panels[panel_idx].id = new_id;
            ps.panels[panel_idx].session_num = session_num;
            used.insert((session_num, new_suffix.clone()));
            start_suffix = new_suffix;
        }
    }
}

/// Generate a unique suffix for a specific session number, avoiding conflicts
fn generate_unique_suffix_for_session(
    session_num: Option<u32>,
    start_suffix: &str,
    used: &HashSet<(Option<u32>, String)>,
) -> String {
    // If start_suffix is not used for this session_num, return it
    if !used.contains(&(session_num, start_suffix.to_string())) {
        return start_suffix.to_string();
    }

    // Start with "A" if starting from empty, otherwise start from start_suffix
    let mut suffix = if start_suffix.is_empty() {
        "A".to_string()
    } else {
        start_suffix.to_string()
    };

    // If we're starting from a non-empty suffix, we need to advance it first
    if !start_suffix.is_empty() {
        suffix = next_alpha_suffix(&suffix);
    }

    loop {
        // Check if this (session_num, suffix) combination is available
        if !used.contains(&(session_num, suffix.clone())) {
            return suffix;
        }

        // Generate next suffix in sequence
        suffix = next_alpha_suffix(&suffix);
    }
}

/// Extract the alpha suffix from an ID (non-P/S part at end)
fn extract_alpha_suffix(id: &str) -> String {
    // Find the last occurrence of non-P/S alphabetic characters at the end
    let mut suffix_start = id.len();
    for (i, ch) in id.chars().rev().enumerate() {
        if ch.is_ascii_alphabetic() && ch != 'P' && ch != 'S' {
            suffix_start = id.len() - i - 1;
        } else {
            break;
        }
    }

    if suffix_start < id.len() {
        id[suffix_start..].to_string()
    } else {
        String::new() // No alpha suffix
    }
}

/// Strip the alpha suffix from an ID
fn strip_alpha_suffix(id: &str) -> String {
    let suffix = extract_alpha_suffix(id);
    if suffix.is_empty() {
        id.to_string()
    } else {
        id[..id.len() - suffix.len()].to_string()
    }
}

/// Generate the next alpha suffix in sequence
/// A..Z, AA..AZ, BA..BZ, ..., AAA..AAZ, ABA..ABZ, etc.
fn next_alpha_suffix(current: &str) -> String {
    if current.is_empty() {
        return "A".to_string();
    }

    let chars: Vec<char> = current.chars().collect();
    let mut result = chars;
    let mut i = result.len() - 1;

    loop {
        if result[i] < 'Z' {
            result[i] = (result[i] as u8 + 1) as char;
            // Skip P and S
            if result[i] == 'P' {
                result[i] = 'Q';
            } else if result[i] == 'S' {
                result[i] = 'T';
            }
            break;
        } else {
            // Wrap this character to A and carry to the next position
            result[i] = 'A';
            if i == 0 {
                // Need to add a new character at the front
                result.insert(0, 'A');
                break;
            } else {
                i -= 1;
            }
        }
    }

    result.iter().collect()
}

#[cfg(test)]
include!("post_process_tests.rs");
