/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Integration tests for the heads-based undo/redo system.

use schedule_core::edit::builder::build_entity;
use schedule_core::edit::command::{add_entity_cmd, EditCommand};
use schedule_core::edit::context::EditContext;
use schedule_core::edit::EditError;
use schedule_core::entity::{RuntimeEntityId, UuidPreference};
use schedule_core::field::set::FieldUpdate;
use schedule_core::field_value;
use schedule_core::schedule::Schedule;
use schedule_core::tables::panel_type::PanelTypeEntityType;

fn make_panel_type_in_context() -> (EditContext, RuntimeEntityId) {
    let mut sched = Schedule::default();
    let id = build_entity::<PanelTypeEntityType>(
        &mut sched,
        UuidPreference::GenerateNew,
        vec![
            FieldUpdate::set("prefix", "GP"),
            FieldUpdate::set("panel_kind", "Guest Panel"),
        ],
    )
    .expect("build_entity succeeded");
    let rid = id.into();
    let ctx = EditContext::new(sched);
    (ctx, rid)
}

// ── UpdateField ──────────────────────────────────────────────────────────

#[test]
fn update_field_applies_and_undoes() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let cmd = ctx
        .update_field_cmd(entity, "prefix", field_value!("AA"))
        .expect("cmd built");
    ctx.apply(cmd, "update prefix").expect("apply succeeded");

    let prefix = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(entity.try_into().expect("typed id"))
        .expect("entity present")
        .data
        .prefix
        .clone();
    assert_eq!(prefix, "AA");

    ctx.undo().expect("undo succeeded");

    let prefix_after_undo = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(entity.try_into().expect("typed id"))
        .expect("entity present")
        .data
        .prefix
        .clone();
    assert_eq!(prefix_after_undo, "GP");
}

#[test]
fn update_field_redo_reapplies() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let cmd = ctx
        .update_field_cmd(entity, "prefix", field_value!("BB"))
        .expect("cmd built");
    ctx.apply(cmd, "update prefix").expect("apply");
    ctx.undo().expect("undo");
    ctx.redo().expect("redo");

    let prefix = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(entity.try_into().expect("typed id"))
        .expect("entity present")
        .data
        .prefix
        .clone();
    assert_eq!(prefix, "BB");
}

// ── Undo clears redo stack ───────────────────────────────────────────────

#[test]
fn apply_after_undo_clears_redo() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let cmd1 = ctx
        .update_field_cmd(entity, "prefix", field_value!("C1"))
        .unwrap();
    ctx.apply(cmd1, "update prefix").unwrap();
    ctx.undo().unwrap();

    assert_eq!(ctx.redo_depth(), 1);

    let cmd2 = ctx
        .update_field_cmd(entity, "prefix", field_value!("C2"))
        .unwrap();
    ctx.apply(cmd2, "update prefix").unwrap();

    assert_eq!(ctx.redo_depth(), 0, "redo stack should be cleared");
}

// ── AddEntity / RemoveEntity ─────────────────────────────────────────────

#[test]
fn add_entity_undo_removes_it() {
    let mut sched = Schedule::default();
    let id = build_entity::<PanelTypeEntityType>(
        &mut sched,
        UuidPreference::GenerateNew,
        vec![
            FieldUpdate::set("prefix", "GP"),
            FieldUpdate::set("panel_kind", "Guest Panel"),
        ],
    )
    .expect("build_entity");
    let rid: RuntimeEntityId = id.into();
    let add_cmd = add_entity_cmd(&sched, rid).expect("add_entity_cmd");

    // Remove the entity to tombstone it before testing add/undo
    sched.remove_entity::<PanelTypeEntityType>(id);

    let mut ctx = EditContext::new(sched);
    ctx.apply(add_cmd, "add panel type").expect("apply add");
    assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 1);

    ctx.undo().expect("undo add");
    assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 0);
}

#[test]
fn add_entity_undo_then_redo_restores_same_uuid() {
    let mut sched = Schedule::default();
    let id = build_entity::<PanelTypeEntityType>(
        &mut sched,
        UuidPreference::GenerateNew,
        vec![
            FieldUpdate::set("prefix", "GP"),
            FieldUpdate::set("panel_kind", "Guest Panel"),
        ],
    )
    .expect("build_entity");
    let rid: RuntimeEntityId = id.into();
    let add_cmd = add_entity_cmd(&sched, rid).expect("add_entity_cmd");

    // Remove the entity to tombstone it before testing add/undo/redo
    sched.remove_entity::<PanelTypeEntityType>(id);

    let mut ctx = EditContext::new(sched);
    ctx.apply(add_cmd, "add panel type").expect("apply");
    ctx.undo().expect("undo");
    ctx.redo().expect("redo");

    assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 1);
    let typed = rid.try_into().expect("typed id");
    let data = ctx.schedule().get_internal::<PanelTypeEntityType>(typed);
    assert!(data.is_some(), "entity restored with same UUID");
    assert_eq!(data.unwrap().data.prefix, "GP");
}

