/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Integration tests for Edit module.

use schedule_core::edit::builder::build_entity;
use schedule_core::edit::command::{add_entity_cmd, EditCommand};
use schedule_core::edit::context::EditContext;
use schedule_core::edit::EditError;
use schedule_core::entity::{RuntimeEntityId, UuidPreference};
use schedule_core::field::set::FieldRef;
use schedule_core::field_value;
use schedule_core::schedule::Schedule;
use schedule_core::tables::panel_type::PanelTypeEntityType;

fn make_panel_type_in_context() -> (EditContext, RuntimeEntityId) {
    let mut sched = Schedule::default();
    let id = build_entity::<PanelTypeEntityType>(
        &mut sched,
        UuidPreference::GenerateNew,
        vec![
            (FieldRef::Name("prefix"), field_value!("GP")),
            (FieldRef::Name("panel_kind"), field_value!("Guest Panel")),
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
    ctx.apply(cmd).expect("apply succeeded");

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
    ctx.apply(cmd).expect("apply");
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
    ctx.apply(cmd1).unwrap();
    ctx.undo().unwrap();

    assert_eq!(ctx.redo_depth(), 1);

    let cmd2 = ctx
        .update_field_cmd(entity, "prefix", field_value!("C2"))
        .unwrap();
    ctx.apply(cmd2).unwrap();

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
            (FieldRef::Name("prefix"), field_value!("GP")),
            (FieldRef::Name("panel_kind"), field_value!("Guest Panel")),
        ],
    )
    .expect("build_entity");
    let rid: RuntimeEntityId = id.into();
    let add_cmd = add_entity_cmd(&sched, rid).expect("add_entity_cmd");

    let mut ctx = EditContext::new(sched);
    ctx.apply(add_cmd).expect("apply add");
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
            (FieldRef::Name("prefix"), field_value!("GP")),
            (FieldRef::Name("panel_kind"), field_value!("Guest Panel")),
        ],
    )
    .expect("build_entity");
    let rid: RuntimeEntityId = id.into();
    let add_cmd = add_entity_cmd(&sched, rid).expect("add_entity_cmd");

    let mut ctx = EditContext::new(sched);
    ctx.apply(add_cmd).expect("apply");
    ctx.undo().expect("undo");
    ctx.redo().expect("redo");

    assert_eq!(ctx.schedule().entity_count::<PanelTypeEntityType>(), 1);
    let typed = rid.try_into().expect("typed id");
    let data = ctx.schedule().get_internal::<PanelTypeEntityType>(typed);
    assert!(data.is_some(), "entity restored with same UUID");
}

#[test]
fn remove_entity_undo_restores_entity() {
    let (mut ctx, entity) = make_panel_type_in_context();

    let remove_cmd = ctx.remove_entity_cmd(entity).expect("remove_entity_cmd");
    ctx.apply(remove_cmd).expect("apply remove");
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
    ctx.apply(batch).expect("apply batch");

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
    ctx.apply(EditCommand::BatchEdit(vec![cmd1, cmd2])).unwrap();
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
    ctx.apply(cmd).unwrap();
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
                (FieldRef::Name("prefix"), field_value!(format!("P{i}"))),
                (FieldRef::Name("panel_kind"), field_value!("Kind")),
            ],
        )
        .expect("build");
        let rid: RuntimeEntityId = id.into();
        let add_cmd = add_entity_cmd(&sched2, rid).expect("add cmd");
        let _ = ctx.apply(add_cmd);
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
