/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`RuntimeFieldNodeId`] and [`FieldNodeId<E>`] — field-based edge endpoint types.
//!
//! These types form the foundation of the FieldNodeId edge system, where each
//! edge endpoint is identified by both the entity UUID *and* which field the
//! relationship belongs to. This makes edge direction self-describing and
//! eliminates the need for a separate `homogeneous_reverse` map.
//!
//! ## Design
//!
//! A [`RuntimeFieldNodeId`] combines a `'static` field descriptor reference with a
//! [`NonNilUuid`] to represent "entity X's field Y" in a type-erased way. This is the
//! unit used as both map keys and neighbor values in [`crate::edge_map::RawEdgeMap`].
//!
//! A [`FieldNodeId<E>`] provides compile-time type safety via a `PhantomData<E>`
//! marker, similar to how [`crate::entity::EntityId`] works for entities.
//!
//! Equality and hashing are based on the pointer address of the field descriptor,
//! which is stable for `'static` references.
//!
//! [`FieldDescriptor<E>`]: crate::field::FieldDescriptor

use crate::entity::{EntityId, EntityType, RuntimeEntityId};
use crate::field::{FieldDescriptor, NamedField};
use std::marker::PhantomData;
use uuid::NonNilUuid;

// ── FieldRef ─────────────────────────────────────────────────────────────────────

/// Wrapper for `&'static dyn NamedField` that implements Eq/Hash based on pointer address.
///
/// This allows field references to be used as HashMap keys without requiring the
/// NamedField trait itself to be dyn-compatible with Eq/Hash.
#[derive(Clone, Copy)]
pub struct FieldRef(pub &'static dyn NamedField);

impl PartialEq for FieldRef {
    fn eq(&self, other: &Self) -> bool {
        // Compare by data pointer address only (ignore vtable)
        // Cast fat pointer to thin pointer (data pointer only) then compare addresses
        std::ptr::eq(
            self.0 as *const dyn NamedField as *const (),
            other.0 as *const dyn NamedField as *const (),
        )
    }
}

impl Eq for FieldRef {}

impl std::hash::Hash for FieldRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash by data pointer address only (ignore vtable)
        // Cast fat pointer to thin pointer (data pointer only) then hash the address
        (self.0 as *const dyn NamedField as *const () as usize).hash(state);
    }
}

impl std::fmt::Debug for FieldRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FieldRef").field(&self.0.name()).finish()
    }
}

// ── RuntimeFieldNodeId ───────────────────────────────────────────────────────────

/// Runtime (untyped) edge endpoint identified by "entity X's field Y".
///
/// Used as both the inner-map key and the neighbor-list entry in
/// [`crate::edge_map::RawEdgeMap`]:
///
/// ```text
/// HashMap<NonNilUuid,                  // outer key: entity UUID
///     HashMap<RuntimeFieldNodeId,      // inner key: which field on that entity
///         Vec<RuntimeFieldNodeId>>>     // values: (field, uuid) of the other side
/// ```
///
/// Storing the full `RuntimeFieldNodeId` in neighbor lists means the reverse side of
/// an edge is self-describing — you know both the entity and the field
/// without any additional lookup.
///
/// Equality and hashing are based on the pointer address of the field descriptor,
/// which is stable for `'static` references.
///
/// For compile-time type safety, use [`FieldNodeId<E>`] instead.
#[derive(Clone, Copy)]
pub struct RuntimeFieldNodeId {
    /// The entity instance.
    pub uuid: NonNilUuid,
    /// Which field on the entity this endpoint represents.
    pub field: &'static dyn NamedField,
}

impl PartialEq for RuntimeFieldNodeId {
    fn eq(&self, other: &Self) -> bool {
        // Compare by data pointer address of the field descriptor and UUID
        // Cast fat pointer to thin pointer (data pointer only) then compare addresses
        std::ptr::eq(
            self.field as *const dyn NamedField as *const (),
            other.field as *const dyn NamedField as *const (),
        ) && self.uuid == other.uuid
    }
}

impl Eq for RuntimeFieldNodeId {}

impl std::hash::Hash for RuntimeFieldNodeId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash by pointer address of the field descriptor and UUID
        // Cast fat pointer to thin pointer (data pointer only) then to usize
        (self.field as *const dyn NamedField as *const () as usize).hash(state);
        self.uuid.hash(state);
    }
}

impl std::fmt::Debug for RuntimeFieldNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeFieldNodeId")
            .field("field", &self.field.name())
            .field("field_type", &self.field.entity_type_name())
            .field("entity", &self.uuid)
            .finish()
    }
}

impl RuntimeFieldNodeId {
    /// Construct from a field descriptor and entity UUID.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `uuid` actually identifies an entity of
    /// type field. Code that has a UUID→type registry (e.g. `Schedule`) can
    /// call this safely after verifying the type.
    pub unsafe fn from_uuid(uuid: NonNilUuid, field: &'static dyn NamedField) -> Self {
        Self { field, uuid }
    }

