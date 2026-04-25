/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`FieldId`] and [`FieldNodeId`] — field-based edge endpoint types.
//!
//! These types form the foundation of the FieldNodeId edge system, where each
//! edge endpoint is identified by both the entity UUID *and* which field the
//! relationship belongs to.  This makes edge direction self-describing and
//! eliminates the need for a separate `homogeneous_reverse` map.
//!
//! ## Design
//!
//! A [`FieldId`] is derived from the address of a `'static`
//! [`FieldDescriptor<E>`] singleton.  Because all field descriptors are
//! `'static`, their addresses are globally unique and stable for the life of
//! the process.
//!
//! A [`FieldNodeId`] combines a [`FieldId`] with a [`NonNilUuid`] to represent
//! "entity X's field Y" — the unit used as both map keys and neighbor values in
//! [`crate::edge_map::RawEdgeMap`].
//!
//! [`FieldDescriptor<E>`]: crate::field::FieldDescriptor

use crate::entity::EntityType;
use crate::field::FieldDescriptor;
use uuid::NonNilUuid;

// ── FieldId ───────────────────────────────────────────────────────────────────

/// Type-erased identity for a [`FieldDescriptor<E>`] static singleton.
///
/// Two `FieldId` values are equal if and only if they were derived from the
/// same static (i.e. the same memory address).
///
/// # Invariant
///
/// Only create `FieldId` values from `'static` references.  Creating one from
/// a stack-allocated `FieldDescriptor` produces a meaningless address that will
/// compare unequal to anything and must not be stored.
///
/// [`FieldDescriptor<E>`]: crate::field::FieldDescriptor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(usize);

impl FieldId {
    /// Create a `FieldId` from a `'static` field descriptor reference.
    ///
    /// The address is stable for the life of the process because all field
    /// descriptors are declared as `static` items.
    pub fn of<E: EntityType>(field: &'static FieldDescriptor<E>) -> Self {
        FieldId(field as *const FieldDescriptor<E> as *const () as usize)
    }

    /// Create a `FieldId` directly from a raw address.
    ///
    /// Only intended for use inside [`crate::field::FieldDescriptorAny`]
    /// implementations, which perform the same cast internally.
    pub(crate) fn from_raw(addr: usize) -> Self {
        FieldId(addr)
    }
}

// ── FieldNodeId ───────────────────────────────────────────────────────────────

/// An edge endpoint identified by "entity X's field Y".
///
/// Used as both the inner-map key and the neighbor-list entry in
/// [`crate::edge_map::RawEdgeMap`]:
///
/// ```text
/// HashMap<NonNilUuid,          // outer key: entity UUID
///     HashMap<FieldId,         // inner key: which field on that entity
///         Vec<FieldNodeId>>>   // values: (field, uuid) of the other side
/// ```
///
/// Storing the full `FieldNodeId` in neighbor lists means the reverse side of
/// an edge is self-describing — you know both the entity and the field
/// without any additional lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldNodeId {
    /// Which field on the entity this endpoint represents.
    pub field: FieldId,
    /// The entity instance.
    pub entity: NonNilUuid,
}

impl FieldNodeId {
    /// Construct from a [`FieldId`] and entity UUID.
    pub fn new(field: FieldId, entity: NonNilUuid) -> Self {
        Self { field, entity }
    }

    /// Construct from a typed `'static` field descriptor and an entity UUID.
    ///
    /// This is the primary constructor for call-sites that have a concrete
    /// `&'static FieldDescriptor<E>` in hand.
    pub fn of<E: EntityType>(field: &'static FieldDescriptor<E>, entity: NonNilUuid) -> Self {
        Self {
            field: FieldId::of(field),
            entity,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityType;
    use crate::field_set::FieldSet;
    use crate::value::{
        CrdtFieldType, FieldCardinality, FieldType, FieldTypeItem, ValidationError,
    };
    use uuid::Uuid;

    // ── Minimal mock entity + two static field descriptors ───────────────────

    struct MockEntity;

    #[derive(Clone, Debug)]
    struct MockData;

    impl EntityType for MockEntity {
        type InternalData = MockData;
        type Data = MockData;
        const TYPE_NAME: &'static str = "mock";
        fn uuid_namespace() -> &'static Uuid {
            static NS: std::sync::LazyLock<Uuid> =
                std::sync::LazyLock::new(|| Uuid::new_v5(&Uuid::NAMESPACE_OID, b"mock"));
            &NS
        }
        fn field_set() -> &'static FieldSet<Self> {
            unimplemented!()
        }
        fn export(_: &MockData) -> MockData {
            MockData
        }
        fn validate(_: &MockData) -> Vec<ValidationError> {
            vec![]
        }
    }

