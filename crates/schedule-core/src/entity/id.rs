/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! [`EntityId`] and [`RuntimeEntityId`] — entity identifier types.
//!
//! These types form the foundation of the entity identity system:
//!
//! - [`EntityId<E>`] — compile-time type-safe entity identifier
//! - [`RuntimeEntityId`] — dynamic (untyped) entity identifier
//!
//! Entity identifiers are defined in [`crate::entity::id`]:
//! - [`crate::entity::id::EntityId`] — compile-time type-safe entity identifier
//! - [`crate::entity::id::RuntimeEntityId`] — dynamic (untyped) entity identifier
//!
//! Non-nil UUID identity uses [`uuid::NonNilUuid`] from the `uuid` crate
//! directly.
use crate::{entity::EntityType, value::ConversionError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::marker::PhantomData;
use uuid::{NonNilUuid, Uuid};

/// Trait for types that hold a UUID.
///
/// This trait provides a uniform way to extract the UUID from different ID types,
/// including both compile-time typed IDs (`EntityId<E>`) and runtime dynamic IDs (`RuntimeEntityId`).
///
/// # Implementors
///
/// - [`RuntimeEntityId`] - dynamic (untyped) entity identifier
/// - [`EntityId<E>`] - compile-time type-safe entity identifier
pub trait EntityUuid {
    /// Get the raw UUID.
    fn entity_uuid(&self) -> NonNilUuid;
}

/// Trait for types that have an associated entity type.
///
/// This trait provides a uniform way to extract the entity type name from different
/// ID types, including both compile-time typed IDs (`EntityId<E>`) and runtime dynamic IDs (`RuntimeEntityId`).
///
/// # Implementors
///
/// - [`RuntimeEntityId`] - dynamic (untyped) entity identifier
/// - [`EntityId<E>`] - compile-time type-safe entity identifier
pub trait EntityTyped {
    /// Get the static entity type name (e.g. `"panel"`, `"presenter"`).
    fn entity_type_name(&self) -> &'static str;
}

/// Common interface for accessing entity ID properties across different ID types.
///
/// This trait provides a uniform way to extract the UUID and type name from both
/// compile-time typed IDs (`EntityId<E>`) and runtime dynamic IDs (`RuntimeEntityId`).
/// It enables APIs to accept any ID type through `impl DynamicEntityId` without needing
/// to know the concrete type.
///
/// This trait is a combination of [`EntityUuid`] (for UUID access), [`EntityTyped`]
/// (for type name access), and [`Copy`] (required so id parameters can be used by
/// value multiple times without ownership gymnastics). A blanket implementation is
/// provided for any type that implements all three constituent traits.
///
/// # Implementors
///
/// - [`RuntimeEntityId`] - dynamic (untyped) entity identifier
/// - [`EntityId<E>`] - compile-time type-safe entity identifier
///
/// # Example
///
/// ```ignore
/// fn print_id_info(id: impl DynamicEntityId) {
///     println!("UUID: {}", id.entity_uuid());
///     println!("Type: {}", id.entity_type_name());
/// }
/// ```
pub trait DynamicEntityId: EntityUuid + EntityTyped + Copy {}

impl<T> DynamicEntityId for T where T: EntityUuid + EntityTyped + Copy {}

// ── RuntimeEntityId ───────────────────────────────────────────────────────────

/// Dynamic (untyped) entity identifier — a non-nil UUID paired with its entity type name.
///
/// Use this when the entity type is not known at compile time, e.g. in
/// serialized change-log entries or mixed-kind search results.
/// For compile-time type safety use [`EntityId<E>`] instead.
///
/// Serializes as the string `"<type_name>:<uuid>"`, matching the `Display` format
/// and [`EntityId<E>`]'s serialized form so both are human-readable and consistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeEntityId {
    /// The entity instance.
    uuid: NonNilUuid,
    /// The entity type name.
    type_name: &'static str,
}

