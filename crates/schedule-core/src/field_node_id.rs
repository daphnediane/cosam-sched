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

use crate::entity::{
    DynamicEntityId, EntityId, EntityType, EntityTyped, EntityUuid, TypedEntityId,
};
use crate::field::{FieldDescriptor, NamedField};
use crate::value::ConversionError;
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

// DynamicFieldNodeId

/// Common interface for accessing field node properties across different ID types.
///
/// This trait extends [`DynamicEntityId`] to add field descriptor access.
/// It provides a uniform way to extract the field descriptor from both
/// compile-time typed IDs (`FieldNodeId<E>`) and runtime dynamic IDs
/// (`RuntimeFieldNodeId`).
///
/// # Implementors
///
/// - [`RuntimeFieldNodeId`] - dynamic (untyped) field node identifier
/// - [`FieldNodeId<E>`] - compile-time type-safe field node identifier
///
/// # Example
///
/// ```ignore
/// fn print_field_info(node: impl DynamicFieldNodeId) {
///     println!("Entity UUID: {}", node.non_nil_uuid());
///     println!("Entity Type: {}", node.type_name());
///     println!("Field: {}", node.field().name());
/// }
/// ```
pub trait DynamicFieldNodeId: DynamicEntityId {
    /// Get the field descriptor as a trait object.
    fn field(&self) -> &'static dyn NamedField;

    /// Try to get the field descriptor as a typed field descriptor.
    ///
    /// Returns `None` if the field's entity type does not match `E`.
    #[must_use]
    fn try_as_typed_field<E: EntityType>(&self) -> Option<&'static FieldDescriptor<E>>;
}

// Marker trait for compile-time typed field node IDs
pub trait TypedFieldNodeId<E: EntityType>: DynamicFieldNodeId + TypedEntityId<E> {
    /// Get the field descriptor as a trait object.
    fn typed_field(&self) -> &'static FieldDescriptor<E>;
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
    uuid: NonNilUuid,
    /// Which field on the entity this endpoint represents.
    field: &'static dyn NamedField,
}

impl EntityUuid for RuntimeFieldNodeId {
    fn entity_uuid(&self) -> NonNilUuid {
        self.uuid
    }
}

impl EntityTyped for RuntimeFieldNodeId {
    fn entity_type_name(&self) -> &'static str {
        self.field.entity_type_name()
    }
}

impl DynamicFieldNodeId for RuntimeFieldNodeId {
    fn field(&self) -> &'static dyn NamedField {
        self.field
    }

    fn try_as_typed_field<E: EntityType>(&self) -> Option<&'static FieldDescriptor<E>> {
        // Try to downcast the trait object to a typed field descriptor
        // This only succeeds if the field's entity type matches E
        if self.field.entity_type_name() == E::TYPE_NAME {
            (self.field as &dyn std::any::Any).downcast_ref::<FieldDescriptor<E>>()
        } else {
            None
        }
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
    pub unsafe fn new_unchecked(uuid: NonNilUuid, field: &'static dyn NamedField) -> Self {
        Self { uuid, field }
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
            uuid: uuid.entity_uuid(),
        }
    }

    /// Try to convert to a typed `FieldNodeId<E>`.
    ///
    /// Returns `None` if the field's entity type name does not match `E::TYPE_NAME`.
    #[must_use]
    pub fn try_as_typed<E: EntityType>(&self) -> Option<FieldNodeId<E>> {
        FieldNodeId::try_new(*self, self.field)
    }

    /// Construct from a DynamicEntityId and a field descriptor.
    ///
    /// This is a convenience constructor for converting from any entity ID type
    /// to a RuntimeFieldNodeId for a specific field.
    #[must_use]
    pub fn from_dynamic<T: DynamicEntityId>(entity: T, field: &'static dyn NamedField) -> Self {
        Self {
            field,
            uuid: entity.entity_uuid(),
        }
    }
}

impl From<RuntimeFieldNodeId> for crate::entity_id::RuntimeEntityId {
    fn from(node: RuntimeFieldNodeId) -> Self {
        // SAFETY: RuntimeFieldNodeId's type_name() always returns a valid entity type name
        unsafe {
            crate::entity_id::RuntimeEntityId::new_unchecked(node.uuid, node.entity_type_name())
        }
    }
}

