use std::collections::{HashMap, HashSet};

use super::event::EventConflict;
use super::panel_type::PanelType;
use super::presenter::Presenter;
use super::schedule::{ConflictEventRef, Schedule, ScheduleConflict};

const GROUP_SUFFIX_PATTERNS: [&str; 1] = ["staff"];

pub fn apply_schedule_parity(schedule: &mut Schedule) {
    normalize_event_times(schedule);
    generate_credits(schedule);
    detect_conflicts(schedule);
    schedule.calculate_schedule_bounds();
}

fn normalize_event_times(schedule: &mut Schedule) {
    for event in &mut schedule.events {
        if event.end_time < event.start_time {
            let duration_minutes = if event.duration == 0 {
                60
            } else {
                event.duration
            };
            event.end_time = event.start_time + chrono::Duration::minutes(duration_minutes as i64);
            event.duration = duration_minutes;
            continue;
        }

        let computed_minutes = (event.end_time - event.start_time).num_minutes();
        if computed_minutes > 0 {
            event.duration = computed_minutes as u32;
        } else if event.duration > 0 {
            event.end_time = event.start_time + chrono::Duration::minutes(event.duration as i64);
        } else {
            event.duration = 60;
            event.end_time = event.start_time + chrono::Duration::minutes(60);
        }
    }
}

fn generate_credits(schedule: &mut Schedule) {
    let presenter_lookup: HashMap<&str, &Presenter> = schedule
        .presenters
        .iter()
        .map(|presenter| (presenter.name.as_str(), presenter))
        .collect();

    for event in &mut schedule.events {
        let presenters = event.presenters.clone();
        let mut credits = Vec::new();
        let mut processed: HashSet<String> = HashSet::new();

        for presenter_name in &presenters {
            if processed.contains(presenter_name) {
                continue;
            }

            let Some(presenter_info) = presenter_lookup.get(presenter_name.as_str()) else {
                continue;
            };

            if presenter_info.always_grouped {
                credits.push(presenter_name.clone());
                processed.insert(presenter_name.clone());
            }
        }

        for presenter_name in &presenters {
            if processed.contains(presenter_name) {
                continue;
            }

            let Some(presenter_info) = presenter_lookup.get(presenter_name.as_str()) else {
                credits.push(presenter_name.clone());
                processed.insert(presenter_name.clone());
                continue;
            };

            if !presenter_info.groups.is_empty() {
                let mut handled_group = false;
                for group_name in &presenter_info.groups {
                    let Some(group_info) = presenter_lookup.get(group_name.as_str()) else {
                        continue;
                    };
                    if !group_info.is_group {
                        continue;
                    }

                    let present_members: Vec<&str> = group_info
                        .members
                        .iter()
                        .map(String::as_str)
                        .filter(|member_name| presenters.iter().any(|name| name == member_name))
                        .collect();

                    for member_name in &present_members {
                        processed.insert((*member_name).to_string());
                    }
                    processed.insert(group_name.clone());

                    if !group_info.members.is_empty()
                        && present_members.len() == group_info.members.len()
                    {
                        credits.push(group_name.clone());
                    } else {
                        for member_name in present_members {
                            credits.push(format!("{member_name} of {group_name}"));
                        }
                    }

                    handled_group = true;
                    break;
                }

                if handled_group {
                    continue;
                }
            }

            if presenter_info.is_group {
                let present_members: Vec<&str> = presenter_info
                    .members
                    .iter()
                    .map(String::as_str)
                    .filter(|member_name| presenters.iter().any(|name| name == member_name))
                    .collect();

                if present_members.is_empty()
                    || (!presenter_info.members.is_empty()
                        && present_members.len() == presenter_info.members.len())
                {
                    credits.push(presenter_name.clone());
                } else {
                    for member_name in &present_members {
                        credits.push(format!("{member_name} of {presenter_name}"));
                    }
                }

                processed.insert(presenter_name.clone());
                for member_name in present_members {
                    processed.insert(member_name.to_string());
                }
                continue;
            }

            credits.push(presenter_name.clone());
            processed.insert(presenter_name.clone());
        }

        event.credits = credits;
    }
}

