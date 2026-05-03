/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Integration tests for edge system symmetry and transitive closure.

use schedule_core::entity::{EntityId, UuidPreference};
use schedule_core::schedule::Schedule;
use schedule_core::tables::presenter::{
    self, PresenterCommonData, PresenterId, PresenterInternalData,
};

fn make_presenter_id() -> PresenterId {
    EntityId::from_preference(UuidPreference::GenerateNew)
}

fn make_presenter(name: &str) -> (PresenterId, PresenterInternalData) {
    let id = make_presenter_id();
    let data = PresenterInternalData {
        id,
        data: PresenterCommonData {
            name: name.into(),
            ..Default::default()
        },
    };
    (id, data)
}

fn make_group(name: &str) -> (PresenterId, PresenterInternalData) {
    let id = make_presenter_id();
    let data = PresenterInternalData {
        id,
        data: PresenterCommonData {
            name: name.into(),
            is_explicit_group: true,
            ..Default::default()
        },
    };
    (id, data)
}

#[test]
fn test_inclusive_groups_transitive_closure() {
    let mut sched = Schedule::default();
    let (member_id, member_data) = make_presenter("Alice");
    let (group_id, group_data) = make_group("MyBand");
    let (parent_group_id, parent_group_data) = make_group("MusicIndustry");

    sched.insert(member_id, member_data);
    sched.insert(group_id, group_data);
    sched.insert(parent_group_id, parent_group_data);

    // member → group → parent_group (transitive chain)
    sched
        .edge_add(member_id, presenter::EDGE_GROUPS, std::iter::once(group_id))
        .expect("edge type validation failed");
    sched
        .edge_add(
            group_id,
            presenter::EDGE_GROUPS,
            std::iter::once(parent_group_id),
        )
        .expect("edge type validation failed");

    // inclusive_edges should return both direct and transitive groups
    let result = sched.inclusive_edges(member_id, presenter::EDGE_GROUPS);
    assert_eq!(result.len(), 2);
    assert!(result.contains(&group_id));
    assert!(result.contains(&parent_group_id));
}

#[test]
fn test_inclusive_members_transitive_closure() {
    let mut sched = Schedule::default();
    let (group_id, group_data) = make_group("MyBand");
    let (member1_id, member1_data) = make_presenter("Alice");
    let (member2_id, member2_data) = make_presenter("Bob");
    let (sub_member_id, sub_member_data) = make_presenter("Charlie");

    sched.insert(group_id, group_data);
    sched.insert(member1_id, member1_data);
    sched.insert(member2_id, member2_data);
    sched.insert(sub_member_id, sub_member_data);

    // group ← member1, member2
    sched
        .edge_add(
            member1_id,
            presenter::EDGE_GROUPS,
            std::iter::once(group_id),
        )
        .expect("edge type validation failed");
    sched
        .edge_add(
            member2_id,
            presenter::EDGE_GROUPS,
            std::iter::once(group_id),
        )
        .expect("edge type validation failed");
    // member1 ← sub_member (nested group relationship)
    sched
        .edge_add(
            sub_member_id,
            presenter::EDGE_GROUPS,
            std::iter::once(member1_id),
        )
        .expect("edge type validation failed");

    // inclusive_edges should return member1, member2, and transitive sub_member
    let result = sched.inclusive_edges(group_id, presenter::EDGE_MEMBERS);
    assert_eq!(result.len(), 3);
    assert!(result.contains(&member1_id));
    assert!(result.contains(&member2_id));
    assert!(result.contains(&sub_member_id));
}