impl<E: EntityType> TryFrom<RuntimeFieldNodeId> for EntityId<E> {
    type Error = ConversionError;

    fn try_from(id: RuntimeFieldNodeId) -> Result<Self, Self::Error> {
        Self::try_from_dynamic(id).ok_or(ConversionError::WrongVariant {
            expected: E::TYPE_NAME,
            got: id.entity_type_name(),
        })
    }
}

impl<E: EntityType> TryFrom<RuntimeFieldNodeId> for FieldNodeId<E> {
    type Error = ConversionError;

    fn try_from(id: RuntimeFieldNodeId) -> Result<Self, Self::Error> {
        FieldNodeId::<E>::try_new(id, id.field).ok_or(ConversionError::WrongVariant {
            expected: E::TYPE_NAME,
            got: id.entity_type_name(),
        })
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

impl<E: EntityType> EntityUuid for FieldNodeId<E> {
    fn entity_uuid(&self) -> NonNilUuid {
        self.uuid
    }
}

impl<E: EntityType> EntityTyped for FieldNodeId<E> {
    fn entity_type_name(&self) -> &'static str {
        self.field.entity_type_name()
    }
}

impl<E: EntityType> TypedEntityId<E> for FieldNodeId<E> {}

impl<E: EntityType> DynamicFieldNodeId for FieldNodeId<E> {
    fn field(&self) -> &'static dyn NamedField {
        self.field
    }

    fn try_as_typed_field<F: EntityType>(&self) -> Option<&'static FieldDescriptor<F>> {
        // Return Some if the field's entity type matches F
        if E::TYPE_NAME == F::TYPE_NAME {
            (self.field as &dyn std::any::Any).downcast_ref::<FieldDescriptor<F>>()
        } else {
            None
        }
    }
}

impl<E: EntityType> TypedFieldNodeId<E> for FieldNodeId<E> {
    fn typed_field(&self) -> &'static FieldDescriptor<E> {
        self.field
    }
}

impl<E: EntityType> FieldNodeId<E> {
    /// Create a field node ID without validation.
    ///
    /// # Safety
    ///
    /// Caller must ensure `uuid` actually identifies an entity of type E.
    /// Code that has a UUID-type registry can use this to create field node IDs
    /// without the overhead of validation.
    #[must_use]
    pub unsafe fn new_unchecked(uuid: NonNilUuid, field: &'static FieldDescriptor<E>) -> Self {
        Self {
            uuid,
            field,
            _marker: PhantomData,
        }
    }

    /// Create a typed field node ID from a typed entity ID and field descriptor.
    ///
    /// This is a safe constructor that uses compile-time type guarantees.
    /// The entity ID must implement `TypedEntityId<E>` and the field descriptor
    /// must be a `FieldDescriptor<E>`, ensuring type safety at compile time.
    #[must_use]
    pub fn new(uuid: impl TypedEntityId<E>, field: &'static FieldDescriptor<E>) -> Self {
        Self {
            uuid: uuid.entity_uuid(),
            field,
            _marker: PhantomData,
        }
    }

    /// Try to create a typed field node ID from a dynamic entity ID and field descriptor.
    ///
    /// Returns `None` if either:
    /// - The field descriptor's entity type does not match `E`
    /// - The dynamic entity ID's type name does not match `E::TYPE_NAME`
    ///
    /// This is a safe constructor that performs runtime type checking using the
    /// `Any` trait for downcasting the field descriptor.
    #[must_use]
    pub fn try_new(uuid: impl DynamicEntityId, field: &'static dyn NamedField) -> Option<Self> {
        let field = (field as &dyn std::any::Any).downcast_ref::<FieldDescriptor<E>>()?;
        if uuid.entity_type_name() == E::TYPE_NAME {
            Some(Self {
                uuid: uuid.entity_uuid(),
                field,
                _marker: PhantomData,
            })
        } else {
            None
        }
    }

    pub fn try_from_dynamic(id: impl DynamicFieldNodeId) -> Option<Self> {
        let field = id.field();
        Self::try_new(id, field)
    }

    pub fn from_typed<T: TypedFieldNodeId<E>>(id: T) -> Self {
        let field = id.typed_field();
        Self::new(id, field)
    }
}

