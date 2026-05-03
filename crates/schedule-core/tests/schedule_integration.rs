/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Integration tests for Schedule module.

use schedule_core::entity::{EntityId, EntityUuid, UuidPreference};
use schedule_core::schedule::{
    entity_ids_to_field_value, field_value_to_entity_ids, LoadError, Schedule,
};
use schedule_core::tables::event_room::{
    EventRoomCommonData, EventRoomEntityType, EventRoomInternalData,
};
use schedule_core::tables::hotel_room::{
    HotelRoomCommonData, HotelRoomEntityType, HotelRoomInternalData,
};
use schedule_core::tables::panel::{PanelCommonData, PanelEntityType, PanelId, PanelInternalData};
use schedule_core::tables::panel_type::{
    PanelTypeCommonData, PanelTypeEntityType, PanelTypeInternalData,
};
use schedule_core::tables::presenter::{
    self, PresenterCommonData, PresenterEntityType, PresenterId, PresenterInternalData,
};
use schedule_core::value::time::TimeRange;
use schedule_core::value::uniq_id::PanelUniqId;
use schedule_core::value::FieldTypeItem;
use schedule_core::EntityTyped;

fn make_panel_type() -> (EntityId<PanelTypeEntityType>, PanelTypeInternalData) {
    let id = EntityId::from_preference(UuidPreference::GenerateNew);
    let data = PanelTypeInternalData {
        id,
        data: PanelTypeCommonData {
            prefix: "GP".into(),
            panel_kind: "Guest Panel".into(),
            ..Default::default()
        },
    };
    (id, data)
}

fn make_panel() -> (PanelId, PanelInternalData) {
    let id = EntityId::from_preference(UuidPreference::GenerateNew);
    let data = PanelInternalData {
        id,
        data: PanelCommonData {
            name: "Test Panel".into(),
            ..Default::default()
        },
        code: PanelUniqId::parse("GP001").unwrap(),
        time_slot: TimeRange::Unspecified,
    };
    (id, data)
}

fn make_presenter(name: &str) -> (EntityId<PresenterEntityType>, PresenterInternalData) {
    let id = EntityId::from_preference(UuidPreference::GenerateNew);
    let data = PresenterInternalData {
        id,
        data: PresenterCommonData {
            name: name.into(),
            ..Default::default()
        },
    };
    (id, data)
}

fn make_event_room(name: &str) -> (EntityId<EventRoomEntityType>, EventRoomInternalData) {
    let id = EntityId::from_preference(UuidPreference::GenerateNew);
    let data = EventRoomInternalData {
        id,
        data: EventRoomCommonData {
            room_name: name.into(),
            ..Default::default()
        },
    };
    (id, data)
}

fn make_hotel_room(name: &str) -> (EntityId<HotelRoomEntityType>, HotelRoomInternalData) {
    let id = EntityId::from_preference(UuidPreference::GenerateNew);
    let data = HotelRoomInternalData {
        id,
        data: HotelRoomCommonData {
            hotel_room_name: name.into(),
        },
    };
    (id, data)
}

// ── Entity storage ────────────────────────────────────────────────────────

#[test]
fn insert_and_get_internal() {
    let mut sched = Schedule::new();
    let (id, data) = make_panel_type();
    sched.insert(id, data.clone());
    let got = sched.get_internal(id).unwrap();
    assert_eq!(got.data.prefix, "GP");
}

#[test]
fn get_internal_missing_returns_none() {
    let sched = Schedule::new();
    let (id, _) = make_panel_type();
    assert!(sched.get_internal(id).is_none());
}

#[test]
fn insert_replaces_existing() {
    let mut sched = Schedule::new();
    let (id, mut data) = make_panel_type();
    sched.insert(id, data.clone());
    data.data.prefix = "SP".into();
    sched.insert(id, data);
    assert_eq!(sched.get_internal(id).unwrap().data.prefix, "SP");
}

#[test]
fn entity_count() {
    let mut sched = Schedule::new();
    assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);
    let (id1, d1) = make_panel_type();
    let (id2, d2) = make_panel_type();
    sched.insert(id1, d1);
    sched.insert(id2, d2);
    assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 2);
}

#[test]
fn iter_entities() {
    let mut sched = Schedule::new();
    let (id1, d1) = make_panel_type();
    let (id2, d2) = make_panel_type();
    sched.insert(id1, d1);
    sched.insert(id2, d2);
    let ids: std::collections::HashSet<_> = sched
        .iter_entities::<PanelTypeEntityType>()
        .map(|(id, _)| id)
        .collect();
    assert!(ids.contains(&id1));
    assert!(ids.contains(&id2));
    assert_eq!(ids.len(), 2);
}

#[test]
fn remove_entity_removes_from_storage() {
    let mut sched = Schedule::new();
    let (id, data) = make_panel_type();
    sched.insert(id, data);
    assert!(sched.get_internal(id).is_some());
    sched.remove_entity::<PanelTypeEntityType>(id);
    assert!(sched.get_internal(id).is_none());
}

// ── Identify ──────────────────────────────────────────────────────────────

#[test]
fn identify_returns_correct_type() {
    let mut sched = Schedule::new();
    let (id, data) = make_panel_type();
    sched.insert(id, data);
    let rid = sched.identify(id.entity_uuid()).unwrap();
    assert_eq!(rid.entity_type_name(), "panel_type");
    assert_eq!(rid.entity_uuid(), id.entity_uuid());
}