#[test]
fn remove_entity_undo_restores_entity() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let remove_cmd = ctx.remove_entity_cmd(entity).expect("remove_entity_cmd");
    ctx.apply(remove_cmd, "remove panel type")
        .expect("apply remove");
    assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 0);

    ctx.undo().expect("undo remove");
    assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 1);

    let typed = entity.try_into().expect("typed id");
    let data = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(typed)
        .expect("entity restored");
    assert_eq!(data.data.prefix, "GP");
}

// ── BatchEdit ────────────────────────────────────────────────────────────

#[test]
fn batch_edit_applies_atomically_and_undoes_in_reverse() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let cmd1 = ctx
        .update_field_cmd(entity, "prefix", field_value!("B1"))
        .unwrap();
    let cmd2 = ctx
        .update_field_cmd(entity, "panel_kind", field_value!("Workshop"))
        .unwrap();
    let batch = EditCommand::BatchEdit(vec![cmd1, cmd2]);
    ctx.apply(batch, "batch update").expect("apply batch");

    let data = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(entity.try_into().unwrap())
        .unwrap();
    assert_eq!(data.data.prefix, "B1");
    assert_eq!(data.data.panel_kind, "Workshop");

    ctx.undo().expect("undo batch");

    let data_after = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(entity.try_into().unwrap())
        .unwrap();
    assert_eq!(data_after.data.prefix, "GP");
    assert_eq!(data_after.data.panel_kind, "Guest Panel");
}

#[test]
fn batch_edit_redo_reapplies_all() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let cmd1 = ctx
        .update_field_cmd(entity, "prefix", field_value!("C1"))
        .unwrap();
    let cmd2 = ctx
        .update_field_cmd(entity, "panel_kind", field_value!("Concert"))
        .unwrap();
    ctx.apply(EditCommand::BatchEdit(vec![cmd1, cmd2]), "batch update")
        .unwrap();
    ctx.undo().unwrap();
    ctx.redo().unwrap();

    let data = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(entity.try_into().unwrap())
        .unwrap();
    assert_eq!(data.data.prefix, "C1");
    assert_eq!(data.data.panel_kind, "Concert");
}

// ── Dirty state ──────────────────────────────────────────────────────────

#[test]
fn dirty_state_tracks_correctly() {
    let (mut ctx, entity) = make_panel_type_in_context();

    assert!(!ctx.is_dirty());

    let cmd = ctx
        .update_field_cmd(entity, "prefix", field_value!("X"))
        .unwrap();
    ctx.apply(cmd, "update prefix").unwrap();
    assert!(ctx.is_dirty());

    ctx.mark_clean();
    assert!(!ctx.is_dirty());

    ctx.undo().unwrap();
    assert!(!ctx.is_dirty());
}

// ── History bounds ───────────────────────────────────────────────────────

#[test]
fn history_respects_max_depth() {
    let sched = Schedule::default();
    let mut ctx = EditContext::with_history_depth(sched, 3);
    let mut sched2 = Schedule::default();
    for i in 0u8..5 {
        let id = build_entity::<PanelTypeEntityType>(
            &mut sched2,
            UuidPreference::GenerateNew,
            vec![
                FieldUpdate::set("prefix", format!("P{i}")),
                FieldUpdate::set("panel_kind", "Kind"),
            ],
        )
        .expect("build");
        let rid: RuntimeEntityId = id.into();
        let add_cmd = add_entity_cmd(&sched2, rid).expect("add cmd");
        let _ = ctx.apply(add_cmd, "add");
    }
    assert_eq!(
        ctx.undo_depth(),
        3,
        "undo stack should not exceed max_depth"
    );
}

// ── Error cases ─────────────────────────────────────────────────────────

#[test]
fn undo_on_empty_stack_returns_error() {
    let sched = Schedule::default();
    let mut ctx = EditContext::new(sched);
    assert!(matches!(ctx.undo(), Err(EditError::NothingToUndo)));
}

#[test]
fn redo_on_empty_stack_returns_error() {
    let sched = Schedule::default();
    let mut ctx = EditContext::new(sched);
    assert!(matches!(ctx.redo(), Err(EditError::NothingToRedo)));
}

// ── touch_modified / metadata ────────────────────────────────────────────