impl<E: EntityType> From<FieldNodeId<E>> for crate::entity_id::RuntimeEntityId {
    fn from(node: FieldNodeId<E>) -> Self {
        // SAFETY: FieldNodeId<E>'s type_name() always returns E::TYPE_NAME
        unsafe {
            crate::entity_id::RuntimeEntityId::new_unchecked(node.uuid, node.entity_type_name())
        }
    }
}

impl<E: EntityType> From<FieldNodeId<E>> for crate::entity_id::EntityId<E> {
    fn from(node: FieldNodeId<E>) -> Self {
        // SAFETY: FieldNodeId<E>'s type_name() always returns E::TYPE_NAME
        unsafe { crate::entity_id::EntityId::new_unchecked(node.uuid) }
    }
}

impl<E: EntityType> From<FieldNodeId<E>> for RuntimeFieldNodeId {
    fn from(node: FieldNodeId<E>) -> Self {
        // SAFETY: FieldNodeId<E>'s type_name() always returns E::TYPE_NAME
        unsafe { RuntimeFieldNodeId::new_unchecked(node.uuid, node.field) }
    }
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
        let entity_id = unsafe { EntityId::<MockEntity>::new_unchecked(uuid) };
        let via_of = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, entity_id);
        // SAFETY: Test fixture uses matching entity type for the field.
        let via_from_uuid = unsafe { RuntimeFieldNodeId::new_unchecked(uuid, &FIELD_A) };
        assert_eq!(via_of, via_from_uuid);
    }

    #[test]
    fn test_runtime_field_node_id_same_field_same_entity_equal() {
        let uuid = nnu(42);
        let entity_id = unsafe { EntityId::<MockEntity>::new_unchecked(uuid) };
        let a = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, entity_id);
        let b = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, entity_id);
        assert_eq!(a, b);
    }

    #[test]
    fn test_runtime_field_node_id_same_field_different_entity_differ() {
        let a = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, unsafe {
            EntityId::<MockEntity>::new_unchecked(nnu(1))
        });
        let b = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, unsafe {
            EntityId::<MockEntity>::new_unchecked(nnu(2))
        });
        assert_ne!(a, b);
    }

    #[test]
    fn test_runtime_field_node_id_different_field_same_entity_differ() {
        let uuid = nnu(1);
        let entity_id = unsafe { EntityId::<MockEntity>::new_unchecked(uuid) };
        let a = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_A, entity_id);
        let b = RuntimeFieldNodeId::from_typed::<MockEntity>(&FIELD_B, entity_id);
        assert_ne!(a, b);
    }

    #[test]
    fn test_runtime_field_node_id_hash_consistent() {
        use std::collections::HashSet;
        let uuid = nnu(7);
        let entity_id = unsafe { EntityId::<MockEntity>::new_unchecked(uuid) };
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
        let typed = unsafe { FieldNodeId::new_unchecked(uuid, &FIELD_A) };
        assert_eq!(typed.entity_uuid(), uuid);
    }

    #[test]
    fn test_typed_field_node_id_to_runtime() {
        let uuid = nnu(1);
        let typed = unsafe { FieldNodeId::new_unchecked(uuid, &FIELD_A) };
        let runtime: RuntimeFieldNodeId = typed.into();
        assert_eq!(runtime.field.name(), FIELD_A.name);
        assert_eq!(runtime.uuid, uuid);
    }

    #[test]
    fn test_typed_field_node_id_from_entity_id() {
        let uuid = uuid::Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        let entity_id: crate::entity::EntityId<MockEntity> =
            unsafe { crate::entity::EntityId::new_unchecked(non_nil_uuid) };
        let typed = FieldNodeId::new(entity_id, &FIELD_A);
        assert_eq!(typed.entity_uuid(), entity_id.entity_uuid());
    }

    #[test]
    fn test_typed_field_node_id_entity_id_accessor() {
        let uuid = nnu(1);
        let typed = unsafe { FieldNodeId::new_unchecked(uuid, &FIELD_A) };
        let entity_id: crate::entity::EntityId<MockEntity> = typed.into();
        assert_eq!(entity_id.entity_uuid(), uuid);
    }
}