#[test]
fn identify_missing_uuid_returns_none() {
    let sched = Schedule::new();
    let (id, _) = make_panel_type();
    assert!(sched.identify(id.entity_uuid()).is_none());
}

#[test]
fn identify_distinguishes_types() {
    let mut sched = Schedule::new();
    let (pt_id, pt_data) = make_panel_type();
    let (p_id, p_data) = make_presenter("Alice");
    sched.insert(pt_id, pt_data);
    sched.insert(p_id, p_data);
    let pt_rid = sched.identify(pt_id.entity_uuid()).unwrap();
    let p_rid = sched.identify(p_id.entity_uuid()).unwrap();
    assert_eq!(pt_rid.entity_type_name(), "panel_type");
    assert_eq!(p_rid.entity_type_name(), "presenter");
}

// ── Het edges ─────────────────────────────────────────────────────────────

#[test]
fn het_edge_add_and_query_both_directions() {
    let mut sched = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (pres_id, pres_data) = make_presenter("Alice");
    sched.insert(panel_id, panel_data);
    sched.insert(pres_id, pres_data);

    let edge = schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS;
    sched
        .edge_add(panel_id, edge, std::iter::once(pres_id))
        .unwrap();

    let presenters = sched
        .connected_field_nodes(panel_id, edge)
        .into_iter()
        .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
        .collect::<Vec<PresenterId>>();
    assert_eq!(presenters, vec![pres_id]);

    let panels = sched
        .connected_field_nodes(pres_id, presenter::EDGE_CREDITED_PANELS)
        .into_iter()
        .map(|e| unsafe { PanelId::new_unchecked(e.entity_uuid()) })
        .collect::<Vec<PanelId>>();
    assert_eq!(panels, vec![panel_id]);
}

#[test]
fn het_edge_remove() {
    let mut sched = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (pres_id, pres_data) = make_presenter("Alice");
    sched.insert(panel_id, panel_data);
    sched.insert(pres_id, pres_data);

    let edge = schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS;
    sched
        .edge_add(panel_id, edge, std::iter::once(pres_id))
        .unwrap();
    sched.edge_remove(panel_id, edge, std::iter::once(pres_id));

    assert!(sched.connected_field_nodes(panel_id, edge).is_empty());
    assert!(sched.connected_field_nodes(pres_id, edge).is_empty());
}

#[test]
fn het_edge_set_replaces_all() {
    let mut sched = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (p1_id, p1_data) = make_presenter("Alice");
    let (p2_id, p2_data) = make_presenter("Bob");
    let (p3_id, p3_data) = make_presenter("Carol");
    sched.insert(panel_id, panel_data);
    sched.insert(p1_id, p1_data);
    sched.insert(p2_id, p2_data);
    sched.insert(p3_id, p3_data);

    sched
        .edge_set(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
            vec![p1_id, p2_id],
        )
        .unwrap();
    let mut presenters = sched.connected_field_nodes(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
    );
    presenters.sort_by_key(|id| id.entity_uuid());
    let mut expected: Vec<schedule_core::entity::RuntimeEntityId> =
        vec![p1_id.into(), p2_id.into()];
    expected.sort_by_key(|id| id.entity_uuid());
    assert_eq!(presenters, expected);

    sched
        .edge_set(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
            vec![p3_id],
        )
        .unwrap();
    assert_eq!(
        sched.connected_field_nodes(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
        ),
        vec![p3_id.into()]
    );
    // p1 and p2 no longer link back to panel
    assert!(sched
        .connected_field_nodes(
            p1_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
        )
        .is_empty());
    assert!(sched
        .connected_field_nodes(
            p2_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
        )
        .is_empty());
}

#[test]
fn remove_entity_clears_het_edges() {
    let mut sched = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (pres_id, pres_data) = make_presenter("Alice");
    sched.insert(panel_id, panel_data);
    sched.insert(pres_id, pres_data);
    let edge = schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS;
    sched
        .edge_add(panel_id, edge, std::iter::once(pres_id))
        .unwrap();

    sched.remove_entity::<PanelEntityType>(panel_id);

    // Edge from presenter side should be gone too
    assert!(sched.connected_field_nodes(pres_id, edge).is_empty());
}

// ── EventRoom / HotelRoom heterogeneous edges ─────────────────────────────

#[test]
fn event_room_hotel_room_het_edge() {
    let mut sched = Schedule::new();
    let (room_id, room_data) = make_event_room("Panel 1");
    let (hotel_id, hotel_data) = make_hotel_room("East Hall");
    sched.insert(room_id, room_data);
    sched.insert(hotel_id, hotel_data);

    sched
        .edge_add(
            room_id,
            schedule_core::tables::event_room::EDGE_HOTEL_ROOMS,
            std::iter::once(hotel_id),
        )
        .unwrap();

    let hotels =
        sched.connected_field_nodes(room_id, schedule_core::tables::event_room::EDGE_HOTEL_ROOMS);
    assert_eq!(hotels, vec![hotel_id.into()]);

    // Reverse: hotel_room.event_rooms via connected_field_nodes with HALF_EDGE_EVENT_ROOMS
    let rooms = sched.connected_field_nodes(
        hotel_id,
        schedule_core::tables::hotel_room::EDGE_EVENT_ROOMS,
    );
    assert_eq!(rooms, vec![room_id.into()]);
}

// ── Homogeneous edges (Presenter → Presenter) ─────────────────────────────