impl EntityUuid for RuntimeEntityId {
    /// Get the entity UUID.
    fn entity_uuid(&self) -> NonNilUuid {
        self.uuid
    }
}

impl EntityTyped for RuntimeEntityId {
    /// Get the entity type name.
    fn entity_type_name(&self) -> &'static str {
        self.type_name
    }
}

impl RuntimeEntityId {
    /// Create a `RuntimeEntityId` from a non-nil UUID and a static entity type name.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `uuid` actually identifies an entity of
    /// type `type_name`. Code that has a UUID→type registry (e.g. `Schedule`) can
    /// call this safely after verifying the type.
    #[must_use]
    pub unsafe fn new_unchecked(uuid: NonNilUuid, type_name: &'static str) -> Self {
        Self { uuid, type_name }
    }

    /// Convert from any type implementing `DynamicEntityId`.
    #[must_use]
    pub fn from_dynamic<T: DynamicEntityId>(entity: T) -> Self {
        Self {
            uuid: entity.entity_uuid(),
            type_name: entity.entity_type_name(),
        }
    }
}

impl<E: EntityType> TryFrom<RuntimeEntityId> for EntityId<E> {
    type Error = ConversionError;

    fn try_from(id: RuntimeEntityId) -> Result<Self, Self::Error> {
        Self::try_from_dynamic(id).ok_or(ConversionError::WrongVariant {
            expected: E::TYPE_NAME,
            got: id.type_name,
        })
    }
}

impl fmt::Display for RuntimeEntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.type_name, self.uuid)
    }
}

impl Serialize for RuntimeEntityId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{}:{}", self.type_name, self.uuid))
    }
}

impl<'de> Deserialize<'de> for RuntimeEntityId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = std::string::String::deserialize(d)?;
        let (type_name_str, uuid_str) = raw
            .split_once(':')
            .ok_or_else(|| serde::de::Error::custom("expected \"<type>:<uuid>\""))?;
        let uuid = Uuid::parse_str(uuid_str)
            .map_err(|e| serde::de::Error::custom(format!("invalid UUID: {e}")))?;
        let nnu = NonNilUuid::new(uuid)
            .ok_or_else(|| serde::de::Error::custom("entity UUID must not be nil"))?;
        let type_name = crate::entity::registered_entity_types()
            .find(|r| r.type_name == type_name_str)
            .map(|r| r.type_name)
            .ok_or_else(|| {
                serde::de::Error::custom(format!("unknown entity type {type_name_str:?}"))
            })?;
        Ok(RuntimeEntityId {
            uuid: nnu,
            type_name,
        })
    }
}

// ── EntityId ──────────────────────────────────────────────────────────────────

/// Compile-time type-safe entity identifier.
///
/// Wraps a [`Uuid`] with a `PhantomData<fn() -> E>` so the type system
/// prevents mixing IDs from different entity types.
///
/// Constructors:
/// - [`from_preference`] — primary constructor for new entities; resolves a
///   [`crate::entity::UuidPreference`] using `E::uuid_namespace()`.
/// - [`new`] — validates a bare `Uuid` (rejects nil); for deserialization.
/// - [`from_uuid`] — `unsafe`; caller must ensure the UUID belongs to type `E`.
///
/// All constructors uphold the non-nil invariant that [`non_nil_uuid`] relies on.
///
/// `Clone` and `Copy` are implemented manually to avoid spurious
/// `E: Clone`/`E: Copy` bounds that derive macros would add.
///
/// [`from_preference`]: EntityId::from_preference
/// [`new`]: EntityId::new
/// [`from_uuid`]: EntityId::from_uuid
/// [`non_nil_uuid`]: EntityId::non_nil_uuid
#[derive(PartialEq, Eq, Hash)]
pub struct EntityId<E: EntityType> {
    uuid: NonNilUuid,
    _marker: PhantomData<fn() -> E>,
}

impl<E: EntityType> EntityUuid for EntityId<E> {
    fn entity_uuid(&self) -> NonNilUuid {
        self.uuid
    }
}

