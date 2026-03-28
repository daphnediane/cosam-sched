/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use crate::data::panel::{ExtraValue, Panel};
    use crate::data::panel_set::PanelSet;
    use crate::data::panel_type::PanelType;
    use crate::data::presenter::{
        Presenter, PresenterGroup, PresenterMember, PresenterRank, PresenterSortRank,
    };
    use crate::data::room::Room;
    use crate::data::schedule::{Meta, Schedule};
    use crate::data::source_info::{ChangeState, ImportedSheetPresence};
    use crate::edit::command::{PanelField, SessionField, SessionScheduleState};
    use crate::edit::context::EditContext;
    use crate::edit::find::{PanelTypeOptions, PresenterOptions, RoomOptions};
    use crate::edit::history::EditHistory;

    fn get_panel<'a>(schedule: &'a Schedule, id: &str) -> &'a Panel {
        schedule
            .panel_sets
            .values()
            .flat_map(|ps| ps.panels.iter())
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("panel '{}' not found", id))
    }

    fn get_panel_mut<'a>(schedule: &'a mut Schedule, id: &str) -> &'a mut Panel {
        schedule
            .panel_sets
            .values_mut()
            .flat_map(|ps| ps.panels.iter_mut())
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("panel '{}' not found", id))
    }

    fn make_test_schedule() -> Schedule {
        let mut panel = Panel::new("panel-1", "panel-1");
        panel.name = "Test Panel".to_string();
        panel.description = Some("Original description".to_string());
        panel.note = Some("Original note".to_string());
        panel.room_ids = vec![10];
        panel.set_start_time_from_str("2026-07-10T10:00:00");
        panel.set_end_time_from_str("2026-07-10T11:00:00");
        panel.set_duration_minutes(60);
        panel.credited_presenters = vec!["Alice".to_string(), "Bob".to_string()];

        let mut ps = PanelSet::new("panel-1");
        ps.panels.push(panel);
        let mut panel_sets = IndexMap::new();
        panel_sets.insert("panel-1".to_string(), ps);

        let rooms = vec![Room {
            uid: 10,
            short_name: "Main".to_string(),
            long_name: "Main Events".to_string(),
            hotel_room: "Salon F/G".to_string(),
            sort_key: 1,
            is_break: false,
            metadata: None,
            source: None,
            change_state: ChangeState::Unchanged,
        }];

        let mut panel_types = IndexMap::new();
        let mut colors = IndexMap::new();
        colors.insert("color".to_string(), "#E2F9D7".to_string());
        panel_types.insert(
            "GP".to_string(),
            PanelType {
                prefix: "GP".to_string(),
                kind: "Guest Panel".to_string(),
                colors,
                is_break: false,
                is_cafe: false,
                is_workshop: false,
                is_hidden: false,
                is_room_hours: false,
                is_timeline: false,
                is_private: false,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
        );

        let presenters = vec![
            Presenter {
                id: None,
                name: "Alice".to_string(),
                rank: PresenterRank::Guest,
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
                sort_rank: None,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
            Presenter {
                id: None,
                name: "Bob".to_string(),
                rank: PresenterRank::FanPanelist,
                is_member: PresenterMember::NotMember,
                is_grouped: PresenterGroup::NotGroup,
                sort_rank: None,
                metadata: None,
                source: None,
                change_state: ChangeState::Unchanged,
            },
        ];

        Schedule {
            conflicts: Vec::new(),
            meta: Meta {
                title: "Test Schedule".to_string(),
                generated: "2026-01-01T00:00:00Z".to_string(),
                version: Some(2),
                variant: None,
                generator: Some("test".to_string()),
                start_time: None,
                end_time: None,
                next_presenter_id: None,
                creator: None,
                last_modified_by: None,
                modified: None,
            },
            timeline: Vec::new(),
            panel_sets,
            rooms,
            panel_types,
            presenters,
            relationships: Default::default(),
            imported_sheets: ImportedSheetPresence::default(),
        }
    }

    // ── History basics ──────────────────────────────────────────

    #[test]
    fn history_undo_redo_state() {
        let history = EditHistory::new();
        assert!(!history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_count(), 0);
        assert_eq!(history.redo_count(), 0);
    }

    // ── set_panel_field + undo ──────────────────────────────────

    #[test]
    fn set_panel_description_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_field(
                "panel-1",
                PanelField::Description,
                Some("New description".to_string()),
            );
        }

        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("New description")
        );
        assert_eq!(
            get_panel(&schedule, "panel-1").change_state,
            ChangeState::Modified
        );
        assert!(history.can_undo());

        // Undo
        history.undo(&mut schedule);
        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("Original description")
        );

        // Redo
        history.redo(&mut schedule);
        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("New description")
        );
    }

    #[test]
    fn clear_panel_field_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_field("panel-1", PanelField::Note, None);
        }

        assert_eq!(get_panel(&schedule, "panel-1").note, None);

        history.undo(&mut schedule);
        assert_eq!(
            get_panel(&schedule, "panel-1").note.as_deref(),
            Some("Original note")
        );
    }

    // ── set_panel_field + undo ────────────────────────────────

    #[test]
    fn set_session_av_notes_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_field(
                "panel-1",
                SessionField::AvNotes,
                Some("Need projector".to_string()),
            );
        }

        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.av_notes.as_deref(), Some("Need projector"));
        assert_eq!(panel.change_state, ChangeState::Modified);

        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.av_notes, None);
    }

    // ── add / remove presenter + undo ───────────────────────────

    #[test]
    fn add_presenter_to_session_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.add_presenter_to_panel("panel-1", "Charlie");
        }

        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.credited_presenters.len(), 3);
        assert_eq!(panel.credited_presenters[2], "Charlie");

        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.credited_presenters.len(), 2);
    }

    #[test]
    fn add_presenter_duplicate_is_noop() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.add_presenter_to_panel("panel-1", "alice"); // case-insensitive
        }

        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.credited_presenters.len(), 2);
        assert!(!history.can_undo());
    }

    #[test]
    fn remove_presenter_from_session_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.remove_presenter_from_panel("panel-1", "Alice");
        }

        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.credited_presenters.len(), 1);
        assert_eq!(panel.credited_presenters[0], "Bob");

        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.credited_presenters.len(), 2);
        assert_eq!(panel.credited_presenters[0], "Alice");
        assert_eq!(panel.credited_presenters[1], "Bob");
    }

    // ── reschedule + undo ───────────────────────────────────────

    #[test]
    fn reschedule_session_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.reschedule_panel(
                "panel-1",
                SessionScheduleState {
                    room_ids: vec![20],
                    timing: crate::data::time::TimeRange::new_scheduled(
                        chrono::NaiveDateTime::parse_from_str(
                            "2026-07-11T14:00:00",
                            "%Y-%m-%dT%H:%M:%S",
                        )
                        .unwrap(),
                        chrono::Duration::minutes(90),
                    )
                    .unwrap(),
                },
            );
        }

        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.room_ids, vec![20]);
        assert_eq!(panel.effective_duration_minutes().unwrap_or(0), 90);
        assert_eq!(
            panel.start_time_str().as_deref(),
            Some("2026-07-11T14:00:00")
        );

        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.room_ids, vec![10]);
        assert_eq!(panel.effective_duration_minutes().unwrap_or(0), 60);
        assert_eq!(
            panel.start_time_str().as_deref(),
            Some("2026-07-10T10:00:00")
        );
    }

    // ── unschedule + undo ───────────────────────────────────────

    #[test]
    fn unschedule_session_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.unschedule_panel("panel-1");
        }

        let panel = get_panel(&schedule, "panel-1");
        assert!(panel.room_ids.is_empty());
        assert_eq!(panel.timing.start_time(), None);
        assert_eq!(panel.effective_end_time(), None);

        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.room_ids, vec![10]);
        assert_eq!(
            panel.start_time_str().as_deref(),
            Some("2026-07-10T10:00:00")
        );
        assert_eq!(panel.effective_duration_minutes().unwrap_or(0), 60);
    }

    // ── soft delete + undo ──────────────────────────────────────

    #[test]
    fn soft_delete_session_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.soft_delete_panel("panel-1");
        }

        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.change_state, ChangeState::Deleted);

        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.change_state, ChangeState::Unchanged);
    }

    #[test]
    fn soft_delete_panel_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.soft_delete_panel("panel-1");
        }

        assert_eq!(
            get_panel(&schedule, "panel-1").change_state,
            ChangeState::Deleted
        );

        history.undo(&mut schedule);
        assert_eq!(
            get_panel(&schedule, "panel-1").change_state,
            ChangeState::Unchanged
        );
    }

    // ── find_or_create_room ─────────────────────────────────────

    #[test]
    fn find_or_create_room_existing() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        let uid = {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.find_or_create_room(
                "Main",
                &RoomOptions {
                    hotel_room: Some("Salon A/B".to_string()),
                    ..Default::default()
                },
            )
        };

        assert_eq!(uid, 10);
        assert_eq!(schedule.rooms[0].hotel_room, "Salon A/B");
        assert_eq!(schedule.rooms[0].change_state, ChangeState::Modified);
        assert_eq!(schedule.rooms.len(), 1);

        history.undo(&mut schedule);
        assert_eq!(schedule.rooms[0].hotel_room, "Salon F/G");
    }

    #[test]
    fn find_or_create_room_new() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        let uid = {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.find_or_create_room(
                "Workshop 1",
                &RoomOptions {
                    long_name: Some("Workshop Room 1".to_string()),
                    hotel_room: Some("Salon C".to_string()),
                    sort_key: Some(5),
                    ..Default::default()
                },
            )
        };

        assert_eq!(uid, 11); // next after max uid 10
        assert_eq!(schedule.rooms.len(), 2);
        assert_eq!(schedule.rooms[1].short_name, "Workshop 1");
        assert_eq!(schedule.rooms[1].long_name, "Workshop Room 1");
        assert_eq!(schedule.rooms[1].change_state, ChangeState::Added);

        history.undo(&mut schedule);
        assert_eq!(schedule.rooms.len(), 1);
    }

    #[test]
    fn find_or_create_room_no_change_noop() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        let uid = {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.find_or_create_room("Main", &RoomOptions::default())
        };

        assert_eq!(uid, 10);
        assert!(!history.can_undo()); // no command pushed
    }

    // ── find_or_create_presenter ────────────────────────────────

    #[test]
    fn find_or_create_presenter_existing_rank_upgrade() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            // Bob is FanPanelist (priority 4); upgrade to Staff (priority 2)
            ctx.find_or_create_presenter(
                "Bob",
                &PresenterOptions {
                    rank: Some(PresenterRank::Staff),
                    ..Default::default()
                },
            );
        }

        let bob = schedule
            .presenters
            .iter()
            .find(|p| p.name == "Bob")
            .unwrap();
        assert_eq!(bob.rank, PresenterRank::Staff);
        assert_eq!(bob.change_state, ChangeState::Modified);

        history.undo(&mut schedule);
        let bob = schedule
            .presenters
            .iter()
            .find(|p| p.name == "Bob")
            .unwrap();
        assert_eq!(bob.rank, PresenterRank::FanPanelist);
    }

    #[test]
    fn find_or_create_presenter_existing_no_downgrade() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            // Alice is Guest (priority 0); trying FanPanelist (priority 4) should not downgrade
            ctx.find_or_create_presenter(
                "Alice",
                &PresenterOptions {
                    rank: Some(PresenterRank::FanPanelist),
                    ..Default::default()
                },
            );
        }

        let alice = schedule
            .presenters
            .iter()
            .find(|p| p.name == "Alice")
            .unwrap();
        assert_eq!(alice.rank, PresenterRank::Guest);
        assert!(!history.can_undo()); // no actual change
    }

    #[test]
    fn find_or_create_presenter_new() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.find_or_create_presenter(
                "Charlie",
                &PresenterOptions {
                    rank: Some(PresenterRank::Judge),
                    add_groups: vec!["Judges Panel".to_string()],
                    ..Default::default()
                },
            );
        }

        assert_eq!(schedule.presenters.len(), 3);
        let charlie = &schedule.presenters[2];
        assert_eq!(charlie.name, "Charlie");
        assert_eq!(charlie.rank, PresenterRank::Judge);
        assert!(charlie.groups().contains("Judges Panel"));
        assert_eq!(charlie.change_state, ChangeState::Added);

        history.undo(&mut schedule);
        assert_eq!(schedule.presenters.len(), 2);
    }

    #[test]
    fn find_or_create_presenter_add_group_membership() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.find_or_create_presenter(
                "Alice",
                &PresenterOptions {
                    add_groups: vec!["Cosplay Group".to_string()],
                    always_grouped: Some(true),
                    ..Default::default()
                },
            );
        }

        let alice = schedule
            .presenters
            .iter()
            .find(|p| p.name == "Alice")
            .unwrap();
        assert!(alice.groups().contains("Cosplay Group"));
        assert!(alice.always_grouped());

        history.undo(&mut schedule);
        let alice = schedule
            .presenters
            .iter()
            .find(|p| p.name == "Alice")
            .unwrap();
        assert!(!alice.always_grouped());
        assert!(alice.groups().is_empty());
    }

    // ── find_or_create_panel_type ───────────────────────────────

    #[test]
    fn find_or_create_panel_type_existing_update() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.find_or_create_panel_type(
                "GP",
                &PanelTypeOptions {
                    color: Some("#FFFFFF".to_string()),
                    is_workshop: Some(true),
                    ..Default::default()
                },
            );
        }

        let gp = &schedule.panel_types["GP"];
        assert_eq!(gp.color(), Some("#FFFFFF"));
        assert!(gp.is_workshop);
        assert_eq!(gp.kind, "Guest Panel"); // unchanged

        history.undo(&mut schedule);
        let gp = &schedule.panel_types["GP"];
        assert_eq!(gp.color(), Some("#E2F9D7"));
        assert!(!gp.is_workshop);
    }

    #[test]
    fn find_or_create_panel_type_new() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.find_or_create_panel_type(
                "WS",
                &PanelTypeOptions {
                    kind: Some("Workshop".to_string()),
                    color: Some("#FDEEB5".to_string()),
                    is_workshop: Some(true),
                    ..Default::default()
                },
            );
        }

        assert_eq!(schedule.panel_types.len(), 2);
        let ws = &schedule.panel_types["WS"];
        assert_eq!(ws.kind, "Workshop");
        assert_eq!(ws.color(), Some("#FDEEB5"));
        assert!(ws.is_workshop);
        assert_eq!(ws.change_state, ChangeState::Added);

        history.undo(&mut schedule);
        assert_eq!(schedule.panel_types.len(), 1);
        assert!(!schedule.panel_types.contains_key("WS"));
    }

    // ── metadata ────────────────────────────────────────────────

    #[test]
    fn set_and_clear_session_metadata() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_metadata(
                "panel-1",
                "ThemeColor",
                ExtraValue::String("#FF0000".to_string()),
            );
        }

        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(
            panel.metadata.get("ThemeColor"),
            Some(&ExtraValue::String("#FF0000".to_string()))
        );

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.clear_panel_metadata("panel-1", "ThemeColor");
        }

        let panel = get_panel(&schedule, "panel-1");
        assert!(!panel.metadata.contains_key("ThemeColor"));

        // Undo clear
        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(
            panel.metadata.get("ThemeColor"),
            Some(&ExtraValue::String("#FF0000".to_string()))
        );

        // Undo set
        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert!(!panel.metadata.contains_key("ThemeColor"));
    }

    // ── batch undo ──────────────────────────────────────────────

    #[test]
    fn batch_undo_reverses_all() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            use crate::edit::command::EditCommand;

            let commands = vec![
                EditCommand::SetPanelField {
                    panel_id: "panel-1".to_string(),
                    field: PanelField::Description,
                    old: None,
                    new: Some("Batch desc".to_string()),
                },
                EditCommand::SetPanelField {
                    panel_id: "panel-1".to_string(),
                    field: PanelField::Note,
                    old: None,
                    new: Some("Batch note".to_string()),
                },
            ];
            ctx.execute_batch(commands);
        }

        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("Batch desc")
        );
        assert_eq!(
            get_panel(&schedule, "panel-1").note.as_deref(),
            Some("Batch note")
        );
        assert_eq!(history.undo_count(), 1); // single batch

        history.undo(&mut schedule);
        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("Original description")
        );
        assert_eq!(
            get_panel(&schedule, "panel-1").note.as_deref(),
            Some("Original note")
        );
    }

    // ── history max depth ───────────────────────────────────────

    #[test]
    fn history_max_depth() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::with_max_depth(3);

        for i in 0..5 {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_field(
                "panel-1",
                PanelField::Description,
                Some(format!("Desc {}", i)),
            );
        }

        assert_eq!(history.undo_count(), 3);
    }

    // ── multiple undo/redo cycle ────────────────────────────────

    #[test]
    fn multiple_undo_redo_cycle() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        // Step 1
        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_field(
                "panel-1",
                PanelField::Description,
                Some("Step 1".to_string()),
            );
        }
        // Step 2
        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_field(
                "panel-1",
                PanelField::Description,
                Some("Step 2".to_string()),
            );
        }
        // Step 3
        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_field(
                "panel-1",
                PanelField::Description,
                Some("Step 3".to_string()),
            );
        }

        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("Step 3")
        );

        // Undo all 3
        history.undo(&mut schedule);
        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("Step 2")
        );
        history.undo(&mut schedule);
        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("Step 1")
        );
        history.undo(&mut schedule);
        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("Original description")
        );

        // Can't undo further
        assert!(!history.can_undo());

        // Redo 2 steps
        history.redo(&mut schedule);
        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("Step 1")
        );
        history.redo(&mut schedule);
        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("Step 2")
        );

        // New edit clears redo stack
        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_field(
                "panel-1",
                PanelField::Description,
                Some("Step 2b".to_string()),
            );
        }
        assert!(!history.can_redo());
        assert_eq!(
            get_panel(&schedule, "panel-1").description.as_deref(),
            Some("Step 2b")
        );
    }

    // ── group with no members ────────────────────────────────────

    #[test]
    fn find_or_create_presenter_group_no_members() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.find_or_create_presenter(
                "Staff Group",
                &PresenterOptions {
                    is_group: Some(true),
                    always_shown: Some(true),
                    ..Default::default()
                },
            );
        }

        let p = schedule
            .presenters
            .iter()
            .find(|p| p.name == "Staff Group")
            .expect("presenter created");
        assert!(
            matches!(&p.is_grouped, PresenterGroup::IsGroup(members, shown)
                if members.is_empty() && *shown),
            "should be IsGroup with empty members and always_shown=true"
        );

        // Undo removes the presenter
        history.undo(&mut schedule);
        assert!(
            !schedule.presenters.iter().any(|p| p.name == "Staff Group"),
            "undo should remove the presenter"
        );
    }

    // ── set_panel_name ──────────────────────────────────────────

    #[test]
    fn set_panel_name_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_name("panel-1", "Renamed Panel");
        }

        assert_eq!(get_panel(&schedule, "panel-1").name, "Renamed Panel");
        assert_eq!(
            get_panel(&schedule, "panel-1").change_state,
            ChangeState::Modified
        );

        history.undo(&mut schedule);
        assert_eq!(get_panel(&schedule, "panel-1").name, "Test Panel");

        history.redo(&mut schedule);
        assert_eq!(get_panel(&schedule, "panel-1").name, "Renamed Panel");
    }

    // ── room metadata ──────────────────────────────────────────

    #[test]
    fn set_and_clear_room_metadata() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_room_metadata(10, "floor", ExtraValue::String("3rd".to_string()));
        }

        let room = &schedule.rooms[0];
        assert_eq!(
            room.metadata.as_ref().unwrap().get("floor"),
            Some(&ExtraValue::String("3rd".to_string()))
        );
        assert_eq!(room.change_state, ChangeState::Modified);

        // Clear
        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.clear_room_metadata(10, "floor");
        }

        let room = &schedule.rooms[0];
        assert!(room.metadata.is_none() || !room.metadata.as_ref().unwrap().contains_key("floor"));

        // Undo clear → key back
        history.undo(&mut schedule);
        let room = &schedule.rooms[0];
        assert_eq!(
            room.metadata.as_ref().unwrap().get("floor"),
            Some(&ExtraValue::String("3rd".to_string()))
        );

        // Undo set → no metadata
        history.undo(&mut schedule);
        let room = &schedule.rooms[0];
        assert!(room.metadata.is_none() || !room.metadata.as_ref().unwrap().contains_key("floor"));
    }

    #[test]
    fn clear_room_metadata_noop_when_missing() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.clear_room_metadata(10, "nonexistent");
        }

        assert!(!history.can_undo());
    }

    // ── panel type metadata ────────────────────────────────────

    #[test]
    fn set_and_clear_panel_type_metadata() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_type_metadata("GP", "priority", ExtraValue::String("high".to_string()));
        }

        let pt = &schedule.panel_types["GP"];
        assert_eq!(
            pt.metadata.as_ref().unwrap().get("priority"),
            Some(&ExtraValue::String("high".to_string()))
        );
        assert_eq!(pt.change_state, ChangeState::Modified);

        // Clear
        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.clear_panel_type_metadata("GP", "priority");
        }

        let pt = &schedule.panel_types["GP"];
        assert!(pt.metadata.is_none() || !pt.metadata.as_ref().unwrap().contains_key("priority"));

        // Undo clear
        history.undo(&mut schedule);
        let pt = &schedule.panel_types["GP"];
        assert_eq!(
            pt.metadata.as_ref().unwrap().get("priority"),
            Some(&ExtraValue::String("high".to_string()))
        );

        // Undo set
        history.undo(&mut schedule);
        let pt = &schedule.panel_types["GP"];
        assert!(pt.metadata.is_none() || !pt.metadata.as_ref().unwrap().contains_key("priority"));
    }

    #[test]
    fn clear_panel_type_metadata_noop_when_missing() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.clear_panel_type_metadata("GP", "nonexistent");
        }

        assert!(!history.can_undo());
    }

    // ── panel presenters ───────────────────────────────────────

    #[test]
    fn set_panel_presenters_and_undo() {
        let mut schedule = make_test_schedule();
        // Give the panel some credited presenters
        get_panel_mut(&mut schedule, "panel-1").credited_presenters =
            vec!["Alice".to_string(), "Bob".to_string()];
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_presenters("panel-1", vec!["Charlie".to_string(), "Diana".to_string()]);
        }

        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(
            panel.credited_presenters,
            vec!["Charlie".to_string(), "Diana".to_string()]
        );
        assert_eq!(panel.change_state, ChangeState::Modified);

        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(
            panel.credited_presenters,
            vec!["Alice".to_string(), "Bob".to_string()]
        );
    }

    // ── session presenters ─────────────────────────────────────

    #[test]
    fn set_session_presenters_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_presenters("panel-1", vec!["Charlie".to_string(), "Diana".to_string()]);
        }

        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(
            panel.credited_presenters,
            vec!["Charlie".to_string(), "Diana".to_string()]
        );
        assert_eq!(panel.change_state, ChangeState::Modified);

        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(
            panel.credited_presenters,
            vec!["Alice".to_string(), "Bob".to_string()]
        );
    }

    #[test]
    fn set_session_presenters_empty_and_undo() {
        let mut schedule = make_test_schedule();
        let mut history = EditHistory::new();

        {
            let mut ctx = EditContext::new(&mut schedule, &mut history);
            ctx.set_panel_presenters("panel-1", Vec::new());
        }

        let panel = get_panel(&schedule, "panel-1");
        assert!(panel.credited_presenters.is_empty());

        history.undo(&mut schedule);
        let panel = get_panel(&schedule, "panel-1");
        assert_eq!(panel.credited_presenters.len(), 2);
    }

    // ── update_or_create_presenter ──────────────────────────────

    fn make_empty_schedule() -> Schedule {
        Schedule {
            conflicts: Vec::new(),
            meta: Meta {
                title: "Test".to_string(),
                generated: "2026-01-01T00:00:00Z".to_string(),
                version: Some(7),
                variant: None,
                generator: Some("test".to_string()),
                start_time: None,
                end_time: None,
                next_presenter_id: None,
                creator: None,
                last_modified_by: None,
                modified: None,
            },
            timeline: Vec::new(),
            panel_sets: Default::default(),
            rooms: Vec::new(),
            panel_types: Default::default(),
            presenters: Vec::new(),
            relationships: Default::default(),
            imported_sheets: ImportedSheetPresence::default(),
        }
    }

    #[test]
    fn update_or_create_tagged_simple() {
        let mut schedule = make_empty_schedule();
        let name = {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.update_or_create_presenter("G:Yaya Han", true, Some(5), Some(0))
        };
        assert_eq!(name, Some("Yaya Han".to_string()));
        assert_eq!(schedule.presenters.len(), 1);
        let p = &schedule.presenters[0];
        assert_eq!(p.rank, PresenterRank::Guest);
        // sort_rank should be schedule_member(5, 0) = {column_index:5, row_index:0, member_index:1}
        assert_eq!(p.sort_rank, Some(PresenterSortRank::schedule_member(5, 0)));
    }

    #[test]
    fn update_or_create_existing_returns_stored_name() {
        let mut schedule = make_empty_schedule();
        // Pre-populate
        schedule.presenters.push(Presenter {
            id: None,
            name: "Alice".to_string(),
            rank: PresenterRank::Guest,
            is_member: PresenterMember::NotMember,
            is_grouped: PresenterGroup::NotGroup,
            sort_rank: None,
            metadata: None,
            source: None,
            change_state: ChangeState::Unchanged,
        });
        let name = {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.update_or_create_presenter("alice", true, None, None)
        };
        // Should return the stored casing
        assert_eq!(name, Some("Alice".to_string()));
        assert_eq!(schedule.presenters.len(), 1);
    }

    #[test]
    fn update_or_create_tagged_with_group() {
        let mut schedule = make_empty_schedule();
        let name = {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.update_or_create_presenter("G:John=UNC Staff", true, Some(3), Some(0))
        };
        assert_eq!(name, Some("John".to_string()));
        // Should have created both John and UNC Staff
        assert_eq!(schedule.presenters.len(), 2);

        let john = schedule
            .presenters
            .iter()
            .find(|p| p.name == "John")
            .unwrap();
        assert!(john.groups().contains("UNC Staff"));
        assert!(!john.always_grouped());

        let group = schedule
            .presenters
            .iter()
            .find(|p| p.name == "UNC Staff")
            .unwrap();
        assert!(group.is_group());
        assert!(group.members().contains("John"));
        assert!(!group.always_shown());
    }

    #[test]
    fn update_or_create_tagged_double_eq_always_shown() {
        let mut schedule = make_empty_schedule();
        let name = {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.update_or_create_presenter("G:John==UNC Staff", true, None, None)
        };
        assert_eq!(name, Some("John".to_string()));

        let group = schedule
            .presenters
            .iter()
            .find(|p| p.name == "UNC Staff")
            .unwrap();
        assert!(group.is_group());
        assert!(group.always_shown(), "== should set always_shown");
    }

    #[test]
    fn update_or_create_tagged_lt_always_grouped() {
        let mut schedule = make_empty_schedule();
        let name = {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.update_or_create_presenter("G:<Jane=UNC Staff", true, None, None)
        };
        assert_eq!(name, Some("Jane".to_string()));

        let jane = schedule
            .presenters
            .iter()
            .find(|p| p.name == "Jane")
            .unwrap();
        assert!(jane.always_grouped(), "< prefix should set always_grouped");
        assert!(jane.groups().contains("UNC Staff"));

        let group = schedule
            .presenters
            .iter()
            .find(|p| p.name == "UNC Staff")
            .unwrap();
        assert!(
            !group.always_shown(),
            "single = should not set always_shown"
        );
    }

    #[test]
    fn update_or_create_tagged_lt_double_eq_combined() {
        let mut schedule = make_empty_schedule();
        let name = {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.update_or_create_presenter("G:<Bob==Team", true, None, None)
        };
        assert_eq!(name, Some("Bob".to_string()));

        let bob = schedule
            .presenters
            .iter()
            .find(|p| p.name == "Bob")
            .unwrap();
        assert!(bob.always_grouped());

        let team = schedule
            .presenters
            .iter()
            .find(|p| p.name == "Team")
            .unwrap();
        assert!(team.always_shown(), "== should set always_shown");
    }

    #[test]
    fn update_or_create_no_tag_creates_with_default_rank() {
        let mut schedule = make_empty_schedule();
        let name = {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.update_or_create_presenter("Some Person", true, None, None)
        };
        assert_eq!(name, Some("Some Person".to_string()));
        assert_eq!(schedule.presenters[0].rank, PresenterRank::FanPanelist);
    }

    #[test]
    fn update_or_create_no_tag_no_create() {
        let mut schedule = make_empty_schedule();
        let name = {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.update_or_create_presenter("Nobody", false, None, None)
        };
        assert_eq!(name, None);
        assert!(schedule.presenters.is_empty());
    }

    #[test]
    fn update_or_create_empty_returns_none() {
        let mut schedule = make_empty_schedule();
        let name = {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.update_or_create_presenter("", true, None, None)
        };
        assert_eq!(name, None);
    }

    #[test]
    fn update_or_create_group_only_no_circular() {
        let mut schedule = make_empty_schedule();
        // "G:==UNC Staff" → empty name, group = UNC Staff, always_shown
        let name = {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.update_or_create_presenter("G:==UNC Staff", true, None, None)
        };
        assert_eq!(name, Some("UNC Staff".to_string()));
        assert_eq!(schedule.presenters.len(), 1);

        let p = &schedule.presenters[0];
        assert!(p.is_group());
        assert!(p.always_shown());
        // Must NOT be a member of itself
        assert!(p.groups().is_empty());
        assert!(!p.members().contains("UNC Staff"));
    }

    #[test]
    fn update_or_create_column_and_index_rank() {
        let mut schedule = make_empty_schedule();
        {
            let mut ctx = EditContext::import(&mut schedule);
            // Create with column_index=7, row_index=3
            ctx.update_or_create_presenter("P:TestPerson", true, Some(7), Some(3));
        }
        let p = &schedule.presenters[0];
        // Tagged presenter gets member_index=1 (schedule_member)
        assert_eq!(p.sort_rank, Some(PresenterSortRank::schedule_member(7, 3)));
    }

    #[test]
    fn add_relationship_command() {
        use crate::data::relationship::GroupEdge;
        use crate::edit::command::EditCommand;

        let mut schedule = make_empty_schedule();
        // Create two presenters first
        {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.find_or_create_presenter(
                "Alice",
                &PresenterOptions {
                    rank: Some(PresenterRank::Guest),
                    ..Default::default()
                },
            );
            ctx.find_or_create_presenter(
                "Team Alice",
                &PresenterOptions {
                    rank: Some(PresenterRank::Guest),
                    is_group: Some(true),
                    ..Default::default()
                },
            );
        }

        // Add a relationship via command
        let mut cmd = EditCommand::AddRelationship {
            edge: GroupEdge::new("Alice".to_string(), "Team Alice".to_string(), true, false),
        };
        cmd.apply(&mut schedule);

        assert!(schedule.relationships.is_group("Team Alice"));
        let members = schedule.relationships.get_direct_members("Team Alice");
        assert!(members.contains(&"Alice".to_string()));
        assert!(schedule.relationships.is_any_always_grouped("Alice"));

        // Undo it
        cmd.undo(&mut schedule);
        let members = schedule.relationships.get_direct_members("Team Alice");
        assert!(
            !members.contains(&"Alice".to_string()),
            "Alice should be removed after undo"
        );
    }

    #[test]
    fn remove_relationship_command() {
        use crate::edit::command::EditCommand;

        let mut schedule = make_empty_schedule();
        // Create a presenter with a group relationship
        {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.find_or_create_presenter(
                "Team Bob",
                &PresenterOptions {
                    rank: Some(PresenterRank::Guest),
                    is_group: Some(true),
                    add_members: vec!["Bob".to_string()],
                    ..Default::default()
                },
            );
            ctx.find_or_create_presenter(
                "Bob",
                &PresenterOptions {
                    rank: Some(PresenterRank::Guest),
                    add_groups: vec!["Team Bob".to_string()],
                    ..Default::default()
                },
            );
        }

        // Verify relationship exists
        assert!(schedule.relationships.is_group("Team Bob"));

        // Remove via command
        let mut cmd = EditCommand::RemoveRelationship {
            member: "Bob".to_string(),
            group: "Team Bob".to_string(),
            old_edge: None,
        };
        cmd.apply(&mut schedule);

        let members = schedule.relationships.get_direct_members("Team Bob");
        assert!(
            !members.contains(&"Bob".to_string()),
            "Bob should be removed from Team Bob"
        );

        // Undo restores the edge
        cmd.undo(&mut schedule);
        let members = schedule.relationships.get_direct_members("Team Bob");
        assert!(
            members.contains(&"Bob".to_string()),
            "Bob should be restored after undo"
        );
    }

    #[test]
    fn create_presenter_syncs_relationships() {
        let mut schedule = make_empty_schedule();
        {
            let mut ctx = EditContext::import(&mut schedule);
            ctx.find_or_create_presenter(
                "Duo Group",
                &PresenterOptions {
                    rank: Some(PresenterRank::Guest),
                    is_group: Some(true),
                    add_members: vec!["Member A".to_string()],
                    always_shown: Some(true),
                    ..Default::default()
                },
            );
        }

        // RelationshipManager should be in sync after CreatePresenter
        assert!(schedule.relationships.is_group("Duo Group"));
        assert!(schedule.relationships.is_always_shown("Duo Group"));
        let members = schedule.relationships.get_direct_members("Duo Group");
        assert!(members.contains(&"Member A".to_string()));
    }

    #[test]
    fn people_table_rank_not_overwritten_by_schedule() {
        // Regression: People table sets sort_rank with column_index=0.
        // Schedule processing must not overwrite with higher column_index.
        let mut schedule = make_empty_schedule();
        {
            let mut ctx = EditContext::import(&mut schedule);
            // Simulate People table: column_index=0, row_index=2
            ctx.find_or_create_presenter(
                "Reika",
                &PresenterOptions {
                    rank: Some(PresenterRank::Guest),
                    sort_rank: Some(PresenterSortRank::people(2)),
                    ..Default::default()
                },
            );
        }
        assert_eq!(
            schedule.presenters[0].sort_rank,
            Some(PresenterSortRank::people(2))
        );

        {
            let mut ctx = EditContext::import(&mut schedule);
            // Simulate schedule column 11, position 0
            ctx.update_or_create_presenter("G:Reika", true, Some(11), Some(0));
        }
        let reika = schedule
            .presenters
            .iter()
            .find(|p| p.name == "Reika")
            .unwrap();
        assert_eq!(
            reika.sort_rank,
            Some(PresenterSortRank::people(2)),
            "People table sort_rank must not be overwritten by schedule"
        );
    }

    #[test]
    fn xlsx_syntax_populates_relationship_manager() {
        // Verify that all G: syntax variants correctly populate the RelationshipManager
        let mut schedule = make_empty_schedule();
        {
            let mut ctx = EditContext::import(&mut schedule);

            // G:Name=Group → always_grouped=false, always_shown=false
            ctx.update_or_create_presenter("G:Alice=TeamA", true, None, None);

            // G:Name==Group → always_shown=true
            ctx.update_or_create_presenter("G:Bob==TeamB", true, None, None);

            // G:<Name=Group → always_grouped=true
            ctx.update_or_create_presenter("G:<Carol=TeamC", true, None, None);

            // G:<Name==Group → always_grouped=true, always_shown=true
            ctx.update_or_create_presenter("G:<Dave==TeamD", true, None, None);

            // G:==Group → group-only edge (empty member), always_shown=true
            ctx.update_or_create_presenter("G:==TeamE", true, None, None);
        }

        let rels = &schedule.relationships;

        // G:Alice=TeamA
        assert!(rels.is_group("TeamA"));
        assert!(
            rels.direct_members_of("TeamA")
                .contains(&"Alice".to_string())
        );
        assert!(
            rels.direct_groups_of("Alice")
                .contains(&"TeamA".to_string())
        );
        assert!(!rels.is_any_always_grouped("Alice"));
        assert!(!rels.is_always_shown("TeamA"));

        // G:Bob==TeamB
        assert!(rels.is_group("TeamB"));
        assert!(rels.direct_members_of("TeamB").contains(&"Bob".to_string()));
        assert!(!rels.is_any_always_grouped("Bob"));
        assert!(rels.is_always_shown("TeamB"));

        // G:<Carol=TeamC
        assert!(rels.is_group("TeamC"));
        assert!(
            rels.direct_members_of("TeamC")
                .contains(&"Carol".to_string())
        );
        assert!(rels.is_any_always_grouped("Carol"));
        assert!(!rels.is_always_shown("TeamC"));

        // G:<Dave==TeamD
        assert!(rels.is_group("TeamD"));
        assert!(
            rels.direct_members_of("TeamD")
                .contains(&"Dave".to_string())
        );
        assert!(rels.is_any_always_grouped("Dave"));
        assert!(rels.is_always_shown("TeamD"));

        // G:==TeamE (group-only, no member)
        assert!(rels.is_group("TeamE"));
        assert!(rels.direct_members_of("TeamE").is_empty());
        assert!(rels.is_always_shown("TeamE"));
    }
}