#[test]
fn homogenous_edge_groups_and_members() {
    let mut sched = Schedule::new();
    let (member_id, member_data) = make_presenter("Alice");
    let (group_id, group_data) = make_presenter("The Group");
    sched.insert(member_id, member_data);
    sched.insert(group_id, group_data);

    // member → group (member is in group: use EDGE_GROUPS)
    sched
        .edge_add(member_id, presenter::EDGE_GROUPS, std::iter::once(group_id))
        .unwrap();

    // groups of member: use EDGE_GROUPS to query member's groups
    let groups = sched.connected_field_nodes(member_id, presenter::EDGE_GROUPS);
    assert_eq!(groups, vec![group_id.into()]);

    // members of group: use EDGE_MEMBERS to query group's members
    let members = sched.connected_field_nodes(group_id, presenter::EDGE_MEMBERS);
    assert_eq!(members, vec![member_id.into()]);
}

#[test]
fn homogenous_edge_remove() {
    let mut sched = Schedule::new();
    let (member_id, member_data) = make_presenter("Alice");
    let (group_id, group_data) = make_presenter("The Group");
    sched.insert(member_id, member_data);
    sched.insert(group_id, group_data);

    sched
        .edge_add(member_id, presenter::EDGE_GROUPS, std::iter::once(group_id))
        .unwrap();
    sched.edge_remove(member_id, presenter::EDGE_GROUPS, std::iter::once(group_id));

    assert!(sched
        .connected_field_nodes(member_id, presenter::EDGE_GROUPS,)
        .is_empty());
    assert!(sched
        .connected_field_nodes(group_id, presenter::EDGE_MEMBERS,)
        .is_empty());
}

#[test]
fn homogenous_edge_set_replaces() {
    let mut sched = Schedule::new();
    let (member_id, member_data) = make_presenter("Alice");
    let (g1_id, g1_data) = make_presenter("Group A");
    let (g2_id, g2_data) = make_presenter("Group B");
    sched.insert(member_id, member_data);
    sched.insert(g1_id, g1_data);
    sched.insert(g2_id, g2_data);

    sched
        .edge_set(member_id, presenter::EDGE_GROUPS, vec![g1_id])
        .unwrap();
    assert_eq!(
        sched.connected_field_nodes(member_id, presenter::EDGE_GROUPS,),
        vec![g1_id.into()]
    );

    sched
        .edge_set(member_id, presenter::EDGE_GROUPS, vec![g2_id])
        .unwrap();
    assert_eq!(
        sched.connected_field_nodes(member_id, presenter::EDGE_GROUPS,),
        vec![g2_id.into()]
    );
    assert!(sched
        .connected_field_nodes(g1_id, presenter::EDGE_MEMBERS,)
        .is_empty());
}

#[test]
fn edge_set_to_sets_members() {
    let mut sched = Schedule::new();
    let (m1_id, m1_data) = make_presenter("Alice");
    let (m2_id, m2_data) = make_presenter("Bob");
    let (g_id, g_data) = make_presenter("The Group");
    sched.insert(m1_id, m1_data);
    sched.insert(m2_id, m2_data);
    sched.insert(g_id, g_data);

    // Set members of group to [m1, m2]
    sched
        .edge_set(g_id, presenter::EDGE_MEMBERS, vec![m1_id, m2_id])
        .unwrap();

    let mut members = sched.connected_field_nodes(g_id, presenter::EDGE_MEMBERS);
    members.sort_by_key(|id| id.entity_uuid());
    let mut expected: Vec<schedule_core::entity::RuntimeEntityId> =
        vec![m1_id.into(), m2_id.into()];
    expected.sort_by_key(|id| id.entity_uuid());
    assert_eq!(members, expected);

    // m1 and m2 should have group in their groups list
    assert_eq!(
        sched.connected_field_nodes(m1_id, presenter::EDGE_GROUPS,),
        vec![g_id.into()]
    );
    assert_eq!(
        sched.connected_field_nodes(m2_id, presenter::EDGE_GROUPS,),
        vec![g_id.into()]
    );

    // Replace with just m1
    sched
        .edge_set(g_id, presenter::EDGE_MEMBERS, vec![m1_id])
        .unwrap();
    assert_eq!(
        sched.connected_field_nodes(g_id, presenter::EDGE_MEMBERS,),
        vec![m1_id.into()]
    );
    assert!(sched
        .connected_field_nodes(m2_id, presenter::EDGE_GROUPS,)
        .is_empty());
}

#[test]
fn remove_entity_clears_homogenous_edges() {
    let mut sched = Schedule::new();
    let (member_id, member_data) = make_presenter("Alice");
    let (group_id, group_data) = make_presenter("The Group");
    sched.insert(member_id, member_data);
    sched.insert(group_id, group_data);
    sched
        .edge_add(member_id, presenter::EDGE_GROUPS, std::iter::once(group_id))
        .unwrap();

    sched.remove_entity::<PresenterEntityType>(member_id);

    // group should no longer see member
    assert!(sched
        .connected_field_nodes(group_id, presenter::EDGE_MEMBERS,)
        .is_empty());
}

// ── entity_ids_to_field_value / field_value_to_entity_ids ─────────────────

#[test]
fn entity_ids_roundtrip_through_field_value() {
    let (id1, _) = make_presenter("Alice");
    let (id2, _) = make_presenter("Bob");
    let ids = vec![id1, id2];
    let fv = entity_ids_to_field_value(ids.clone());
    let back = field_value_to_entity_ids::<PresenterEntityType>(fv).unwrap();
    assert_eq!(back, ids);
}