    /// Construct from a typed `'static` field descriptor and an entity UUID.
    ///
    /// This is the primary constructor for call-sites that have a concrete
    /// `&'static FieldDescriptor<E>` in hand.
    pub fn from_typed<E: EntityType>(
        field: &'static FieldDescriptor<E>,
        uuid: EntityId<E>,
    ) -> Self {
        Self {
            field,
            uuid: uuid.non_nil_uuid(),
        }
    }

    /// Get the entity UUID.
    #[must_use]
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        self.uuid
    }

    /// Get the entity type name from the field descriptor.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        self.field.entity_type_name()
    }

    /// Try to convert to a typed `FieldNodeId<E>`.
    ///
    /// Returns `None` if the field's entity type name does not match `E::TYPE_NAME`.
    #[must_use]
    pub fn try_as_typed<E: EntityType>(&self) -> Option<FieldNodeId<E>> {
        if self.type_name() == E::TYPE_NAME {
            // SAFETY: type_name match confirms the UUID belongs to entity type E.
            Some(unsafe { FieldNodeId::new(self.field, self.uuid) })
        } else {
            None
        }
    }
}

// ── FieldNodeId<E> ─────────────────────────────────────────────────────────────

/// Compile-time type-safe field node identifier.
///
/// Wraps a field and entity UUID with a `PhantomData<E>` so the type system
/// prevents mixing field node IDs from different entity types.
///
/// This is the typed counterpart to [`RuntimeFieldNodeId`], similar to how
/// [`crate::entity::EntityId`] relates to [`crate::entity::RuntimeEntityId`].
///
/// Equality and hashing are based on the pointer address of the field descriptor,
/// which is stable for `'static` references.
pub struct FieldNodeId<E: EntityType> {
    uuid: NonNilUuid,
    field: &'static FieldDescriptor<E>,
    _marker: PhantomData<fn() -> E>,
}

impl<E: EntityType> PartialEq for FieldNodeId<E> {
    fn eq(&self, other: &Self) -> bool {
        // Compare by pointer address of the field descriptor and UUID
        std::ptr::eq(self.field, other.field) && self.uuid == other.uuid
    }
}

impl<E: EntityType> Eq for FieldNodeId<E> {}

impl<E: EntityType> std::hash::Hash for FieldNodeId<E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash by pointer address of the field descriptor and UUID
        // Cast fat pointer to thin pointer (data pointer only) then to usize
        (self.field as *const dyn NamedField as *const () as usize).hash(state);
        self.uuid.hash(state);
    }
}

impl<E: EntityType> std::fmt::Debug for FieldNodeId<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldNodeId")
            .field("field", &self.field.name())
            .field("field_type", &self.field.entity_type_name())
            .field("entity_type", &E::TYPE_NAME)
            .field("entity", &self.uuid)
            .finish()
    }
}

impl<E: EntityType> Clone for FieldNodeId<E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E: EntityType> Copy for FieldNodeId<E> {}

impl<E: EntityType> FieldNodeId<E> {
    /// Create a typed field node ID from a field descriptor and entity UUID.
    ///
    /// # Safety
    ///
    /// Caller must ensure `field` is actually a `FieldDescriptor<E>`. This is
    /// typically verified by the caller using a type registry before calling this.
    #[must_use]
    pub unsafe fn new(field: &'static dyn NamedField, entity: NonNilUuid) -> Self {
        use std::any::Any;
        let field_descriptor = (field as &dyn Any)
            .downcast_ref::<FieldDescriptor<E>>()
            .expect("downcast failed: field is not a FieldDescriptor<E>");
        Self {
            field: field_descriptor,
            uuid: entity,
            _marker: PhantomData,
        }
    }

    /// Create from a typed field descriptor and entity UUID.
    #[must_use]
    pub fn from_descriptor(field: &'static FieldDescriptor<E>, entity: NonNilUuid) -> Self {
        Self {
            field,
            uuid: entity,
            _marker: PhantomData,
        }
    }

    /// Create from a field descriptor and typed entity ID.
    ///
    /// # Safety
    ///
    /// Caller must ensure `field` is actually a `FieldDescriptor<E>`. This is
    /// typically verified by the caller using a type registry before calling this.
    #[must_use]
    pub unsafe fn from_entity_id(field: &'static dyn NamedField, entity: EntityId<E>) -> Self {
        use std::any::Any;
        let field_descriptor = (field as &dyn Any)
            .downcast_ref::<FieldDescriptor<E>>()
            .expect("downcast failed: field is not a FieldDescriptor<E>");
        Self {
            field: field_descriptor,
            uuid: entity.non_nil_uuid(),
            _marker: PhantomData,
        }
    }