fn detect_conflicts(schedule: &mut Schedule) {
    let panel_type_lookup: HashMap<String, &PanelType> = schedule
        .panel_types
        .iter()
        .map(|panel_type| (panel_type.effective_uid(), panel_type))
        .collect();

    let mut presenter_events: HashMap<String, Vec<usize>> = HashMap::new();
    let mut room_events: HashMap<u32, Vec<usize>> = HashMap::new();

    for (event_index, event) in schedule.events.iter().enumerate() {
        if is_break_event(event.panel_type.as_deref(), &panel_type_lookup) {
            continue;
        }

        for presenter_name in &event.presenters {
            presenter_events
                .entry(presenter_name.clone())
                .or_default()
                .push(event_index);
        }

        if let Some(room_id) = event.room_id {
            room_events.entry(room_id).or_default().push(event_index);
        }
    }

    let mut top_level_conflicts = Vec::new();
    let mut per_event_conflicts: HashMap<usize, Vec<EventConflict>> = HashMap::new();

    for (presenter_name, event_indexes) in presenter_events {
        if event_indexes.len() < 2 {
            continue;
        }

        let mut sorted_event_indexes = event_indexes;
        sorted_event_indexes.sort_by_key(|index| schedule.events[*index].start_time);

        let overlap_groups = find_overlap_groups(&sorted_event_indexes, schedule);
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

            for first_position in 0..(overlap_group.len() - 1) {
                for second_position in (first_position + 1)..overlap_group.len() {
                    let first_event_index = overlap_group[first_position];
                    let second_event_index = overlap_group[second_position];
                    add_conflict_pair(
                        schedule,
                        &mut top_level_conflicts,
                        &mut per_event_conflicts,
                        first_event_index,
                        second_event_index,
                        conflict_type,
                        Some(presenter_name.clone()),
                        None,
                    );
                }
            }
        }
    }

    for (room_id, event_indexes) in room_events {
        if event_indexes.len() < 2 {
            continue;
        }

        let mut sorted_event_indexes = event_indexes;
        sorted_event_indexes.sort_by_key(|index| schedule.events[*index].start_time);
        let overlap_groups = find_overlap_groups(&sorted_event_indexes, schedule);

        for overlap_group in overlap_groups {
            if overlap_group.len() < 2 {
                continue;
            }

            for first_position in 0..(overlap_group.len() - 1) {
                for second_position in (first_position + 1)..overlap_group.len() {
                    let first_event_index = overlap_group[first_position];
                    let second_event_index = overlap_group[second_position];
                    add_conflict_pair(
                        schedule,
                        &mut top_level_conflicts,
                        &mut per_event_conflicts,
                        first_event_index,
                        second_event_index,
                        "room",
                        None,
                        Some(serde_json::json!(room_id)),
                    );
                }
            }
        }
    }

    for (event_index, event) in schedule.events.iter_mut().enumerate() {
        event.conflicts = per_event_conflicts.remove(&event_index).unwrap_or_default();
    }

    schedule.conflicts = top_level_conflicts;
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
        .map(|presenter| presenter.is_group)
        .unwrap_or(false)
}

fn find_overlap_groups(event_indexes: &[usize], schedule: &Schedule) -> Vec<Vec<usize>> {
    let mut overlap_groups: Vec<Vec<usize>> = Vec::new();
    let Some(first_index) = event_indexes.first().copied() else {
        return overlap_groups;
    };

    let mut current_group = vec![first_index];
    let mut current_end = schedule.events[first_index].end_time;

    for &event_index in event_indexes.iter().skip(1) {
        let event = &schedule.events[event_index];
        if event.start_time < current_end {
            current_group.push(event_index);
            if event.end_time > current_end {
                current_end = event.end_time;
            }
            continue;
        }

        overlap_groups.push(current_group);
        current_group = vec![event_index];
        current_end = event.end_time;
    }

    overlap_groups.push(current_group);
    overlap_groups
}