#[test]
fn field_value_to_entity_ids_wrong_type_is_error() {
    let (room_id, _) = make_event_room("Panel 1");
    let fv = entity_ids_to_field_value(vec![room_id]);
    let result = field_value_to_entity_ids::<PresenterEntityType>(fv);
    assert!(result.is_err());
}

// ── CRDT mirror ──────────────────────────────────────────────────────────

#[test]
fn crdt_mirror_populates_doc_on_insert() {
    use schedule_core::crdt;
    use schedule_core::crdt::CrdtFieldType;
    use schedule_core::value::FieldTypeItem;

    let mut sched = Schedule::new();
    let (id, data) = make_panel_type();
    sched.insert(id, data);

    // `prefix` was "GP" on the input InternalData; expect it in the doc.
    let prefix = crdt::read_field(
        sched.doc(),
        "panel_type",
        id.entity_uuid(),
        "prefix",
        FieldTypeItem::String,
        CrdtFieldType::Scalar,
    )
    .unwrap();
    assert_eq!(prefix.unwrap().to_string(), "GP");
    assert!(!crdt::is_deleted(
        sched.doc(),
        "panel_type",
        id.entity_uuid()
    ));
}

#[test]
fn crdt_mirror_tracks_single_field_write() {
    use schedule_core::crdt;
    use schedule_core::crdt::CrdtFieldType;
    use schedule_core::entity::EntityType;
    use schedule_core::value::{FieldValue, FieldValueItem};

    let mut sched = Schedule::new();
    let (id, data) = make_panel_type();
    sched.insert(id, data);

    PanelTypeEntityType::field_set()
        .write_field_value(
            "prefix",
            id,
            &mut sched,
            FieldValue::Single(FieldValueItem::String("SP".into())),
        )
        .unwrap();

    let got = crdt::read_field(
        sched.doc(),
        "panel_type",
        id.entity_uuid(),
        "prefix",
        FieldTypeItem::String,
        CrdtFieldType::Scalar,
    )
    .unwrap()
    .unwrap();
    assert_eq!(got.to_string(), "SP");
}

#[test]
fn remove_entity_soft_deletes_in_doc_and_evicts_cache() {
    use schedule_core::crdt;

    let mut sched = Schedule::new();
    let (id, data) = make_panel_type();
    sched.insert(id, data);
    assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 1);
    assert!(!crdt::is_deleted(
        sched.doc(),
        "panel_type",
        id.entity_uuid()
    ));

    sched.remove_entity::<PanelTypeEntityType>(id);

    assert_eq!(sched.entity_count::<PanelTypeEntityType>(), 0);
    assert!(crdt::is_deleted(
        sched.doc(),
        "panel_type",
        id.entity_uuid()
    ));
}

// ── Save / Load round-trip ────────────────────────────────────────────────

#[test]
fn save_load_roundtrips_panel_type() {
    let mut sched = Schedule::new();
    let (id, data) = make_panel_type();
    sched.insert(id, data);

    let bytes = sched.save();
    let loaded = Schedule::load(&bytes).expect("load");

    assert_eq!(loaded.entity_count::<PanelTypeEntityType>(), 1);
    let got = loaded.get_internal::<PanelTypeEntityType>(id).unwrap();
    assert_eq!(got.data.prefix, "GP");
    assert_eq!(got.data.panel_kind, "Guest Panel");
}

#[test]
fn save_load_roundtrips_multiple_entity_types() {
    let mut sched = Schedule::new();
    let (pt_id, pt_data) = make_panel_type();
    let (pr_id, pr_data) = make_presenter("Alice");
    let (er_id, er_data) = make_event_room("Panel 1");
    let (hr_id, hr_data) = make_hotel_room("Suite A");
    sched.insert(pt_id, pt_data);
    sched.insert(pr_id, pr_data);
    sched.insert(er_id, er_data);
    sched.insert(hr_id, hr_data);

    let bytes = sched.save();
    let loaded = Schedule::load(&bytes).expect("load");

    assert_eq!(loaded.entity_count::<PanelTypeEntityType>(), 1);
    assert_eq!(loaded.entity_count::<PresenterEntityType>(), 1);
    assert_eq!(loaded.entity_count::<EventRoomEntityType>(), 1);
    assert_eq!(loaded.entity_count::<HotelRoomEntityType>(), 1);

    assert_eq!(
        loaded
            .get_internal::<PresenterEntityType>(pr_id)
            .unwrap()
            .data
            .name,
        "Alice"
    );
    assert_eq!(
        loaded
            .get_internal::<EventRoomEntityType>(er_id)
            .unwrap()
            .data
            .room_name,
        "Panel 1"
    );
    assert_eq!(
        loaded
            .get_internal::<HotelRoomEntityType>(hr_id)
            .unwrap()
            .data
            .hotel_room_name,
        "Suite A"
    );
}

#[test]
fn save_load_respects_soft_delete() {
    let mut sched = Schedule::new();
    let (kept_id, kept_data) = make_panel_type();
    let (gone_id, gone_data) = make_panel_type();
    sched.insert(kept_id, kept_data);
    sched.insert(gone_id, gone_data);
    sched.remove_entity::<PanelTypeEntityType>(gone_id);

    let bytes = sched.save();
    let loaded = Schedule::load(&bytes).expect("load");

    assert_eq!(loaded.entity_count::<PanelTypeEntityType>(), 1);
    assert!(loaded
        .get_internal::<PanelTypeEntityType>(kept_id)
        .is_some());
    assert!(loaded
        .get_internal::<PanelTypeEntityType>(gone_id)
        .is_none());
}