    static FIELD_A: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "field_a",
        display: "Field A",
        description: "Test field A",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Text),
        example: "",
        order: 0,
        read_fn: None,
        write_fn: None,
        verify_fn: None,
    };

    static FIELD_B: FieldDescriptor<MockEntity> = FieldDescriptor {
        name: "field_b",
        display: "Field B",
        description: "Test field B",
        aliases: &[],
        required: false,
        crdt_type: CrdtFieldType::Derived,
        field_type: FieldType(FieldCardinality::Optional, FieldTypeItem::Text),
        example: "",
        order: 1,
        read_fn: None,
        write_fn: None,
        verify_fn: None,
    };

    fn nnu(n: u128) -> NonNilUuid {
        NonNilUuid::new(Uuid::from_u128(n)).expect("test UUID must not be nil")
    }

    // ── FieldId tests ────────────────────────────────────────────────────────

    #[test]
    fn test_field_id_same_static_is_equal() {
        let id1 = FieldId::of::<MockEntity>(&FIELD_A);
        let id2 = FieldId::of::<MockEntity>(&FIELD_A);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_field_id_different_statics_differ() {
        let id_a = FieldId::of::<MockEntity>(&FIELD_A);
        let id_b = FieldId::of::<MockEntity>(&FIELD_B);
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn test_field_id_hash_consistent() {
        use std::collections::HashSet;
        let id = FieldId::of::<MockEntity>(&FIELD_A);
        let mut set = HashSet::new();
        set.insert(id);
        assert!(set.contains(&FieldId::of::<MockEntity>(&FIELD_A)));
        assert!(!set.contains(&FieldId::of::<MockEntity>(&FIELD_B)));
    }

    // ── FieldNodeId tests ────────────────────────────────────────────────────

    #[test]
    fn test_field_node_id_of_equals_new() {
        let uuid = nnu(1);
        let via_of = FieldNodeId::of::<MockEntity>(&FIELD_A, uuid);
        let via_new = FieldNodeId::new(FieldId::of::<MockEntity>(&FIELD_A), uuid);
        assert_eq!(via_of, via_new);
    }

    #[test]
    fn test_field_node_id_same_field_same_entity_equal() {
        let uuid = nnu(42);
        let a = FieldNodeId::of::<MockEntity>(&FIELD_A, uuid);
        let b = FieldNodeId::of::<MockEntity>(&FIELD_A, uuid);
        assert_eq!(a, b);
    }

    #[test]
    fn test_field_node_id_same_field_different_entity_differ() {
        let a = FieldNodeId::of::<MockEntity>(&FIELD_A, nnu(1));
        let b = FieldNodeId::of::<MockEntity>(&FIELD_A, nnu(2));
        assert_ne!(a, b);
    }

    #[test]
    fn test_field_node_id_different_field_same_entity_differ() {
        let uuid = nnu(1);
        let a = FieldNodeId::of::<MockEntity>(&FIELD_A, uuid);
        let b = FieldNodeId::of::<MockEntity>(&FIELD_B, uuid);
        assert_ne!(a, b);
    }

    #[test]
    fn test_field_node_id_hash_consistent() {
        use std::collections::HashSet;
        let uuid = nnu(7);
        let node = FieldNodeId::of::<MockEntity>(&FIELD_A, uuid);
        let mut set = HashSet::new();
        set.insert(node);
        assert!(set.contains(&FieldNodeId::of::<MockEntity>(&FIELD_A, uuid)));
        assert!(!set.contains(&FieldNodeId::of::<MockEntity>(&FIELD_B, uuid)));
    }
}