fn add_conflict_pair(
    schedule: &Schedule,
    top_level_conflicts: &mut Vec<ScheduleConflict>,
    per_event_conflicts: &mut HashMap<usize, Vec<EventConflict>>,
    first_event_index: usize,
    second_event_index: usize,
    conflict_type: &str,
    presenter_name: Option<String>,
    room_value: Option<serde_json::Value>,
) {
    let first_event = &schedule.events[first_event_index];
    let second_event = &schedule.events[second_event_index];

    top_level_conflicts.push(ScheduleConflict {
        event1: ConflictEventRef {
            id: first_event.id.clone(),
            name: first_event.name.clone(),
        },
        event2: ConflictEventRef {
            id: second_event.id.clone(),
            name: second_event.name.clone(),
        },
        presenter: presenter_name.clone(),
        room: room_value.clone(),
        conflict_type: conflict_type.to_string(),
    });

    let first_details = match conflict_type {
        "group_presenter" => presenter_name
            .as_ref()
            .map(|name| format!("Group presenter overlap: {name} in multiple events")),
        "presenter" => presenter_name.as_ref().map(|name| {
            format!(
                "Double-booked with: {} (presenter: {name})",
                second_event.name
            )
        }),
        _ => Some(format!("Room conflict with: {}", second_event.name)),
    };

    let second_details = match conflict_type {
        "group_presenter" => presenter_name
            .as_ref()
            .map(|name| format!("Group presenter overlap: {name} in multiple events")),
        "presenter" => presenter_name.as_ref().map(|name| {
            format!(
                "Double-booked with: {} (presenter: {name})",
                first_event.name
            )
        }),
        _ => Some(format!("Room conflict with: {}", first_event.name)),
    };

    per_event_conflicts
        .entry(first_event_index)
        .or_default()
        .push(EventConflict {
            conflict_type: conflict_type.to_string(),
            details: first_details,
            conflict_event_id: Some(second_event.id.clone()),
        });

    per_event_conflicts
        .entry(second_event_index)
        .or_default()
        .push(EventConflict {
            conflict_type: conflict_type.to_string(),
            details: second_details,
            conflict_event_id: Some(first_event.id.clone()),
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::event::Event;
    use crate::data::schedule::Meta;
    use crate::data::source_info::{ChangeState, ImportedSheetPresence};
    use chrono::NaiveDateTime;

    fn parse_datetime(timestamp: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S").expect("valid datetime")
    }

    fn empty_schedule() -> Schedule {
        Schedule {
            conflicts: Vec::new(),
            meta: Meta {
                title: "Test".to_string(),
                generated: "2026-01-01T00:00:00Z".to_string(),
                version: Some(4),
                generator: Some("test".to_string()),
                start_time: None,
                end_time: None,
            },
            timeline: Vec::new(),
            events: Vec::new(),
            rooms: Vec::new(),
            panel_types: Vec::new(),
            time_types: Vec::new(),
            presenters: Vec::new(),
            imported_sheets: ImportedSheetPresence::default(),
        }
    }

    #[test]
    fn generates_group_credits() {
        let mut schedule = empty_schedule();
        schedule.presenters = vec![
            Presenter {
                name: "Pros and Cons Cosplay".to_string(),
                rank: "guest".to_string(),
                is_group: true,
                members: vec!["Pro".to_string(), "Con".to_string()],
                groups: Vec::new(),
                always_grouped: false,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                name: "Pro".to_string(),
                rank: "guest".to_string(),
                is_group: false,
                members: Vec::new(),
                groups: vec!["Pros and Cons Cosplay".to_string()],
                always_grouped: false,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                name: "Con".to_string(),
                rank: "guest".to_string(),
                is_group: false,
                members: Vec::new(),
                groups: vec!["Pros and Cons Cosplay".to_string()],
                always_grouped: false,
                source: None,
                change_state: ChangeState::Unchanged,
            },
        ];

        schedule.events.push(Event {
            id: "GP001".to_string(),
            name: "Panel".to_string(),
            description: None,
            start_time: parse_datetime("2026-06-26T10:00:00"),
            end_time: parse_datetime("2026-06-26T11:00:00"),
            duration: 60,
            room_id: Some(1),
            panel_type: None,
            cost: None,
            capacity: None,
            difficulty: None,
            note: None,
            prereq: None,
            ticket_url: None,
            presenters: vec!["Pro".to_string(), "Con".to_string()],
            credits: Vec::new(),
            conflicts: Vec::new(),
            is_free: true,
            is_full: false,
            is_kids: false,
            hide_panelist: false,
            alt_panelist: None,
            source: None,
            change_state: ChangeState::Unchanged,
        });

        apply_schedule_parity(&mut schedule);

        assert_eq!(schedule.events[0].credits, vec!["Pros and Cons Cosplay"]);
    }

    #[test]
    fn detects_presenter_conflicts() {
        let mut schedule = empty_schedule();
        schedule.events.push(Event {
            id: "A".to_string(),
            name: "A".to_string(),
            description: None,
            start_time: parse_datetime("2026-06-26T10:00:00"),
            end_time: parse_datetime("2026-06-26T11:00:00"),
            duration: 60,
            room_id: Some(1),
            panel_type: None,
            cost: None,
            capacity: None,
            difficulty: None,
            note: None,
            prereq: None,
            ticket_url: None,
            presenters: vec!["Alice".to_string()],
            credits: Vec::new(),
            conflicts: Vec::new(),
            is_free: true,
            is_full: false,
            is_kids: false,
            hide_panelist: false,
            alt_panelist: None,
            source: None,
            change_state: ChangeState::Unchanged,
        });
        schedule.events.push(Event {
            id: "B".to_string(),
            name: "B".to_string(),
            description: None,
            start_time: parse_datetime("2026-06-26T10:30:00"),
            end_time: parse_datetime("2026-06-26T11:30:00"),
            duration: 60,
            room_id: Some(2),
            panel_type: None,
            cost: None,
            capacity: None,
            difficulty: None,
            note: None,
            prereq: None,
            ticket_url: None,
            presenters: vec!["Alice".to_string()],
            credits: Vec::new(),
            conflicts: Vec::new(),
            is_free: true,
            is_full: false,
            is_kids: false,
            hide_panelist: false,
            alt_panelist: None,
            source: None,
            change_state: ChangeState::Unchanged,
        });

        apply_schedule_parity(&mut schedule);

        assert_eq!(schedule.conflicts.len(), 1);
        assert_eq!(schedule.conflicts[0].conflict_type, "presenter");
        assert_eq!(schedule.events[0].conflicts.len(), 1);
        assert_eq!(schedule.events[1].conflicts.len(), 1);
    }
}
