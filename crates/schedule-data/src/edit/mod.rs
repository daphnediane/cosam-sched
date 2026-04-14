/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Edit command system for tracked, reversible schedule mutations.
//!
//! This module provides:
//!
//! - [`EditCommand`] — the atomic unit of mutation (field updates, compound
//!   operations) with `apply` and `undo` methods.
//! - [`EditHistory`] — linear undo/redo stacks.
//! - [`EditContext`] — convenience wrapper pairing a [`Schedule`] with an
//!   optional [`EditHistory`] for single-call execute/undo/redo.
//!
//! All schedule mutations in the editor flow through `EditCommand` so that
//! every change is reversible. Compound commands bundle multi-entity
//! operations (e.g., "add tagged presenter to panel") into a single
//! undo/redo step.

mod command;
mod context;
mod history;

pub use command::EditCommand;
pub use context::EditContext;
pub use history::EditHistory;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityKind, TypedId};
    use crate::field::FieldValue;
    use crate::schedule::Schedule;

    /// Helper: create a schedule with one panel and return (schedule, panel_uuid).
    fn schedule_with_panel() -> (Schedule, uuid::NonNilUuid) {
        let mut schedule = Schedule::new();
        let panel_id = crate::entity::panel::PanelBuilder::new()
            .with_uid("TST-001".to_string())
            .with_name("Test Panel".to_string())
            .build(&mut schedule)
            .expect("build panel");
        let uuid = TypedId::non_nil_uuid(&panel_id);
        (schedule, uuid)
    }

    fn make_uuid(b: u8) -> uuid::NonNilUuid {
        // Safety: byte array is non-zero
        unsafe {
            uuid::NonNilUuid::new_unchecked(uuid::Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, b,
            ]))
        }
    }

    #[test]
    fn test_update_field_apply_and_undo() {
        let (mut schedule, uuid) = schedule_with_panel();

        // Read initial value
        let initial = schedule
            .read_field_value(EntityKind::Panel, uuid, "title")
            .expect("read")
            .expect("has value");
        assert_eq!(initial, FieldValue::String("Test Panel".to_string()));

        // Create and apply an update command
        let cmd = EditCommand::update_field(
            &schedule,
            EntityKind::Panel,
            uuid,
            "title",
            FieldValue::String("Updated Title".to_string()),
        )
        .expect("create cmd");

        cmd.apply(&mut schedule).expect("apply");

        let updated = schedule
            .read_field_value(EntityKind::Panel, uuid, "title")
            .expect("read")
            .expect("has value");
        assert_eq!(updated, FieldValue::String("Updated Title".to_string()));

        // Undo
        cmd.undo(&mut schedule).expect("undo");

        let restored = schedule
            .read_field_value(EntityKind::Panel, uuid, "title")
            .expect("read")
            .expect("has value");
        assert_eq!(restored, FieldValue::String("Test Panel".to_string()));
    }

    #[test]
    fn test_compound_apply_and_undo() {
        let (mut schedule, uuid) = schedule_with_panel();

        let cmd1 = EditCommand::update_field(
            &schedule,
            EntityKind::Panel,
            uuid,
            "title",
            FieldValue::String("Title A".to_string()),
        )
        .expect("cmd1");

        // Apply cmd1 first so cmd2 sees the intermediate state
        cmd1.apply(&mut schedule).expect("apply cmd1");

        let cmd2 = EditCommand::update_field(
            &schedule,
            EntityKind::Panel,
            uuid,
            "title",
            FieldValue::String("Title B".to_string()),
        )
        .expect("cmd2");

        // Undo cmd1 to restore original before building compound
        cmd1.undo(&mut schedule).expect("undo cmd1");

        let compound = EditCommand::compound("batch update", vec![cmd1, cmd2]);

        compound.apply(&mut schedule).expect("apply compound");

        let value = schedule
            .read_field_value(EntityKind::Panel, uuid, "title")
            .expect("read")
            .expect("has value");
        assert_eq!(value, FieldValue::String("Title B".to_string()));

        compound.undo(&mut schedule).expect("undo compound");

        let restored = schedule
            .read_field_value(EntityKind::Panel, uuid, "title")
            .expect("read")
            .expect("has value");
        assert_eq!(restored, FieldValue::String("Test Panel".to_string()));
    }

    #[test]
    fn test_edit_context_execute_undo_redo() {
        let (schedule, uuid) = schedule_with_panel();
        let mut ctx = EditContext::new(schedule);

        assert!(!ctx.can_undo());
        assert!(!ctx.can_redo());

        let cmd = EditCommand::update_field(
            ctx.schedule(),
            EntityKind::Panel,
            uuid,
            "title",
            FieldValue::String("New Title".to_string()),
        )
        .expect("create cmd");

        ctx.execute(cmd).expect("execute");

        assert!(ctx.can_undo());
        assert!(!ctx.can_redo());

        let value = ctx
            .schedule()
            .read_field_value(EntityKind::Panel, uuid, "title")
            .expect("read")
            .expect("has value");
        assert_eq!(value, FieldValue::String("New Title".to_string()));

        // Undo
        assert!(ctx.undo().expect("undo"));
        assert!(!ctx.can_undo());
        assert!(ctx.can_redo());

        let restored = ctx
            .schedule()
            .read_field_value(EntityKind::Panel, uuid, "title")
            .expect("read")
            .expect("has value");
        assert_eq!(restored, FieldValue::String("Test Panel".to_string()));

        // Redo
        assert!(ctx.redo().expect("redo"));
        assert!(ctx.can_undo());
        assert!(!ctx.can_redo());

        let redone = ctx
            .schedule()
            .read_field_value(EntityKind::Panel, uuid, "title")
            .expect("read")
            .expect("has value");
        assert_eq!(redone, FieldValue::String("New Title".to_string()));
    }

    #[test]
    fn test_history_push_clears_redo() {
        let (schedule, uuid) = schedule_with_panel();
        let mut ctx = EditContext::new(schedule);

        let cmd1 = EditCommand::update_field(
            ctx.schedule(),
            EntityKind::Panel,
            uuid,
            "title",
            FieldValue::String("First".to_string()),
        )
        .expect("cmd1");

        ctx.execute(cmd1).expect("exec1");
        ctx.undo().expect("undo");
        assert!(ctx.can_redo());

        // Push a new command — redo stack should be cleared
        let cmd2 = EditCommand::update_field(
            ctx.schedule(),
            EntityKind::Panel,
            uuid,
            "title",
            FieldValue::String("Second".to_string()),
        )
        .expect("cmd2");

        ctx.execute(cmd2).expect("exec2");
        assert!(!ctx.can_redo());
    }

    #[test]
    fn test_edit_history_clear() {
        let mut history = EditHistory::new();

        let cmd = EditCommand::UpdateField {
            kind: EntityKind::Panel,
            uuid: make_uuid(99),
            field_name: "title".to_string(),
            old_value: FieldValue::String("old".to_string()),
            new_value: FieldValue::String("new".to_string()),
        };

        history.push(cmd);
        assert_eq!(history.undo_count(), 1);

        history.clear();
        assert_eq!(history.undo_count(), 0);
        assert_eq!(history.redo_count(), 0);
    }

    #[test]
    fn test_without_history_mode() {
        let (schedule, uuid) = schedule_with_panel();
        let mut ctx = EditContext::without_history(schedule);

        assert!(!ctx.can_undo());

        let cmd = EditCommand::update_field(
            ctx.schedule(),
            EntityKind::Panel,
            uuid,
            "title",
            FieldValue::String("Fire and Forget".to_string()),
        )
        .expect("create cmd");

        ctx.execute(cmd).expect("execute");

        // Command applied but no history
        assert!(!ctx.can_undo());

        let value = ctx
            .schedule()
            .read_field_value(EntityKind::Panel, uuid, "title")
            .expect("read")
            .expect("has value");
        assert_eq!(value, FieldValue::String("Fire and Forget".to_string()));
    }

    #[test]
    fn test_update_nonexistent_entity() {
        let schedule = Schedule::new();
        let fake_uuid = make_uuid(42);

        let result = EditCommand::update_field(
            &schedule,
            EntityKind::Panel,
            fake_uuid,
            "title",
            FieldValue::String("nope".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_nonexistent_field() {
        let (schedule, uuid) = schedule_with_panel();

        let result = EditCommand::update_field(
            &schedule,
            EntityKind::Panel,
            uuid,
            "nonexistent_field",
            FieldValue::String("nope".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_command_description() {
        let cmd = EditCommand::UpdateField {
            kind: EntityKind::Presenter,
            uuid: make_uuid(77),
            field_name: "name".to_string(),
            old_value: FieldValue::None,
            new_value: FieldValue::String("Alice".to_string()),
        };
        assert_eq!(cmd.description(), "Update Presenter.name");

        let compound = EditCommand::compound("Add presenter", vec![cmd]);
        assert_eq!(compound.description(), "Add presenter (1 sub-commands)");
    }

    // ------------------------------------------------------------------
    // CreateEntity / RemoveEntity
    // ------------------------------------------------------------------

    #[test]
    fn test_create_entity_apply_and_undo() {
        let (schedule, uuid) = schedule_with_panel();

        // Capture a snapshot of the existing panel, then remove it so we can
        // test re-creation via the command.
        let snapshot = schedule
            .snapshot_entity(EntityKind::Panel, uuid)
            .expect("snapshot");

        let mut schedule2 = Schedule::new(); // fresh schedule — panel not present
        let cmd = EditCommand::create_entity(snapshot);

        // Apply: panel should now exist
        cmd.apply(&mut schedule2).expect("apply create");
        assert!(schedule2
            .snapshot_entity(EntityKind::Panel, uuid)
            .is_some());

        // Undo: panel should be gone again
        cmd.undo(&mut schedule2).expect("undo create");
        assert!(schedule2
            .snapshot_entity(EntityKind::Panel, uuid)
            .is_none());
    }

    #[test]
    fn test_remove_entity_apply_and_undo() {
        let (mut schedule, uuid) = schedule_with_panel();

        let cmd =
            EditCommand::remove_entity(&schedule, EntityKind::Panel, uuid).expect("build cmd");

        // Apply: panel should be gone
        cmd.apply(&mut schedule).expect("apply remove");
        assert!(schedule.snapshot_entity(EntityKind::Panel, uuid).is_none());

        // Undo: panel should be back
        cmd.undo(&mut schedule).expect("undo remove");
        assert!(schedule.snapshot_entity(EntityKind::Panel, uuid).is_some());
    }

    #[test]
    fn test_remove_entity_missing_returns_none() {
        let schedule = Schedule::new();
        let cmd = EditCommand::remove_entity(&schedule, EntityKind::Panel, make_uuid(99));
        assert!(cmd.is_none());
    }

    #[test]
    fn test_create_entity_description() {
        let (schedule, uuid) = schedule_with_panel();
        let snapshot = schedule
            .snapshot_entity(EntityKind::Panel, uuid)
            .expect("snapshot");
        let cmd = EditCommand::create_entity(snapshot);
        assert!(cmd.description().starts_with("Create Panel"));
    }

    #[test]
    fn test_remove_entity_description() {
        let (schedule, uuid) = schedule_with_panel();
        let cmd =
            EditCommand::remove_entity(&schedule, EntityKind::Panel, uuid).expect("build cmd");
        assert!(cmd.description().starts_with("Remove Panel"));
    }

    // ------------------------------------------------------------------
    // Dirty state tracking
    // ------------------------------------------------------------------

    #[test]
    fn test_dirty_state_initially_clean() {
        let (schedule, _) = schedule_with_panel();
        let ctx = EditContext::new(schedule);
        assert!(!ctx.is_dirty());
    }

    #[test]
    fn test_dirty_set_after_execute() {
        let (schedule, uuid) = schedule_with_panel();
        let mut ctx = EditContext::new(schedule);

        let cmd = EditCommand::update_field(
            ctx.schedule(),
            EntityKind::Panel,
            uuid,
            "title",
            FieldValue::String("Dirty".to_string()),
        )
        .expect("cmd");

        ctx.execute(cmd).expect("execute");
        assert!(ctx.is_dirty());
    }

    #[test]
    fn test_mark_clean_clears_dirty() {
        let (schedule, uuid) = schedule_with_panel();
        let mut ctx = EditContext::new(schedule);

        let cmd = EditCommand::update_field(
            ctx.schedule(),
            EntityKind::Panel,
            uuid,
            "title",
            FieldValue::String("Dirty".to_string()),
        )
        .expect("cmd");

        ctx.execute(cmd).expect("execute");
        assert!(ctx.is_dirty());

        ctx.mark_clean();
        assert!(!ctx.is_dirty());
    }

    #[test]
    fn test_dirty_set_after_execute_in_without_history_mode() {
        let (schedule, uuid) = schedule_with_panel();
        let mut ctx = EditContext::without_history(schedule);

        let cmd = EditCommand::update_field(
            ctx.schedule(),
            EntityKind::Panel,
            uuid,
            "title",
            FieldValue::String("Dirty".to_string()),
        )
        .expect("cmd");

        ctx.execute(cmd).expect("execute");
        assert!(ctx.is_dirty());
    }

    #[test]
    fn test_failed_execute_does_not_set_dirty() {
        let schedule = Schedule::new();
        let mut ctx = EditContext::new(schedule);

        // Try to update a non-existent entity — should fail without dirtying.
        let cmd = EditCommand::UpdateField {
            kind: EntityKind::Panel,
            uuid: make_uuid(99),
            field_name: "title".to_string(),
            old_value: FieldValue::None,
            new_value: FieldValue::String("nope".to_string()),
        };

        assert!(ctx.execute(cmd).is_err());
        assert!(!ctx.is_dirty());
    }
}
