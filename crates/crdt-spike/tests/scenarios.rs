/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CRDT spike scenarios for FEATURE-022.
//!
//! Each scenario validates one invariant required of the future
//! `schedule-core::crdt` helper module. Scenarios map 1:1 to the bullet list
//! in `plans/crdt-redesign-fd8f49.md` Phase B.

use automerge::transaction::Transactable;
use automerge::{ObjType, ReadDoc, Value, ROOT};
use chrono::Duration;
use crdt_spike::{panel_type_spike_fields, panel_type_type_name, parse_ndt, CrdtDoc, SpikeField};
use schedule_core::entity::RuntimeEntityId;
use schedule_core::value::{
    FieldCardinality, FieldType, FieldTypeItem, FieldValue, FieldValueItem,
};
use uuid::{NonNilUuid, Uuid};

const PANEL: &str = "panel";
const PANEL_TYPE: &str = "panel_type";
const PRESENTER: &str = "presenter";

fn fresh_uuid() -> Uuid {
    Uuid::now_v7()
}

fn presenter_id(u: Uuid) -> FieldValueItem {
    let nn = NonNilUuid::new(u).expect("non-nil uuid in test");
    // SAFETY: tests fabricate type-name/UUID pairs; no registry is consulted.
    let rid = unsafe { RuntimeEntityId::from_uuid(nn, PRESENTER) };
    FieldValueItem::EntityIdentifier(rid)
}

// ── Scenario 1: every FieldValue variant round-trips ───────────────────────

#[test]
fn scenario_1_scalar_variants_roundtrip() {
    let mut doc = CrdtDoc::new();
    let uuid = fresh_uuid();

    let cases: &[(SpikeField, FieldValue)] = &[
        (
            SpikeField::scalar("s_string", FieldTypeItem::String),
            FieldValue::Single(FieldValueItem::String("hello".into())),
        ),
        (
            SpikeField::scalar("s_int", FieldTypeItem::Integer),
            FieldValue::Single(FieldValueItem::Integer(-42)),
        ),
        (
            SpikeField::scalar("s_float", FieldTypeItem::Float),
            FieldValue::Single(FieldValueItem::Float(1.5)),
        ),
        (
            SpikeField::scalar("s_bool", FieldTypeItem::Boolean),
            FieldValue::Single(FieldValueItem::Boolean(true)),
        ),
        (
            SpikeField::scalar("s_dt", FieldTypeItem::DateTime),
            FieldValue::Single(FieldValueItem::DateTime(parse_ndt("2026-04-20T12:34:56"))),
        ),
        (
            SpikeField::scalar("s_dur", FieldTypeItem::Duration),
            FieldValue::Single(FieldValueItem::Duration(Duration::minutes(90))),
        ),
        (
            SpikeField::scalar("s_ent", FieldTypeItem::EntityIdentifier(PRESENTER)),
            FieldValue::Single(presenter_id(fresh_uuid())),
        ),
    ];

    for (field, value) in cases {
        doc.write_field(PANEL, uuid, field.name, field.crdt, value)
            .expect("write scalar");
        let got = doc
            .read_field(PANEL, uuid, field.name, field.ty, field.crdt)
            .expect("read scalar")
            .expect("value present");
        assert_eq!(&got, value, "round-trip for {}", field.name);
    }
}

#[test]
fn scenario_1_text_variant_roundtrip() {
    let mut doc = CrdtDoc::new();
    let uuid = fresh_uuid();
    let field = SpikeField::text("description");
    let body = "This is a long-ish prose description.\nIt has multiple lines.";
    let value = FieldValue::Single(FieldValueItem::Text(body.into()));

    doc.write_field(PANEL, uuid, field.name, field.crdt, &value)
        .expect("write text");

    let got = doc
        .read_field(PANEL, uuid, field.name, field.ty, field.crdt)
        .expect("read text")
        .expect("text present");
    assert_eq!(got, value);
}