#[test]
fn load_rejects_garbage_bytes() {
    let err = Schedule::load(b"this is not an automerge doc").expect_err("must error");
    assert!(matches!(err, LoadError::Codec(_)));
}

// ── Native file format (FEATURE-025) ──────────────────────────────────────

#[test]
fn save_to_file_load_from_file_roundtrips_entity_data() {
    let mut sched = Schedule::new();
    let (pt_id, pt_data) = make_panel_type();
    let (pr_id, pr_data) = make_presenter("Alice");
    sched.insert(pt_id, pt_data);
    sched.insert(pr_id, pr_data);

    let bytes = sched.save_to_file();
    let loaded = Schedule::load_from_file(&bytes).expect("load_from_file");

    assert_eq!(loaded.entity_count::<PanelTypeEntityType>(), 1);
    assert_eq!(loaded.entity_count::<PresenterEntityType>(), 1);
    assert_eq!(
        loaded
            .get_internal::<PresenterEntityType>(pr_id)
            .unwrap()
            .data
            .name,
        "Alice"
    );
}

#[test]
fn save_to_file_load_from_file_preserves_metadata() {
    let mut sched = Schedule::new();
    sched.metadata.generator = "cosam-convert 0.1".into();
    sched.metadata.version = 42;
    let saved_id = sched.metadata.schedule_id;
    let saved_at = sched.metadata.created_at;

    let bytes = sched.save_to_file();
    let loaded = Schedule::load_from_file(&bytes).expect("load_from_file");

    assert_eq!(loaded.metadata.schedule_id, saved_id);
    assert_eq!(loaded.metadata.created_at, saved_at);
    assert_eq!(loaded.metadata.generator, "cosam-convert 0.1");
    assert_eq!(loaded.metadata.version, 42);
}

#[test]
fn save_to_file_load_from_file_preserves_edges() {
    let mut sched = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (pres_id, pres_data) = make_presenter("Alice");
    sched.insert(panel_id, panel_data);
    sched.insert(pres_id, pres_data);
    sched
        .edge_add(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
            std::iter::once(pres_id),
        )
        .unwrap();

    let bytes = sched.save_to_file();
    let loaded = Schedule::load_from_file(&bytes).expect("load_from_file");

    let forwards = loaded.connected_field_nodes(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
    );
    assert_eq!(forwards, vec![pres_id.into()]);
}

#[test]
fn load_from_file_rejects_too_short() {
    let err = Schedule::load_from_file(b"short").expect_err("must error");
    assert!(matches!(err, LoadError::Format(_)));
}

#[test]
fn load_from_file_rejects_wrong_magic() {
    let mut bad = b"WRONG\x00\x01\x00\x00\x00\x00\x00".to_vec();
    bad.extend_from_slice(&automerge::AutoCommit::new().save());
    let err = Schedule::load_from_file(&bad).expect_err("must error");
    assert!(matches!(err, LoadError::Format(_)));
}

#[test]
fn load_from_file_rejects_unsupported_version() {
    // Write a valid magic + version 99 header.
    let version: u16 = 99;
    let meta_json = b"{}";
    let meta_len = meta_json.len() as u32;
    let mut buf = Vec::new();
    buf.extend_from_slice(b"COSAM\x00");
    buf.extend_from_slice(&version.to_le_bytes());
    buf.extend_from_slice(&meta_len.to_le_bytes());
    buf.extend_from_slice(meta_json);
    buf.extend_from_slice(&automerge::AutoCommit::new().save());
    let err = Schedule::load_from_file(&buf).expect_err("must error");
    assert!(matches!(err, LoadError::Format(_)));
}

#[test]
fn load_from_file_rejects_garbage_bytes() {
    let err = Schedule::load_from_file(b"this is not a cosam file").expect_err("must error");
    assert!(matches!(err, LoadError::Format(_)));
}

// ── Edge CRDT round-trip (FEATURE-023) ────────────────────────────────────

#[test]
fn save_load_roundtrips_panel_presenter_edge() {
    let mut sched = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (pres_id, pres_data) = make_presenter("Alice");
    sched.insert(panel_id, panel_data);
    sched.insert(pres_id, pres_data);
    sched
        .edge_add(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
            std::iter::once(pres_id),
        )
        .unwrap();

    let bytes = sched.save();
    let loaded = Schedule::load(&bytes).expect("load");

    // Forward edge (panel → presenter)
    let forwards = loaded.connected_field_nodes(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
    );
    assert_eq!(forwards, vec![pres_id.into()]);
    // Reverse edge (presenter → panel) also rebuilt from the single
    // owner list on the panel side.
    let reverses = loaded.connected_field_nodes(pres_id, presenter::EDGE_CREDITED_PANELS);
    assert_eq!(reverses, vec![panel_id.into()]);
}