#[test]
fn touch_modified_sets_timestamp() {
    let mut sched = Schedule::default();
    assert!(sched.metadata.modified_at.is_none());
    sched.touch_modified();
    assert!(sched.metadata.modified_at.is_some());
}

#[test]
fn touch_modified_does_not_change_version() {
    let mut sched = Schedule::default();
    assert_eq!(sched.metadata.version, 0);
    sched.touch_modified();
    assert_eq!(sched.metadata.version, 0);
}

#[test]
fn apply_calls_touch_modified() {
    let sched = Schedule::default();
    let mut ctx = EditContext::new(sched);
    assert!(ctx.schedule().metadata.modified_at.is_none());

    let id = schedule_core::entity::EntityId::<PanelTypeEntityType>::generate();
    let rid: RuntimeEntityId = id.into();
    let cmd = EditCommand::AddEntity {
        entity: rid,
        fields: vec![
            ("prefix", field_value!("TS")),
            ("panel_kind", field_value!("Test")),
        ],
    };
    ctx.apply(cmd, "add panel type").expect("apply succeeded");

    assert!(
        ctx.schedule().metadata.modified_at.is_some(),
        "apply should call touch_modified"
    );
}

#[test]
fn undo_calls_touch_modified() {
    let (mut ctx, entity) = make_panel_type_in_context();
    // Reset modified_at so we can detect the change from undo
    ctx.schedule_mut().metadata.modified_at = None;

    let cmd = ctx
        .update_field_cmd(entity, "prefix", field_value!("ZZ"))
        .expect("cmd built");
    ctx.apply(cmd, "update prefix").expect("apply");
    // Reset again after apply
    ctx.schedule_mut().metadata.modified_at = None;

    ctx.undo().expect("undo");
    assert!(
        ctx.schedule().metadata.modified_at.is_some(),
        "undo should call touch_modified"
    );
}

#[test]
fn redo_calls_touch_modified() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let cmd = ctx
        .update_field_cmd(entity, "prefix", field_value!("ZZ"))
        .expect("cmd built");
    ctx.apply(cmd, "update prefix").expect("apply");
    ctx.undo().expect("undo");
    // Reset before redo
    ctx.schedule_mut().metadata.modified_at = None;

    ctx.redo().expect("redo");
    assert!(
        ctx.schedule().metadata.modified_at.is_some(),
        "redo should call touch_modified"
    );
}

#[test]
fn schedule_mut_allows_stamping_generator() {
    let sched = Schedule::default();
    let mut ctx = EditContext::new(sched);
    ctx.schedule_mut().metadata.generator = "test-tool 1.0".to_string();
    assert_eq!(ctx.schedule().metadata.generator, "test-tool 1.0");
}

// ── Labels ───────────────────────────────────────────────────────────────

#[test]
fn apply_records_undo_label() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let cmd = ctx
        .update_field_cmd(entity, "prefix", field_value!("LB"))
        .unwrap();
    ctx.apply(cmd, "Update prefix").unwrap();

    assert_eq!(ctx.undo_label(), Some("Update prefix"));
    assert_eq!(ctx.redo_label(), None);
}

#[test]
fn undo_transfers_label_to_redo_stack() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let cmd = ctx
        .update_field_cmd(entity, "prefix", field_value!("LB"))
        .unwrap();
    ctx.apply(cmd, "Update prefix").unwrap();
    ctx.undo().unwrap();

    assert_eq!(ctx.undo_label(), None);
    assert_eq!(ctx.redo_label(), Some("Update prefix"));
}

#[test]
fn redo_transfers_label_back_to_undo_stack() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let cmd = ctx
        .update_field_cmd(entity, "prefix", field_value!("LB"))
        .unwrap();
    ctx.apply(cmd, "Update prefix").unwrap();
    ctx.undo().unwrap();
    ctx.redo().unwrap();

    assert_eq!(ctx.undo_label(), Some("Update prefix"));
    assert_eq!(ctx.redo_label(), None);
}

#[test]
fn dynamic_string_label_works() {
    let (mut ctx, entity) = make_panel_type_in_context();
    let label = format!("update {}", "prefix");
    let cmd = ctx
        .update_field_cmd(entity, "prefix", field_value!("DL"))
        .unwrap();
    ctx.apply(cmd, label).unwrap();
    assert_eq!(ctx.undo_label(), Some("update prefix"));
}

// ── run_checkpoint ───────────────────────────────────────────────────────