#[test]
fn scenario_1_list_variant_roundtrip() {
    let mut doc = CrdtDoc::new();
    let panel = fresh_uuid();
    let p1 = fresh_uuid();
    let p2 = fresh_uuid();
    let field = SpikeField::list("presenter_ids", FieldTypeItem::EntityIdentifier(PRESENTER));
    let value = FieldValue::List(vec![presenter_id(p1), presenter_id(p2)]);

    doc.write_field(PANEL, panel, field.name, field.crdt, &value)
        .expect("write list");

    let got = doc
        .read_field(PANEL, panel, field.name, field.ty, field.crdt)
        .expect("read list")
        .expect("list present");
    assert_eq!(got, value);
}

// ── Scenario 2: concurrent writes to different scalars both preserved ──────

#[test]
fn scenario_2_disjoint_scalar_writes_both_preserved() {
    let mut base = CrdtDoc::new();
    let uuid = fresh_uuid();
    base.create_entity(PANEL_TYPE, uuid).unwrap();

    let mut actor_a = base.fork();
    let mut actor_b = base.fork();

    let f_prefix = SpikeField::scalar("prefix", FieldTypeItem::String);
    let f_color = SpikeField::scalar("color", FieldTypeItem::String);

    actor_a
        .write_field(
            PANEL_TYPE,
            uuid,
            f_prefix.name,
            f_prefix.crdt,
            &FieldValue::Single(FieldValueItem::String("WS".into())),
        )
        .unwrap();
    actor_b
        .write_field(
            PANEL_TYPE,
            uuid,
            f_color.name,
            f_color.crdt,
            &FieldValue::Single(FieldValueItem::String("#ff0066".into())),
        )
        .unwrap();

    actor_a.merge(&mut actor_b).unwrap();

    let prefix = actor_a
        .read_field(PANEL_TYPE, uuid, f_prefix.name, f_prefix.ty, f_prefix.crdt)
        .unwrap()
        .unwrap();
    let color = actor_a
        .read_field(PANEL_TYPE, uuid, f_color.name, f_color.ty, f_color.crdt)
        .unwrap()
        .unwrap();
    assert_eq!(
        prefix,
        FieldValue::Single(FieldValueItem::String("WS".into()))
    );
    assert_eq!(
        color,
        FieldValue::Single(FieldValueItem::String("#ff0066".into()))
    );
}

// ── Scenario 3: concurrent writes to same scalar → deterministic LWW ───────

#[test]
fn scenario_3_same_scalar_converges() {
    let mut base = CrdtDoc::new();
    let uuid = fresh_uuid();
    base.create_entity(PANEL_TYPE, uuid).unwrap();

    let mut a = base.fork();
    let mut b = base.fork();

    let f = SpikeField::scalar("prefix", FieldTypeItem::String);
    a.write_field(
        PANEL_TYPE,
        uuid,
        f.name,
        f.crdt,
        &FieldValue::Single(FieldValueItem::String("AAA".into())),
    )
    .unwrap();
    b.write_field(
        PANEL_TYPE,
        uuid,
        f.name,
        f.crdt,
        &FieldValue::Single(FieldValueItem::String("BBB".into())),
    )
    .unwrap();

    let mut merged_ab = a.fork();
    merged_ab.merge(&mut b.fork()).unwrap();
    let mut merged_ba = b.fork();
    merged_ba.merge(&mut a.fork()).unwrap();

    let v_ab = merged_ab
        .read_field(PANEL_TYPE, uuid, f.name, f.ty, f.crdt)
        .unwrap()
        .unwrap();
    let v_ba = merged_ba
        .read_field(PANEL_TYPE, uuid, f.name, f.ty, f.crdt)
        .unwrap()
        .unwrap();
    assert_eq!(v_ab, v_ba, "LWW must converge regardless of merge order");
    assert!(
        matches!(&v_ab, FieldValue::Single(FieldValueItem::String(s)) if s == "AAA" || s == "BBB"),
        "converged value should be one of the two concurrent writes, got {v_ab:?}"
    );
}

