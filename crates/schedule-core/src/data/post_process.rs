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
    normalize_event_times(schedule);
    resolve_session_conflicts(schedule);
    generate_credits(schedule);
    detect_conflicts(schedule);
    detect_panel_conflicts(schedule);
    schedule.calculate_schedule_bounds();
}

fn normalize_event_times(schedule: &mut Schedule) {
    // Normalize times for all sessions in panels
    for panel in schedule.panels.values_mut() {
        for part in &mut panel.parts {
            for session in &mut part.sessions {
                if let (Some(start), Some(end)) = (&session.start_time, &session.end_time) {
                    // Parse the time strings
                    if let (Ok(start_dt), Ok(end_dt)) = (
                        chrono::NaiveDateTime::parse_from_str(start, "%-m/%-d/%Y %-I:%M %p"),
                        chrono::NaiveDateTime::parse_from_str(end, "%-m/%-d/%Y %-I:%M %p"),
                    ) {
                        if end_dt < start_dt {
                            let duration_minutes = if session.duration == 0 {
                                60
                            } else {
                                session.duration
                            };
                            let new_end =
                                start_dt + chrono::Duration::minutes(duration_minutes as i64);
                            session.end_time =
                                Some(new_end.format("%-m/%-d/%Y %-I:%M %p").to_string());
                            session.duration = duration_minutes;
                        } else {
                            let computed_minutes = (end_dt - start_dt).num_minutes();
                            if computed_minutes > 0 {
                                session.duration = computed_minutes as u32;
                            } else if session.duration > 0 {
                                let new_end =
                                    start_dt + chrono::Duration::minutes(session.duration as i64);
                                session.end_time =
                                    Some(new_end.format("%-m/%-d/%Y %-I:%M %p").to_string());
                            } else {
                                session.duration = 60;
                                let new_end = start_dt + chrono::Duration::minutes(60);
                                session.end_time =
                                    Some(new_end.format("%-m/%-d/%Y %-I:%M %p").to_string());
                            }
                        }
                    }
                }
            }
        }
    }
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

    // Collect all panel sessions with their time and location info
    let mut panel_sessions: Vec<(
        String,
        String,
        chrono::NaiveDateTime,
        chrono::NaiveDateTime,
        Vec<u32>,
        Vec<String>,
    )> = Vec::new();
    let mut session_index_map: HashMap<String, usize> = HashMap::new();

    for (_panel_idx, (panel_id, panel)) in schedule.panels.iter().enumerate() {
        if let Some(ref pt_uid) = panel.panel_type {
            if is_break_event(Some(pt_uid), &panel_type_lookup) {
                continue;
            }
        }

        for (part_idx, part) in panel.parts.iter().enumerate() {
            for (session_idx, session) in part.sessions.iter().enumerate() {
                if let (Some(start_str), Some(end_str)) = (&session.start_time, &session.end_time) {
                    if let (Ok(start_time), Ok(end_time)) = (
                        chrono::NaiveDateTime::parse_from_str(start_str, "%Y-%m-%dT%H:%M:%S"),
                        chrono::NaiveDateTime::parse_from_str(end_str, "%Y-%m-%dT%H:%M:%S"),
                    ) {
                        let session_key = format!("{}-{}-{}", panel_id, part_idx, session_idx);
                        let all_presenters: Vec<String> = session
                            .credited_presenters
                            .iter()
                            .chain(session.uncredited_presenters.iter())
                            .cloned()
                            .collect();

                        panel_sessions.push((
                            session_key.clone(),
                            format!(
                                "{} (Part {}, Session {})",
                                panel.name,
                                part.part_num.unwrap_or(0),
                                session.session_num.unwrap_or(0)
                            ),
                            start_time,
                            end_time,
                            session.room_ids.clone(),
                            all_presenters,
                        ));

                        session_index_map.insert(session_key, panel_sessions.len() - 1);
                    }
                }
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
        let group_presenter = is_group_presenter(&presenter_name, &schedule.presenters);
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
    // Add to top-level conflicts
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

    // Find the actual panel sessions and add conflicts to them
    if let Some((first_panel_id, first_part_idx, first_session_idx)) =
        parse_session_key(&first_session.0)
    {
        if let Some(panel) = schedule.panels.get_mut(first_panel_id) {
            if let Some(part) = panel.parts.get_mut(first_part_idx) {
                if let Some(session) = part.sessions.get_mut(first_session_idx) {
                    let details = match conflict_type {
                        "group_presenter" => presenter_name.as_ref().map(|name| {
                            format!("Group presenter overlap: {name} in multiple events")
                        }),
                        "presenter" => presenter_name.as_ref().map(|name| {
                            format!(
                                "Double-booked with: {} (presenter: {name})",
                                second_session.1
                            )
                        }),
                        _ => Some(format!("Room conflict with: {}", second_session.1)),
                    };

                    session.conflicts.push(EventConflict {
                        conflict_type: conflict_type.to_string(),
                        details,
                        conflict_event_id: Some(second_session.0.clone()),
                    });
                }
            }
        }
    }

    if let Some((second_panel_id, second_part_idx, second_session_idx)) =
        parse_session_key(&second_session.0)
    {
        if let Some(panel) = schedule.panels.get_mut(second_panel_id) {
            if let Some(part) = panel.parts.get_mut(second_part_idx) {
                if let Some(session) = part.sessions.get_mut(second_session_idx) {
                    let details = match conflict_type {
                        "group_presenter" => presenter_name.as_ref().map(|name| {
                            format!("Group presenter overlap: {name} in multiple events")
                        }),
                        "presenter" => presenter_name.as_ref().map(|name| {
                            format!(
                                "Double-booked with: {} (presenter: {name})",
                                first_session.1
                            )
                        }),
                        _ => Some(format!("Room conflict with: {}", first_session.1)),
                    };

                    session.conflicts.push(EventConflict {
                        conflict_type: conflict_type.to_string(),
                        details,
                        conflict_event_id: Some(first_session.0.clone()),
                    });
                }
            }
        }
    }
}

/// Parse a session key back into panel_id, part_idx, session_idx
fn parse_session_key(key: &str) -> Option<(&str, usize, usize)> {
    let mut parts = key.splitn(3, '-');
    let panel_id = parts.next()?;
    let part_idx: usize = parts.next()?.parse().ok()?;
    let session_idx: usize = parts.next()?.parse().ok()?;
    Some((panel_id, part_idx, session_idx))
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

fn is_group_presenter(presenter_name: &str, presenters: &[Presenter]) -> bool {
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

    presenters
        .iter()
        .find(|presenter| presenter.name == presenter_name)
        .map(|presenter| presenter.is_group())
        .unwrap_or(false)
}

/// Resolve session ID conflicts by assigning unique alpha suffixes
/// When multiple sessions have the same Uniq ID and same panel name,
/// they get alpha suffixes (A, B, C, etc.) skipping P and S
/// Uses proper suffix generation: A..Z, AA..AZ, BA..BZ, etc.
fn resolve_session_conflicts(schedule: &mut Schedule) {
    // Process each panel and part once
    for (panel_id, panel) in &mut schedule.panels {
        for part in &mut panel.parts {
            resolve_part_conflicts(panel_id, part);
        }
    }
}

/// Resolve conflicts within a single part
fn resolve_part_conflicts(_panel_id: &str, part: &mut crate::data::panel::PanelPart) {
    // Short circuit if only one session (the common case)
    if part.sessions.len() <= 1 {
        return;
    }

    // First pass: collect session number/suffix combinations and detect conflicts
    // We can assume all sessions have the same base ID in this part
    let mut session_groups: HashMap<(Option<u32>, String), Vec<usize>> = HashMap::new();
    for (idx, session) in part.sessions.iter().enumerate() {
        let suffix = extract_alpha_suffix(&session.id);
        let key = (session.session_num, suffix);
        session_groups.entry(key).or_insert_with(Vec::new).push(idx);
    }

    // Find conflicts (same session number AND same suffix appearing multiple times)
    let conflicts: Vec<_> = session_groups
        .iter()
        .filter(|(_, indices)| indices.len() > 1)
        .collect();

    if conflicts.is_empty() {
        return; // No conflicts in this part
    }

    // Second pass: resolve conflicts by assigning unique suffixes
    for ((session_num, suffix), indices) in conflicts {
        if indices.len() > 1 {
            resolve_part_session_conflicts(part, *session_num, &suffix, &indices);
        }
    }
}

/// Resolve conflicts for sessions with the same session number and suffix within a single part
fn resolve_part_session_conflicts(
    part: &mut crate::data::panel::PanelPart,
    session_num: Option<u32>,
    original_suffix: &str,
    indices: &[usize],
) {
    // Extract base without suffix from any of the conflicting sessions
    let base_without_suffix = if let Some(first_idx) = indices.first() {
        strip_alpha_suffix(&part.sessions[*first_idx].id)
    } else {
        return;
    };

    // Collect existing (session_num, suffix) combinations for this base ID
    let mut used_combinations: HashSet<(Option<u32>, String)> = HashSet::new();
    for session in &part.sessions {
        if session.id.starts_with(&base_without_suffix) {
            let suffix = extract_alpha_suffix(&session.id);
            used_combinations.insert((session.session_num, suffix));
        }
    }

    // Assign new suffixes to ALL conflicting sessions
    // Start from the conflicted suffix and advance from there
    let mut start_suffix = original_suffix.to_string();
    for &session_idx in indices.iter() {
        let new_suffix =
            generate_unique_suffix_for_session(session_num, &start_suffix, &used_combinations);
        let new_id = format!("{}{}", base_without_suffix, new_suffix);

        part.sessions[session_idx].id = new_id.clone();
        part.sessions[session_idx].session_num = session_num;
        used_combinations.insert((session_num, new_suffix.clone()));

        // For next iteration, start from the suffix we just used
        start_suffix = new_suffix;
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