#[test]
fn save_load_roundtrips_event_room_hotel_room_edge() {
    let mut sched = Schedule::new();
    let (er_id, er_data) = make_event_room("Panel 1");
    let (hr_id, hr_data) = make_hotel_room("Suite A");
    sched.insert(er_id, er_data);
    sched.insert(hr_id, hr_data);
    sched
        .edge_add(
            er_id,
            schedule_core::tables::event_room::EDGE_HOTEL_ROOMS,
            std::iter::once(hr_id),
        )
        .unwrap();

    let bytes = sched.save();
    let loaded = Schedule::load(&bytes).expect("load");

    let hotel_rooms =
        loaded.connected_field_nodes(er_id, schedule_core::tables::event_room::EDGE_HOTEL_ROOMS);
    assert_eq!(hotel_rooms, vec![hr_id.into()]);
    let event_rooms =
        loaded.connected_field_nodes(hr_id, schedule_core::tables::hotel_room::EDGE_EVENT_ROOMS);
    assert_eq!(event_rooms, vec![er_id.into()]);
}

#[test]
fn save_load_roundtrips_presenter_group_edge() {
    let mut sched = Schedule::new();
    let (alice_id, alice) = make_presenter("Alice");
    let (group_id, group) = make_presenter("Speakers");
    sched.insert(alice_id, alice);
    sched.insert(group_id, group);
    // alice is a member of the Speakers group
    sched
        .edge_add(alice_id, presenter::EDGE_GROUPS, std::iter::once(group_id))
        .unwrap();

    let bytes = sched.save();
    let loaded = Schedule::load(&bytes).expect("load");

    let groups = loaded.connected_field_nodes(alice_id, presenter::EDGE_GROUPS);
    assert_eq!(groups, vec![group_id.into()]);
    let members = loaded.connected_field_nodes(group_id, presenter::EDGE_MEMBERS);
    assert_eq!(members, vec![alice_id.into()]);
}

#[test]
fn edge_remove_roundtrips_through_save_load() {
    let mut sched = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (pres_id, pres_data) = make_presenter("Alice");
    sched.insert(panel_id, panel_data);
    sched.insert(pres_id, pres_data);
    sched
        .edge_add(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
            std::iter::once(pres_id),
        )
        .unwrap();
    sched.edge_remove(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
        std::iter::once(pres_id),
    );

    let bytes = sched.save();
    let loaded = Schedule::load(&bytes).expect("load");

    let forwards = loaded.connected_field_nodes(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
    );
    assert!(forwards.is_empty());
}

#[test]
fn edge_set_replaces_through_save_load() {
    let mut sched = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (alice_id, alice_data) = make_presenter("Alice");
    let (bob_id, bob_data) = make_presenter("Bob");
    sched.insert(panel_id, panel_data);
    sched.insert(alice_id, alice_data);
    sched.insert(bob_id, bob_data);
    sched
        .edge_add(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
            std::iter::once(alice_id),
        )
        .unwrap();
    sched
        .edge_set(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
            vec![bob_id],
        )
        .unwrap();

    let bytes = sched.save();
    let loaded = Schedule::load(&bytes).expect("load");

    let forwards = loaded.connected_field_nodes(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
    );
    assert_eq!(forwards, vec![bob_id.into()]);
}

/// Concurrent add/add from two replicas converges to the union.
#[test]
fn concurrent_edge_adds_merge_to_union() {
    use automerge::AutoCommit;

    // Base replica holds a panel + two presenters, no edges yet.
    let mut base = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (alice_id, alice_data) = make_presenter("Alice");
    let (bob_id, bob_data) = make_presenter("Bob");
    base.insert(panel_id, panel_data);
    base.insert(alice_id, alice_data);
    base.insert(bob_id, bob_data);
    let base_bytes = base.save();

    // Replica A adds Alice.
    let mut replica_a = Schedule::load(&base_bytes).expect("load A");
    replica_a
        .edge_add(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
            std::iter::once(alice_id),
        )
        .unwrap();

    // Replica B (independent) adds Bob.
    let mut replica_b = Schedule::load(&base_bytes).expect("load B");
    replica_b
        .edge_add(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
            std::iter::once(bob_id),
        )
        .unwrap();

    // Merge A ← B at the automerge layer, then rebuild via load().
    let mut doc_a = AutoCommit::load(&replica_a.save()).unwrap();
    let mut doc_b = AutoCommit::load(&replica_b.save()).unwrap();
    doc_a.merge(&mut doc_b).unwrap();
    let merged = Schedule::load(&doc_a.save()).expect("load merged");

    let mut forwards = merged.connected_field_nodes(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
    );
    forwards.sort_by_key(|id| id.entity_uuid());
    let mut expected: Vec<schedule_core::entity::RuntimeEntityId> =
        vec![alice_id.into(), bob_id.into()];
    expected.sort_by_key(|id| id.entity_uuid());
    assert_eq!(forwards, expected);
}

// ── Change tracking / merge / conflicts (FEATURE-024) ────────────────────

#[test]
fn merge_two_schedules_combines_entities() {
    let mut a = Schedule::new();
    let (pt_id, pt_data) = make_panel_type();
    a.insert(pt_id, pt_data);

    // B starts from the shared base state and adds a presenter.
    let mut b = Schedule::load(&a.save()).expect("load base");
    let (pr_id, pr_data) = make_presenter("Alice");
    b.insert(pr_id, pr_data);

    a.merge(&mut b).expect("merge");

    assert_eq!(a.entity_count::<PanelTypeEntityType>(), 1);
    assert_eq!(a.entity_count::<PresenterEntityType>(), 1);
    assert!(a.get_internal::<PresenterEntityType>(pr_id).is_some());
}