// ── Scenario 4: concurrent text splices merge (RGA) ────────────────────────

#[test]
fn scenario_4_concurrent_text_rga_merges() {
    // Set up a shared starting point with a description.
    let mut base = CrdtDoc::new();
    let panel = fresh_uuid();
    let f = SpikeField::text("description");
    let initial = "Hello world.";
    base.write_field(
        PANEL,
        panel,
        f.name,
        f.crdt,
        &FieldValue::Single(FieldValueItem::Text(initial.into())),
    )
    .unwrap();

    let mut a = base.fork();
    let mut b = base.fork();

    // Actor A prepends "[Edit] ", Actor B appends " Goodbye." concurrently.
    // Both use splice_text directly to get character-granular merges.
    let obj_a = text_obj(&mut a, panel, f.name);
    a.inner.splice_text(&obj_a, 0, 0, "[Edit] ").unwrap();

    let obj_b = text_obj(&mut b, panel, f.name);
    let len_b = b.inner.length(&obj_b);
    b.inner.splice_text(&obj_b, len_b, 0, " Goodbye.").unwrap();

    a.merge(&mut b).unwrap();

    let merged = a
        .read_field(PANEL, panel, f.name, f.ty, f.crdt)
        .unwrap()
        .unwrap();
    let s = match merged {
        FieldValue::Single(FieldValueItem::Text(s)) => s,
        other => panic!("expected Text, got {other:?}"),
    };
    assert!(s.starts_with("[Edit] "), "A's prepend preserved, got {s:?}");
    assert!(
        s.contains("Hello world."),
        "original body preserved, got {s:?}"
    );
    assert!(s.ends_with("Goodbye."), "B's append preserved, got {s:?}");
}

fn text_obj(doc: &mut CrdtDoc, uuid: Uuid, field: &str) -> automerge::ObjId {
    // Resolve the Text object id through the nested entities map.
    let entities = match doc.inner.get(&ROOT, "entities").unwrap().unwrap() {
        (Value::Object(ObjType::Map), id) => id,
        other => panic!("entities map missing: {other:?}"),
    };
    let type_map = match doc.inner.get(&entities, PANEL).unwrap().unwrap() {
        (Value::Object(ObjType::Map), id) => id,
        other => panic!("type map missing: {other:?}"),
    };
    let entity = match doc.inner.get(&type_map, uuid.to_string()).unwrap().unwrap() {
        (Value::Object(ObjType::Map), id) => id,
        other => panic!("entity map missing: {other:?}"),
    };
    match doc.inner.get(&entity, field).unwrap().unwrap() {
        (Value::Object(ObjType::Text), id) => id,
        other => panic!("text obj missing: {other:?}"),
    }
}

// ── Scenario 5: concurrent list pushes both land ───────────────────────────

#[test]
fn scenario_5_concurrent_list_adds_both_present() {
    let mut base = CrdtDoc::new();
    let panel = fresh_uuid();
    let field = SpikeField::list("presenter_ids", FieldTypeItem::EntityIdentifier(PRESENTER));
    // Initialize the list.
    base.write_field(
        PANEL,
        panel,
        field.name,
        field.crdt,
        &FieldValue::List(vec![]),
    )
    .unwrap();

    let mut a = base.fork();
    let mut b = base.fork();

    let p_a = fresh_uuid();
    let p_b = fresh_uuid();
    a.list_push(PANEL, panel, field.name, &presenter_id(p_a))
        .unwrap();
    b.list_push(PANEL, panel, field.name, &presenter_id(p_b))
        .unwrap();

    a.merge(&mut b).unwrap();

    let list = a
        .read_field(PANEL, panel, field.name, field.ty, field.crdt)
        .unwrap()
        .unwrap();
    let items = match list {
        FieldValue::List(v) => v,
        other => panic!("expected List, got {other:?}"),
    };
    assert_eq!(items.len(), 2, "both concurrent adds preserved");
    assert!(items.contains(&presenter_id(p_a)));
    assert!(items.contains(&presenter_id(p_b)));
}