#[test]
fn run_checkpoint_groups_multiple_writes_as_one_undo_step() {
    let mut sched = Schedule::default();
    // Create two panel types directly on the schedule (not through EditContext).
    let id1 = build_entity::<PanelTypeEntityType>(
        &mut sched,
        UuidPreference::GenerateNew,
        vec![
            FieldUpdate::set("prefix", "A1"),
            FieldUpdate::set("panel_kind", "Kind A"),
        ],
    )
    .unwrap();
    let id2 = build_entity::<PanelTypeEntityType>(
        &mut sched,
        UuidPreference::GenerateNew,
        vec![
            FieldUpdate::set("prefix", "B1"),
            FieldUpdate::set("panel_kind", "Kind B"),
        ],
    )
    .unwrap();
    let _ = (id1, id2);

    let mut ctx = EditContext::new(sched);
    assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 2);

    // Wrap two separate entity additions in a checkpoint — should be ONE undo step.
    let rid1: RuntimeEntityId = id1.into();
    let rid2: RuntimeEntityId = id2.into();
    ctx.run_checkpoint::<_, schedule_core::edit::EditError>("bulk rename", |sched| {
        // Use UpdateField via write-field-fn to rename both entities.
        use schedule_core::entity::EntityUuid;
        let reg = schedule_core::entity::registered_entity_types()
            .find(|r| r.type_name == "panel_type")
            .unwrap();
        (reg.write_field_fn)(sched, rid1.entity_uuid(), "prefix", field_value!("A2")).unwrap();
        (reg.write_field_fn)(sched, rid2.entity_uuid(), "prefix", field_value!("B2")).unwrap();
        Ok(())
    })
    .unwrap();

    assert_eq!(ctx.undo_depth(), 1, "both writes are one undo step");
    assert_eq!(ctx.undo_label(), Some("bulk rename"));

    // Undo should revert both renames at once.
    ctx.undo().unwrap();

    let d1 = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(id1)
        .unwrap();
    let d2 = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(id2)
        .unwrap();
    assert_eq!(d1.data.prefix, "A1");
    assert_eq!(d2.data.prefix, "B1");
}

#[test]
fn run_checkpoint_noop_does_not_push_undo_entry() {
    let sched = Schedule::default();
    let mut ctx = EditContext::new(sched);

    // Closure that reads but does not write — no CRDT changes.
    ctx.run_checkpoint::<_, std::convert::Infallible>("noop", |_sched| Ok(()))
        .unwrap();

    assert_eq!(ctx.undo_depth(), 0, "no-op checkpoint must not push");
    assert!(!ctx.is_dirty());
}

#[test]
fn run_checkpoint_error_does_not_push_undo_entry() {
    let sched = Schedule::default();
    let mut ctx = EditContext::new(sched);

    let result = ctx.run_checkpoint("will fail", |_sched| {
        Err::<(), _>(anyhow::anyhow!("deliberate error"))
    });

    assert!(result.is_err());
    assert_eq!(ctx.undo_depth(), 0, "failed checkpoint must not push");
}

// ── Multiple operations undo/redo sequence ───────────────────────────────

#[test]
fn multi_step_undo_redo_sequence() {
    let (mut ctx, entity) = make_panel_type_in_context();

    // Apply three sequential edits.
    let cmd1 = ctx
        .update_field_cmd(entity, "prefix", field_value!("S1"))
        .unwrap();
    ctx.apply(cmd1, "step 1").unwrap();

    let cmd2 = ctx
        .update_field_cmd(entity, "prefix", field_value!("S2"))
        .unwrap();
    ctx.apply(cmd2, "step 2").unwrap();

    let cmd3 = ctx
        .update_field_cmd(entity, "prefix", field_value!("S3"))
        .unwrap();
    ctx.apply(cmd3, "step 3").unwrap();

    assert_eq!(ctx.undo_depth(), 3);

    // Undo all three.
    ctx.undo().unwrap();
    let prefix = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(entity.try_into().unwrap())
        .unwrap()
        .data
        .prefix
        .clone();
    assert_eq!(prefix, "S2");

    ctx.undo().unwrap();
    let prefix = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(entity.try_into().unwrap())
        .unwrap()
        .data
        .prefix
        .clone();
    assert_eq!(prefix, "S1");

    ctx.undo().unwrap();
    let prefix = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(entity.try_into().unwrap())
        .unwrap()
        .data
        .prefix
        .clone();
    assert_eq!(prefix, "GP");

    // Redo all the way back.
    ctx.redo().unwrap();
    ctx.redo().unwrap();
    ctx.redo().unwrap();
    let prefix = ctx
        .schedule()
        .get_internal::<PanelTypeEntityType>(entity.try_into().unwrap())
        .unwrap()
        .data
        .prefix
        .clone();
    assert_eq!(prefix, "S3");
}