#[test]
fn test_edge_add_multiple_targets_symmetry() {
    let mut sched = Schedule::default();
    let (member_id, member_data) = make_presenter("Alice");
    let (group1_id, group1_data) = make_group("MyBand");
    let (group2_id, group2_data) = make_group("YourBand");

    sched.insert(member_id, member_data);
    sched.insert(group1_id, group1_data);
    sched.insert(group2_id, group2_data);

    // Add member to both groups in one call
    let added = sched
        .edge_add(
            member_id,
            presenter::EDGE_GROUPS,
            vec![group1_id, group2_id],
        )
        .expect("edge type validation failed");
    assert_eq!(added.len(), 2);

    // Verify member's groups contains both
    let groups = sched.connected_field_nodes(member_id, presenter::EDGE_GROUPS);
    assert_eq!(groups.len(), 2);
    assert!(groups.contains(&group1_id.into()));
    assert!(groups.contains(&group2_id.into()));

    // Verify both groups' members contain member
    assert_eq!(
        sched.connected_field_nodes(group1_id, presenter::EDGE_MEMBERS),
        vec![member_id.into()]
    );
    assert_eq!(
        sched.connected_field_nodes(group2_id, presenter::EDGE_MEMBERS),
        vec![member_id.into()]
    );

    // Remove from both groups
    let removed = sched.edge_remove(
        member_id,
        presenter::EDGE_GROUPS,
        vec![group1_id, group2_id],
    );
    assert_eq!(removed.len(), 2);

    // Verify all directions cleared
    assert!(sched
        .connected_field_nodes(member_id, presenter::EDGE_GROUPS)
        .is_empty());
    assert!(sched
        .connected_field_nodes(group1_id, presenter::EDGE_MEMBERS)
        .is_empty());
    assert!(sched
        .connected_field_nodes(group2_id, presenter::EDGE_MEMBERS)
        .is_empty());
}

#[test]
fn test_edge_add_from_group_side_symmetry() {
    let mut sched = Schedule::default();
    let (member_id, member_data) = make_presenter("Alice");
    let (group_id, group_data) = make_group("MyBand");

    sched.insert(member_id, member_data);
    sched.insert(group_id, group_data);

    // Add edge from group side: group → member
    sched
        .edge_add(
            group_id,
            presenter::EDGE_MEMBERS,
            std::iter::once(member_id),
        )
        .expect("edge type validation failed");

    // Verify member's groups contains group
    assert_eq!(
        sched.connected_field_nodes(member_id, presenter::EDGE_GROUPS),
        vec![group_id.into()]
    );
    // Verify group's members contains member
    assert_eq!(
        sched.connected_field_nodes(group_id, presenter::EDGE_MEMBERS),
        vec![member_id.into()]
    );

    // Remove edge from group side
    sched.edge_remove(
        group_id,
        presenter::EDGE_MEMBERS,
        std::iter::once(member_id),
    );

    // Verify both directions are cleared
    assert!(sched
        .connected_field_nodes(member_id, presenter::EDGE_GROUPS)
        .is_empty());
    assert!(sched
        .connected_field_nodes(group_id, presenter::EDGE_MEMBERS)
        .is_empty());
}

#[test]
fn test_edge_add_multiple_members_from_group_side() {
    let mut sched = Schedule::default();
    let (group_id, group_data) = make_group("MyBand");
    let (member1_id, member1_data) = make_presenter("Alice");
    let (member2_id, member2_data) = make_presenter("Bob");

    sched.insert(group_id, group_data);
    sched.insert(member1_id, member1_data);
    sched.insert(member2_id, member2_data);

    // Add both members from group side
    let added = sched
        .edge_add(
            group_id,
            presenter::EDGE_MEMBERS,
            vec![member1_id, member2_id],
        )
        .expect("edge type validation failed");
    assert_eq!(added.len(), 2);

    // Verify group's members contains both
    let members = sched.connected_field_nodes(group_id, presenter::EDGE_MEMBERS);
    assert_eq!(members.len(), 2);
    assert!(members.contains(&member1_id.into()));
    assert!(members.contains(&member2_id.into()));

    // Verify both members' groups contain group
    assert_eq!(
        sched.connected_field_nodes(member1_id, presenter::EDGE_GROUPS),
        vec![group_id.into()]
    );
    assert_eq!(
        sched.connected_field_nodes(member2_id, presenter::EDGE_GROUPS),
        vec![group_id.into()]
    );

    // Remove both members from group side
    let removed = sched.edge_remove(
        group_id,
        presenter::EDGE_MEMBERS,
        vec![member1_id, member2_id],
    );
    assert_eq!(removed.len(), 2);

    // Verify all directions cleared
    assert!(sched
        .connected_field_nodes(group_id, presenter::EDGE_MEMBERS)
        .is_empty());
    assert!(sched
        .connected_field_nodes(member1_id, presenter::EDGE_GROUPS)
        .is_empty());
    assert!(sched
        .connected_field_nodes(member2_id, presenter::EDGE_GROUPS)
        .is_empty());
}