#[test]
fn merge_preserves_edges_from_both_sides() {
    use schedule_core::entity::EntityType;

    let mut base = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (alice_id, alice_data) = make_presenter("Alice");
    let (bob_id, bob_data) = make_presenter("Bob");
    base.insert(panel_id, panel_data);
    base.insert(alice_id, alice_data);
    base.insert(bob_id, bob_data);

    let mut a = Schedule::load(&base.save()).expect("load A");
    let mut b = Schedule::load(&base.save()).expect("load B");
    a.edge_add(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
        std::iter::once(alice_id),
    )
    .unwrap();
    b.edge_add(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
        std::iter::once(bob_id),
    )
    .unwrap();

    a.merge(&mut b).expect("merge");

    let mut ids: Vec<_> = a
        .connected_field_nodes(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
        )
        .iter()
        .map(|id| id.entity_uuid())
        .collect();
    ids.sort();
    let mut expected = vec![alice_id.entity_uuid(), bob_id.entity_uuid()];
    expected.sort();
    assert_eq!(ids, expected);
    let _ = PanelEntityType::TYPE_NAME; // suppress unused-trait-import warning
}

#[test]
fn apply_changes_delta_sync_roundtrip() {
    // A creates a panel_type, captures heads.  B diverges: loads A's
    // state, adds a presenter, sends back only the changes A hasn't
    // observed.  A applies them and should see the new presenter.
    let mut a = Schedule::new();
    let (pt_id, pt_data) = make_panel_type();
    a.insert(pt_id, pt_data);
    let heads_a = a.get_heads();

    let mut b = Schedule::load(&a.save()).expect("load");
    let (pr_id, pr_data) = make_presenter("Alice");
    b.insert(pr_id, pr_data);

    let delta = b.get_changes_since(&heads_a);
    assert!(!delta.is_empty(), "expected at least one new change");

    a.apply_changes(&delta).expect("apply");

    assert!(a.get_internal::<PresenterEntityType>(pr_id).is_some());
    assert_eq!(a.entity_count::<PanelTypeEntityType>(), 1);
}

#[test]
fn get_changes_returns_full_history() {
    let mut a = Schedule::new();
    let (pt_id, pt_data) = make_panel_type();
    a.insert(pt_id, pt_data);

    let changes = a.get_changes();
    assert!(!changes.is_empty());

    // Replay the changes into a fresh schedule and verify the entity
    // is reconstructed.
    let mut b = Schedule::new();
    b.apply_changes(&changes).expect("apply");
    assert!(b.get_internal::<PanelTypeEntityType>(pt_id).is_some());
}