// ── Scenario 6: concurrent add on one side, remove-same on other → add wins ─

#[test]
fn scenario_6_concurrent_add_wins_over_unobserved_remove() {
    let mut base = CrdtDoc::new();
    let panel = fresh_uuid();
    let field = SpikeField::list("presenter_ids", FieldTypeItem::EntityIdentifier(PRESENTER));
    base.write_field(
        PANEL,
        panel,
        field.name,
        field.crdt,
        &FieldValue::List(vec![]),
    )
    .unwrap();

    let mut a = base.fork();
    let mut b = base.fork();

    let p_x = fresh_uuid();

    // A adds X.
    a.list_push(PANEL, panel, field.name, &presenter_id(p_x))
        .unwrap();
    // B has not observed A's add and attempts to remove X (no-op on B side).
    b.list_remove_id(PANEL, panel, field.name, p_x).unwrap();

    a.merge(&mut b).unwrap();

    let list = a
        .read_field(PANEL, panel, field.name, field.ty, field.crdt)
        .unwrap()
        .unwrap();
    let items = match list {
        FieldValue::List(v) => v,
        other => panic!("expected List, got {other:?}"),
    };
    assert!(
        items.contains(&presenter_id(p_x)),
        "A's add must win over B's unobserved remove; got {items:?}"
    );
}

// ── Scenario 7: full round-trip through save/load ──────────────────────────

#[test]
fn scenario_7_save_load_preserves_panel_type_state() {
    let mut doc = CrdtDoc::new();
    let uuid = fresh_uuid();
    let fields = panel_type_spike_fields();
    let values: [FieldValue; 5] = [
        FieldValue::Single(FieldValueItem::String("WS".into())),
        FieldValue::Single(FieldValueItem::String("workshop".into())),
        FieldValue::Single(FieldValueItem::Boolean(false)),
        FieldValue::Single(FieldValueItem::String("#336699".into())),
        FieldValue::Single(FieldValueItem::Integer(10)),
    ];

    for (field, value) in fields.iter().zip(values.iter()) {
        doc.write_field(panel_type_type_name(), uuid, field.name, field.crdt, value)
            .unwrap();
    }

    let bytes = doc.save();
    let loaded = CrdtDoc::load(&bytes).unwrap();

    for (field, value) in fields.iter().zip(values.iter()) {
        let got = loaded
            .read_field(
                panel_type_type_name(),
                uuid,
                field.name,
                field.ty,
                field.crdt,
            )
            .unwrap()
            .unwrap();
        assert_eq!(&got, value, "round-trip after save/load: {}", field.name);
    }

    assert_eq!(loaded.list_entities(panel_type_type_name()), vec![uuid]);
}

// ── Bonus: soft-delete hides entity from list_entities ─────────────────────

#[test]
fn bonus_soft_delete_filters_list() {
    let mut doc = CrdtDoc::new();
    let kept = fresh_uuid();
    let gone = fresh_uuid();
    doc.create_entity(PANEL_TYPE, kept).unwrap();
    doc.create_entity(PANEL_TYPE, gone).unwrap();
    doc.soft_delete(PANEL_TYPE, gone, true).unwrap();

    let alive = doc.list_entities(PANEL_TYPE);
    assert_eq!(alive, vec![kept]);
    assert!(doc.is_deleted(PANEL_TYPE, gone));
    assert!(!doc.is_deleted(PANEL_TYPE, kept));
}

// Ensure FieldCardinality::Optional is visible (keeps `use` statements honest
// even if future scenarios don't touch it directly).
#[allow(dead_code)]
const _OPTIONAL_CARDINALITY: FieldCardinality = FieldCardinality::Optional;
#[allow(dead_code)]
const _FIELD_TYPE_SAMPLE: FieldType = FieldType(FieldCardinality::Single, FieldTypeItem::String);