    /// Create from a field descriptor and runtime entity ID.
    ///
    /// Returns `None` if the runtime entity's type name does not match `E::TYPE_NAME`.
    #[must_use]
    pub fn from_runtime_entity_id(
        field: &'static dyn NamedField,
        entity: RuntimeEntityId,
    ) -> Option<Self> {
        if entity.type_name() == E::TYPE_NAME {
            // SAFETY: type_name match confirms the UUID belongs to entity type E.
            Some(unsafe { Self::new(field, entity.non_nil_uuid()) })
        } else {
            None
        }
    }

    /// Get the entity UUID.
    #[must_use]
    pub fn entity(&self) -> NonNilUuid {
        self.uuid
    }

    /// Get the entity as a typed `EntityId<E>`.
    #[must_use]
    pub fn entity_id(&self) -> EntityId<E> {
        // SAFETY: self.entity is guaranteed to be of type E by construction.
        unsafe { EntityId::from_uuid(self.uuid) }
    }

    /// Convert to runtime `RuntimeFieldNodeId`.
    #[must_use]
    pub fn to_runtime(&self) -> RuntimeFieldNodeId {
        RuntimeFieldNodeId {
            field: self.field,
            uuid: self.uuid,
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

    // ── RuntimeFieldNodeId tests ────────────────────────────────────────────────

    #[test]
    fn test_runtime_field_node_id_of_equals_from_uuid() {
        let uuid = nnu(1);
        let entity_id = unsafe { EntityId::<MockEntity>::from_uuid(uuid) };
        let via_of = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, entity_id);
        // SAFETY: Test fixture uses matching entity type for the field.
        let via_from_uuid = unsafe { RuntimeFieldNodeId::from_uuid(uuid, &FIELD_A) };
        assert_eq!(via_of, via_from_uuid);
    }

    #[test]
    fn test_runtime_field_node_id_same_field_same_entity_equal() {
        let uuid = nnu(42);
        let entity_id = unsafe { EntityId::<MockEntity>::from_uuid(uuid) };
        let a = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, entity_id);
        let b = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, entity_id);
        assert_eq!(a, b);
    }

    #[test]
    fn test_runtime_field_node_id_same_field_different_entity_differ() {
        let a = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, unsafe {
            EntityId::<MockEntity>::from_uuid(nnu(1))
        });
        let b = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, unsafe {
            EntityId::<MockEntity>::from_uuid(nnu(2))
        });
        assert_ne!(a, b);
    }

    #[test]
    fn test_runtime_field_node_id_different_field_same_entity_differ() {
        let uuid = nnu(1);
        let entity_id = unsafe { EntityId::<MockEntity>::from_uuid(uuid) };
        let a = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, entity_id);
        let b = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_B, entity_id);
        assert_ne!(a, b);
    }

    #[test]
    fn test_runtime_field_node_id_hash_consistent() {
        use std::collections::HashSet;
        let uuid = nnu(7);
        let entity_id = unsafe { EntityId::<MockEntity>::from_uuid(uuid) };
        let node = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, entity_id);
        let mut set = HashSet::new();
        set.insert(node);
        assert!(set.contains(&RuntimeFieldNodeId::from_typed::<MockEntity>(
            &FIELD_A, entity_id
        )));
        assert!(!set.contains(&RuntimeFieldNodeId::from_typed::<MockEntity>(
            &FIELD_B, entity_id
        )));
    }

    // ── FieldNodeId<E> tests ─────────────────────────────────────────────────────

    #[test]
    fn test_typed_field_node_id_from_descriptor() {
        let uuid = nnu(1);
        let typed = FieldNodeId::from_descriptor(&FIELD_A, uuid);
        assert_eq!(typed.entity(), uuid);
    }

    #[test]
    fn test_typed_field_node_id_to_runtime() {
        let uuid = nnu(1);
        let typed = FieldNodeId::from_descriptor(&FIELD_A, uuid);
        let runtime = typed.to_runtime();
        assert_eq!(runtime.field.name(), FIELD_A.name);
        assert_eq!(runtime.uuid, uuid);
    }

    #[test]
    fn test_typed_field_node_id_from_entity_id() {
        let uuid = uuid::Uuid::new_v4();
        let entity_id: crate::entity::EntityId<MockEntity> =
            crate::entity::EntityId::new(uuid).expect("non-nil");
        let typed = unsafe { FieldNodeId::from_entity_id(&FIELD_A, entity_id) };
        assert_eq!(typed.entity(), entity_id.non_nil_uuid());
    }

    #[test]
    fn test_typed_field_node_id_entity_id_accessor() {
        let uuid = nnu(1);
        let typed = FieldNodeId::from_descriptor(&FIELD_A, uuid);
        let entity_id = typed.entity_id();
        assert_eq!(entity_id.non_nil_uuid(), uuid);
    }
}