#[test]
fn conflicts_for_reports_concurrent_scalar_writes() {
    // Two replicas concurrently write different `prefix` values to the
    // same panel_type; after merge, `conflicts_for` surfaces both.
    use schedule_core::entity::EntityType;
    use schedule_core::value::{FieldValue, FieldValueItem};

    let mut base = Schedule::new();
    let (pt_id, pt_data) = make_panel_type();
    base.insert(pt_id, pt_data);

    let mut a = Schedule::load(&base.save()).expect("load A");
    let mut b = Schedule::load(&base.save()).expect("load B");

    PanelTypeEntityType::field_set()
        .write_field_value(
            "prefix",
            pt_id,
            &mut a,
            FieldValue::Single(FieldValueItem::String("A-PREFIX".into())),
        )
        .unwrap();
    PanelTypeEntityType::field_set()
        .write_field_value(
            "prefix",
            pt_id,
            &mut b,
            FieldValue::Single(FieldValueItem::String("B-PREFIX".into())),
        )
        .unwrap();

    a.merge(&mut b).expect("merge");

    let conflicts = a.conflicts_for::<PanelTypeEntityType>(pt_id, "prefix");
    let strs: Vec<String> = conflicts
        .into_iter()
        .filter_map(|fv| match fv {
            FieldValue::Single(FieldValueItem::String(s)) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(strs.len(), 2, "expected both concurrent values: {strs:?}");
    assert!(strs.contains(&"A-PREFIX".to_string()));
    assert!(strs.contains(&"B-PREFIX".to_string()));
}

#[test]
fn conflicts_for_returns_single_when_no_conflict() {
    use schedule_core::entity::EntityType;
    use schedule_core::value::{FieldValue, FieldValueItem};

    let mut sched = Schedule::new();
    let (pt_id, pt_data) = make_panel_type();
    sched.insert(pt_id, pt_data);
    PanelTypeEntityType::field_set()
        .write_field_value(
            "prefix",
            pt_id,
            &mut sched,
            FieldValue::Single(FieldValueItem::String("solo".into())),
        )
        .unwrap();

    let conflicts = sched.conflicts_for::<PanelTypeEntityType>(pt_id, "prefix");
    assert_eq!(conflicts.len(), 1);
    match &conflicts[0] {
        FieldValue::Single(FieldValueItem::String(s)) => assert_eq!(s, "solo"),
        other => panic!("unexpected conflict value: {other:?}"),
    }
}

/// Concurrent add vs. unobserved remove resolves add-wins under
/// automerge's list semantics.
#[test]
fn concurrent_add_beats_unobserved_remove() {
    use automerge::AutoCommit;

    let mut base = Schedule::new();
    let (panel_id, panel_data) = make_panel();
    let (alice_id, alice_data) = make_presenter("Alice");
    base.insert(panel_id, panel_data);
    base.insert(alice_id, alice_data);
    let base_bytes = base.save();

    // A adds Alice without knowing about any remove on B's side.
    let mut replica_a = Schedule::load(&base_bytes).expect("load A");
    replica_a
        .edge_add(
            panel_id,
            schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
            std::iter::once(alice_id),
        )
        .unwrap();

    // B starts from the same base (no edge), removes Alice (no-op on its
    // own state but records a causally-unordered change); this simulates
    // B having never observed A's add.
    let mut replica_b = Schedule::load(&base_bytes).expect("load B");
    replica_b.edge_remove(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
        std::iter::once(alice_id),
    );

    let mut doc_a = AutoCommit::load(&replica_a.save()).unwrap();
    let mut doc_b = AutoCommit::load(&replica_b.save()).unwrap();
    doc_a.merge(&mut doc_b).unwrap();
    let merged = Schedule::load(&doc_a.save()).expect("load merged");

    // Add wins: Alice is still in the list.
    let forwards = merged.connected_field_nodes(
        panel_id,
        schedule_core::tables::panel::EDGE_CREDITED_PRESENTERS,
    );
    assert_eq!(forwards, vec![alice_id.into()]);
}

// ── Edge cache / transitive closure tests ────────────────────────────────
//
// These tests use `PresenterEntityType` which has the `EDGE_MEMBERS`
// and `EDGE_GROUPS` descriptor with `is_transitive: true`.
// `edge_add(...,EDGE_MEMBERS,...)` triggers `TransitiveEdgeCache` invalidation and
// `inclusive_edges_from/to` follows the transitive closure.

#[test]
fn inclusive_edges_from_transitive_closure() {
    let mut sched = Schedule::new();
    let (p1_id, p1_data) = make_presenter("P1");
    let (p2_id, p2_data) = make_presenter("P2");
    let (p3_id, p3_data) = make_presenter("P3");
    sched.insert(p1_id, p1_data);
    sched.insert(p2_id, p2_data);
    sched.insert(p3_id, p3_data);

    // Chain: p1 → p2 → p3 (member-of-group direction)
    sched
        .edge_add(p1_id, presenter::EDGE_GROUPS, std::iter::once(p2_id))
        .unwrap();
    sched
        .edge_add(p2_id, presenter::EDGE_GROUPS, std::iter::once(p3_id))
        .unwrap();

    // Inclusive groups from p1 should reach both p2 and p3 transitively.
    let result = sched.inclusive_edges(p1_id, presenter::EDGE_GROUPS);
    assert_eq!(result.len(), 2);
    assert!(result.contains(&p2_id));
    assert!(result.contains(&p3_id));
}

#[test]
fn inclusive_edges_to_transitive_closure() {
    let mut sched = Schedule::new();
    let (p1_id, p1_data) = make_presenter("P1");
    let (p2_id, p2_data) = make_presenter("P2");
    let (p3_id, p3_data) = make_presenter("P3");
    sched.insert(p1_id, p1_data);
    sched.insert(p2_id, p2_data);
    sched.insert(p3_id, p3_data);

    // Chain: p1 → p2 → p3 (member-of-group direction)
    sched
        .edge_add(p1_id, presenter::EDGE_GROUPS, std::iter::once(p2_id))
        .unwrap();
    sched
        .edge_add(p2_id, presenter::EDGE_GROUPS, std::iter::once(p3_id))
        .unwrap();

    // Inclusive members of p3 should include both p1 and p2 transitively.
    let result = sched.inclusive_edges(p3_id, presenter::EDGE_MEMBERS);
    assert_eq!(result.len(), 2);
    assert!(result.contains(&p1_id));
    assert!(result.contains(&p2_id));
}

#[test]
fn inclusive_edges_cycle_handling() {
    let mut sched = Schedule::new();
    let (p1_id, p1_data) = make_presenter("P1");
    let (p2_id, p2_data) = make_presenter("P2");
    sched.insert(p1_id, p1_data);
    sched.insert(p2_id, p2_data);

    // Cycle: p1 → p2, p2 → p1 (member-of-group direction)
    sched
        .edge_add(p1_id, presenter::EDGE_GROUPS, std::iter::once(p2_id))
        .unwrap();
    sched
        .edge_add(p2_id, presenter::EDGE_GROUPS, std::iter::once(p1_id))
        .unwrap();

    // Should not infinite loop; p2 is reachable from p1.
    let result = sched.inclusive_edges(p1_id, presenter::EDGE_GROUPS);
    assert!(result.contains(&p2_id));
}

#[test]
fn inclusive_edges_cache_invalidation() {
    let mut sched = Schedule::new();
    let (p1_id, p1_data) = make_presenter("P1");
    let (p2_id, p2_data) = make_presenter("P2");
    let (p3_id, p3_data) = make_presenter("P3");
    sched.insert(p1_id, p1_data);
    sched.insert(p2_id, p2_data);
    sched.insert(p3_id, p3_data);

    // Add initial edge p1 → p2 (member-of-group direction).
    sched
        .edge_add(p1_id, presenter::EDGE_GROUPS, std::iter::once(p2_id))
        .unwrap();
    let result1: Vec<PresenterId> = sched.inclusive_edges(p1_id, presenter::EDGE_GROUPS);
    assert_eq!(result1.len(), 1);

    // Add p2 → p3; cache should invalidate and now p3 is reachable from p1.
    sched
        .edge_add(p2_id, presenter::EDGE_GROUPS, std::iter::once(p3_id))
        .unwrap();
    let result2 = sched.inclusive_edges(p1_id, presenter::EDGE_GROUPS);
    assert!(result2.contains(&p2_id));
    assert!(result2.contains(&p3_id));
}