impl<E: EntityType> EntityTyped for EntityId<E> {
    fn entity_type_name(&self) -> &'static str {
        E::TYPE_NAME
    }
}

impl<E: EntityType> EntityId<E> {
    /// Create an EntityId from a [`NonNilUuid`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that `uuid` actually identifies an entity of
    /// type `E`. Code that has a UUID→type registry (e.g. `Schedule`) can
    /// call this safely after verifying the type.
    #[must_use]
    pub unsafe fn new_unchecked(uuid: NonNilUuid) -> Self {
        Self {
            uuid,
            _marker: PhantomData,
        }
    }

    /// Create a typed entity ID by resolving a [`crate::entity::UuidPreference`].
    ///
    /// Uses [`E::uuid_namespace()`](EntityType::uuid_namespace) for deterministic
    /// v5 UUID generation, so the caller does not need to supply a namespace.
    #[must_use]
    pub fn from_preference(preference: crate::entity::UuidPreference) -> Self {
        let uuid: NonNilUuid = match preference {
            // SAFETY: `Uuid::now_v7()` is guaranteed to be non-nil
            crate::entity::UuidPreference::GenerateNew => unsafe {
                NonNilUuid::new_unchecked(Uuid::now_v7())
            },
            // SAFETY: `Uuid::new_v5()` is guaranteed to be non-nil
            crate::entity::UuidPreference::FromV5 { name } => unsafe {
                NonNilUuid::new_unchecked(Uuid::new_v5(E::uuid_namespace(), name.as_bytes()))
            },
            crate::entity::UuidPreference::Exact(id) => id,
        };
        Self {
            uuid,
            _marker: PhantomData,
        }
    }

    pub fn try_from_dynamic(id: impl DynamicEntityId) -> Option<Self> {
        if id.entity_type_name() == E::TYPE_NAME {
            // SAFETY: The caller has verified that the type name matches E::TYPE_NAME
            Some(unsafe { Self::new_unchecked(id.entity_uuid()) })
        } else {
            None
        }
    }
}

impl<E: EntityType> From<EntityId<E>> for RuntimeEntityId {
    fn from(id: EntityId<E>) -> Self {
        Self {
            uuid: id.entity_uuid(),
            type_name: E::TYPE_NAME,
        }
    }
}

impl<E: EntityType> fmt::Debug for EntityId<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EntityId<{}>({:?})", E::TYPE_NAME, self.uuid)
    }
}

impl<E: EntityType> Clone for EntityId<E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E: EntityType> Copy for EntityId<E> {}

impl<E: EntityType> fmt::Display for EntityId<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

impl<E: EntityType> Serialize for EntityId<E> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{}:{}", E::TYPE_NAME, self.uuid))
    }
}

impl<'de, E: EntityType> Deserialize<'de> for EntityId<E> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = std::string::String::deserialize(d)?;
        let (type_name, uuid_str) = raw
            .split_once(':')
            .ok_or_else(|| serde::de::Error::custom("expected \"<type>:<uuid>\""))?;
        if type_name != E::TYPE_NAME {
            return Err(serde::de::Error::custom(format!(
                "expected type \"{}\", got \"{type_name}\"",
                E::TYPE_NAME
            )));
        }
        let uuid = Uuid::parse_str(uuid_str)
            .map_err(|e| serde::de::Error::custom(format!("invalid UUID: {e}")))?;
        let non_nil_uuid = NonNilUuid::new(uuid)
            .ok_or_else(|| serde::de::Error::custom("EntityId UUID must not be nil"))?;
        Ok(unsafe { EntityId::new_unchecked(non_nil_uuid) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityType, UuidPreference};
    use serde_json;

    // Minimal mock entity type for testing.
    #[derive(PartialEq, Eq, Hash)]
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

        fn field_set() -> &'static crate::field::set::FieldSet<Self> {
            unimplemented!()
        }

        fn export(_: &Self::InternalData) -> Self::Data {
            MockData
        }

        fn validate(_: &Self::InternalData) -> Vec<crate::value::ValidationError> {
            vec![]
        }
    }

    fn make_non_nil_uuid() -> NonNilUuid {
        // SAFETY: Uuid::now_v7() sets version bits to 7; result is never nil.
        unsafe { NonNilUuid::new_unchecked(Uuid::now_v7()) }
    }

    // ── RuntimeEntityId ──

    #[test]
    fn test_runtime_entity_id_from_uuid() {
        let nnu = make_non_nil_uuid();
        // SAFETY: test-only; no real registry to verify against.
        let rid = unsafe { RuntimeEntityId::new_unchecked(nnu, "TestEntity") };
        assert_eq!(rid.entity_uuid(), nnu);
        assert_eq!(rid.entity_type_name(), "TestEntity");
    }

    #[test]
    fn test_runtime_entity_id_from_typed() {
        let nnu = make_non_nil_uuid();
        // SAFETY: test controls the type; nnu is for MockEntity.
        let typed_id = unsafe { EntityId::<MockEntity>::new_unchecked(nnu) };
        let rid = RuntimeEntityId::from_dynamic(typed_id);
        assert_eq!(rid.entity_uuid(), nnu);
        assert_eq!(rid.entity_type_name(), "mock");
    }

    #[test]
    fn test_runtime_entity_id_try_as_typed_matching() {
        let nnu = make_non_nil_uuid();
        // SAFETY: test controls the type; nnu is for MockEntity.
        let typed_id = unsafe { EntityId::<MockEntity>::new_unchecked(nnu) };
        let rid = RuntimeEntityId::from_dynamic(typed_id);
        let back: Result<EntityId<MockEntity>, _> = rid.try_into();
        assert!(back.is_ok());
        assert_eq!(back.unwrap().entity_uuid(), nnu);
    }

    #[test]
    fn test_runtime_entity_id_try_as_typed_non_matching() {
        let nnu = make_non_nil_uuid();
        // SAFETY: test-only; deliberately mismatched type name.
        let rid = unsafe { RuntimeEntityId::new_unchecked(nnu, "OtherEntity") };
        let back: Result<EntityId<MockEntity>, _> = rid.try_into();
        assert!(back.is_err());
    }

    #[test]
    fn test_runtime_entity_id_display() {
        let nnu = make_non_nil_uuid();
        // SAFETY: test-only; no real registry to verify against.
        let rid = unsafe { RuntimeEntityId::new_unchecked(nnu, "Panel") };
        let s = rid.to_string();
        assert!(s.starts_with("Panel:"));
        assert!(s.contains(&nnu.to_string()));
    }

    #[test]
    fn test_runtime_entity_id_serde_roundtrip() {
        let rid = unsafe { RuntimeEntityId::new_unchecked(make_non_nil_uuid(), "presenter") };
        let json_string = serde_json::to_string(&rid).unwrap();
        let back: RuntimeEntityId = serde_json::from_str(&json_string).unwrap();
        assert_eq!(rid, back);
    }

    #[test]
    fn test_runtime_entity_id_deserialize_unknown_type_is_error() {
        let json = format!("\"unknown_type:{}\"", make_non_nil_uuid());
        let result: Result<RuntimeEntityId, _> = serde_json::from_str(&json);
        assert!(result.is_err());
    }

    #[test]
    #[allow(clippy::clone_on_copy)] // explicitly exercising the Clone impl
    fn test_runtime_entity_id_clone() {
        let rid = unsafe { RuntimeEntityId::new_unchecked(make_non_nil_uuid(), "event_room") };
        let clone = rid.clone();
        assert_eq!(rid, clone);
    }

    // ── EntityId::from_preference ──

    #[test]
    fn test_from_preference_from_v5_is_deterministic() {
        let id1 = EntityId::<MockEntity>::from_preference(UuidPreference::FromV5 {
            name: "GP001".into(),
        });
        let id2 = EntityId::<MockEntity>::from_preference(UuidPreference::FromV5 {
            name: "GP001".into(),
        });
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_from_preference_from_v5_differs_by_name() {
        let id1 = EntityId::<MockEntity>::from_preference(UuidPreference::FromV5 {
            name: "GP001".into(),
        });
        let id2 = EntityId::<MockEntity>::from_preference(UuidPreference::FromV5 {
            name: "GP002".into(),
        });
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_from_preference_exact_preserves_id() {
        let nnu = make_non_nil_uuid();
        let id = EntityId::<MockEntity>::from_preference(UuidPreference::Exact(nnu));
        assert_eq!(id.entity_uuid(), nnu);
    }

    // ── EntityId ──

    #[test]
    fn test_entity_id_new_accepts_non_nil() {
        let uuid = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        let _id = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
    }

    #[test]
    fn test_entity_id_uuid_roundtrip() {
        let raw = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(raw) };
        let id = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        assert_eq!(id.entity_uuid().get(), raw);
    }

    #[test]
    fn test_entity_id_non_nil_uuid() {
        let raw = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(raw) };
        let id = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        assert_eq!(id.entity_uuid().get(), raw);
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_entity_id_copy_clone() {
        let uuid = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(uuid) };
        let id = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        let copy = id;
        let clone = id.clone();
        assert_eq!(id, copy);
        assert_eq!(id, clone);
    }

    #[test]
    fn test_entity_id_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<EntityId<MockEntity>>();
    }

    #[test]
    fn test_entity_id_hash_eq() {
        use std::collections::HashSet;
        let raw = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(raw) };
        let a = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        let b = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn test_entity_id_display() {
        let raw = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(raw) };
        let id = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        assert_eq!(id.to_string(), raw.to_string());
    }

    #[test]
    fn test_entity_id_debug() {
        let raw = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(raw) };
        let id = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        let debug_str = format!("{:?}", id);
        assert!(debug_str.contains("EntityId"));
        assert!(debug_str.contains(&raw.to_string()));
    }

    #[test]
    fn test_entity_id_partial_eq() {
        let raw = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(raw) };
        let id = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        assert_eq!(id, id);
        let other_raw = Uuid::new_v4();
        let other_non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(other_raw) };
        let other_id = unsafe { EntityId::<MockEntity>::new_unchecked(other_non_nil_uuid) };
        assert_ne!(id, other_id);
    }

    #[test]
    fn test_entity_id_hash() {
        use std::hash::{Hash, Hasher};
        let raw = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(raw) };
        let id = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher);
        let hash1 = hasher.finish();

        let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
        id.hash(&mut hasher2);
        let hash2 = hasher2.finish();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_entity_id_serde() {
        let raw = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(raw) };
        let id = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        let json = serde_json::to_string(&id).unwrap();
        let back: EntityId<MockEntity> = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn test_entity_id_deserialize_nil_is_error() {
        let nil_json = format!("\"mock:{}\"", Uuid::nil());
        let result: Result<EntityId<MockEntity>, _> = serde_json::from_str(&nil_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_entity_id_deserialize_wrong_type_is_error() {
        let raw = Uuid::new_v4();
        let json = format!("\"other:{raw}\"");
        let result: Result<EntityId<MockEntity>, _> = serde_json::from_str(&json);
        assert!(result.is_err());
    }

    #[test]
    fn test_entity_id_serde_format() {
        let raw = Uuid::new_v4();
        let non_nil_uuid = unsafe { uuid::NonNilUuid::new_unchecked(raw) };
        let id = unsafe { EntityId::<MockEntity>::new_unchecked(non_nil_uuid) };
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"mock:{raw}\""));
    }

    #[test]
    fn test_runtime_entity_id_serde_format() {
        let nnu = make_non_nil_uuid();
        let rid = unsafe { RuntimeEntityId::new_unchecked(nnu, "panel") };
        let json = serde_json::to_string(&rid).unwrap();
        assert_eq!(json, format!("\"panel:{}\"", nnu.get()));
    }
}
