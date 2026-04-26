/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity type system — [`EntityType`] trait, [`EntityId`], [`RuntimeEntityId`],
//! and [`UuidPreference`].
//!
//! Non-nil UUID identity uses [`uuid::NonNilUuid`] from the `uuid` crate
//! directly.

use crate::value::ValidationError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::marker::PhantomData;
use uuid::{NonNilUuid, Uuid};

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
    pub uuid: NonNilUuid,
    pub type_name: &'static str,
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
    pub unsafe fn from_uuid(uuid: NonNilUuid, type_name: &'static str) -> Self {
        Self { uuid, type_name }
    }

    /// Get the UUID.
    #[must_use]
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        self.uuid
    }

    /// Get the static entity type name (e.g. `"panel"`, `"presenter"`).
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Convert from a typed `EntityId`.
    #[must_use]
    pub fn from_typed<E: EntityType>(id: EntityId<E>) -> Self {
        Self {
            uuid: id.non_nil_uuid(),
            type_name: E::TYPE_NAME,
        }
    }

    /// Try to convert to a typed `EntityId`.
    ///
    /// Returns `None` if the stored type name does not match `E::TYPE_NAME`.
    #[must_use]
    pub fn try_as_typed<E: EntityType>(&self) -> Option<EntityId<E>> {
        if self.type_name == E::TYPE_NAME {
            // SAFETY: type_name match confirms the UUID belongs to entity type E.
            Some(unsafe { EntityId::from_uuid(self.uuid) })
        } else {
            None
        }
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
        let type_name = registered_entity_types()
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

// ── UuidPreference ────────────────────────────────────────────────────────────

/// Controls UUID assignment when creating a new entity via a builder.
///
/// Most business logic should not name this type directly — it is a builder
/// concern.
#[derive(Debug, Clone)]
pub enum UuidPreference {
    /// Generate a fresh v7 (time-ordered) UUID.
    ///
    /// This is the default for new entities with no external natural key.
    GenerateNew,

    /// Derive a deterministic v5 UUID from the entity-type namespace and a
    /// natural-key string (e.g. `"GP001"`, a presenter name, a room name).
    ///
    /// Re-importing the same spreadsheet produces the same UUIDs.
    FromV5 { name: String },

    /// Use an exact, caller-supplied UUID.
    ///
    /// Use this when round-tripping a previously serialized entity so its
    /// identity is preserved unchanged.
    Exact(NonNilUuid),
}

// ── FieldSet ─────────────────────────────────────────────────────────────────

/// Re-export so callers can use `entity::FieldSet<E>` without importing `field_set`.
pub use crate::field_set::FieldSet;

// ── EntityType trait ──────────────────────────────────────────────────────────

/// Core trait implemented by every entity type singleton struct.
pub trait EntityType: 'static + Sized {
    /// Runtime storage struct; the field system operates on this.
    type InternalData: Clone + Send + Sync + fmt::Debug + 'static;

    /// Export/API view produced by [`EntityType::export`].
    type Data: Clone;

    /// Short, stable name for this entity type (e.g. `"panel_type"`).
    const TYPE_NAME: &'static str;

    /// The v5 UUID namespace for this entity type.
    ///
    /// This namespace is used for deterministic v5 UUID generation from
    /// natural keys (e.g., `"GP001"`). Each entity type has a unique,
    /// fixed namespace to ensure IDs derived from the same name are
    /// unique across types.
    ///
    /// Implementations should use a `static LazyLock<Uuid>` internally
    /// to compute the namespace once and return a reference.
    fn uuid_namespace() -> &'static Uuid;

    /// Return the static field registry for this entity type.
    fn field_set() -> &'static FieldSet<Self>;

    /// Produce the public export view from internal storage data.
    fn export(internal: &Self::InternalData) -> Self::Data;

    /// Validate internal data and return any constraint violations.
    fn validate(data: &Self::InternalData) -> Vec<ValidationError>;
}

// ── Inventory registration types ──────────────────────────────────────────────

/// Type alias for the entity build function used in edit commands.
///
/// Builds an entity with the given exact UUID and field name+value pairs.
/// Used by the edit command system to replay `AddEntity` / undo `RemoveEntity`.
pub type EntityBuildFn = fn(
    &mut crate::schedule::Schedule,
    NonNilUuid,
    &[(&'static str, crate::value::FieldValue)],
) -> Result<NonNilUuid, crate::builder::BuildError>;

/// Type-erased entity type descriptor, registered globally via `inventory`.
///
/// Each concrete entity type impl block submits one of these. Use
/// [`registered_entity_types`] to iterate all registered types at runtime.
pub struct RegisteredEntityType {
    /// Stable snake_case type name (e.g. `"panel"`, `"presenter"`).
    pub type_name: &'static str,
    /// Returns the UUID namespace used for deterministic v5 ID generation.
    pub uuid_namespace: fn() -> &'static Uuid,
    /// Returns the `TypeId` of this entity type's `InternalData` associated type.
    /// Used by `Schedule::identify` to map a bare UUID to its entity type.
    pub type_id: fn() -> std::any::TypeId,
    /// Build an entity with the given exact UUID and field name+value pairs.
    ///
    /// Used by the edit command system to replay `AddEntity` / undo `RemoveEntity`.
    /// The UUID is always used as-is (`UuidPreference::Exact`), guaranteeing
    /// that redo recreates the same identity.  Field names are canonical names
    /// or aliases registered in the entity's [`FieldSet`].
    ///
    /// Returns the resulting [`NonNilUuid`] on success, or a [`BuildError`] if
    /// any write or validation step fails.
    ///
    /// [`FieldSet`]: crate::field_set::FieldSet
    /// [`BuildError`]: crate::builder::BuildError
    pub build_fn: EntityBuildFn,

    /// Read a single field value from an existing entity by field name.
    ///
    /// Used by the edit command system to capture `old_value` before applying
    /// an `UpdateField` command.  Returns `None` if the field returns no value
    /// (unset optional), or `Err` if the field is write-only or the entity is
    /// absent.
    pub read_field_fn: fn(
        &crate::schedule::Schedule,
        NonNilUuid,
        &'static str,
    )
        -> Result<Option<crate::value::FieldValue>, crate::value::FieldError>,

    /// Write a single field value into an existing entity by field name.
    ///
    /// Used by the edit command system to apply and undo `UpdateField` commands.
    /// Returns `Err` if the field is read-only, the entity is absent, or the
    /// value conversion fails.
    pub write_field_fn: fn(
        &mut crate::schedule::Schedule,
        NonNilUuid,
        &'static str,
        crate::value::FieldValue,
    ) -> Result<(), crate::value::FieldError>,

    /// Snapshot all read+write fields of an existing entity into a
    /// `Vec<(&'static str, FieldValue)>`.
    ///
    /// Used by the edit command system to capture state before `RemoveEntity`,
    /// enabling undo to restore the entity via [`Self::build_fn`].
    /// Only fields that have both `read_fn` and `write_fn` are included;
    /// read-only computed fields and write-only modifier fields are skipped.
    /// Fields whose read returns `None` (unset optional fields) are also skipped.
    pub snapshot_fn:
        fn(&crate::schedule::Schedule, NonNilUuid) -> Vec<(&'static str, crate::value::FieldValue)>,

    /// Remove the entity with the given UUID from the schedule, clearing all edges.
    ///
    /// Used by the edit command system to apply `RemoveEntity` and undo `AddEntity`.
    pub remove_fn: fn(&mut crate::schedule::Schedule, NonNilUuid),

    /// Rehydrate an entity from the authoritative CRDT document into the
    /// in-memory cache.
    ///
    /// Reads every non-derived writable field for this entity type out of
    /// `schedule.doc()` via [`crate::crdt::read_field`], collects them into
    /// a `(field_name, FieldValue)` batch, and invokes
    /// [`crate::builder::build_entity`] with `UuidPreference::Exact(uuid)`.
    ///
    /// The caller is responsible for disabling the CRDT mirror
    /// ([`crate::schedule::Schedule::with_mirror_disabled`]) before calling
    /// this so rehydrated writes don't re-emit change records against the
    /// doc they were just read from.
    pub rehydrate_fn: fn(
        &mut crate::schedule::Schedule,
        NonNilUuid,
    ) -> Result<NonNilUuid, crate::builder::BuildError>,
}
inventory::collect!(RegisteredEntityType);

/// Iterate over all entity types registered via `inventory::submit!`.
pub fn registered_entity_types() -> impl Iterator<Item = &'static RegisteredEntityType> {
    inventory::iter::<RegisteredEntityType>()
}

// ── EntityId ──────────────────────────────────────────────────────────────────

/// Compile-time type-safe entity identifier.
///
/// Wraps a [`Uuid`] with a `PhantomData<fn() -> E>` so the type system
/// prevents mixing IDs from different entity types.
///
/// Constructors:
/// - [`from_preference`] — primary constructor for new entities; resolves a
///   [`UuidPreference`] using `E::uuid_namespace()`.
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
    uuid: Uuid,
    _marker: PhantomData<fn() -> E>,
}

impl<E: EntityType> Clone for EntityId<E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E: EntityType> Copy for EntityId<E> {}

impl<E: EntityType> EntityId<E> {
    /// Construct a typed entity ID, returning `None` if `uuid` is nil.
    ///
    /// Prefer [`from_preference`](Self::from_preference) for new entities.
    /// This constructor exists for deserialization and other cases where a
    /// bare `Uuid` must be validated.
    #[must_use]
    pub fn new(uuid: Uuid) -> Option<Self> {
        if uuid.is_nil() {
            None
        } else {
            Some(Self {
                uuid,
                _marker: PhantomData,
            })
        }
    }

    /// Create a typed entity ID by resolving a [`UuidPreference`].
    ///
    /// Uses [`E::uuid_namespace()`](EntityType::uuid_namespace) for deterministic
    /// v5 UUID generation, so the caller does not need to supply a namespace.
    #[must_use]
    pub fn from_preference(preference: UuidPreference) -> Self {
        let uuid: Uuid = match preference {
            UuidPreference::GenerateNew => Uuid::now_v7(),
            UuidPreference::FromV5 { name } => Uuid::new_v5(E::uuid_namespace(), name.as_bytes()),
            UuidPreference::Exact(id) => id.into(),
        };
        Self {
            uuid,
            _marker: PhantomData,
        }
    }

    /// Create an EntityId from a [`NonNilUuid`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that `uuid` actually identifies an entity of
    /// type `E`. Code that has a UUID→type registry (e.g. `Schedule`) can
    /// call this safely after verifying the type.
    #[must_use]
    pub unsafe fn from_uuid(uuid: NonNilUuid) -> Self {
        Self {
            uuid: uuid.into(),
            _marker: PhantomData,
        }
    }

    /// Return the underlying [`Uuid`].
    #[must_use]
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Return the UUID as a [`NonNilUuid`].
    ///
    /// Safe because all constructors uphold the non-nil invariant:
    /// [`new`](Self::new) rejects nil, [`from_preference`](Self::from_preference)
    /// produces v5/v7 UUIDs (never nil), and [`from_uuid`](Self::from_uuid)
    /// takes a `NonNilUuid`.
    #[must_use]
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        // SAFETY: all constructors guarantee self.uuid is never nil.
        unsafe { NonNilUuid::new_unchecked(self.uuid) }
    }
}

impl<E: EntityType> fmt::Debug for EntityId<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EntityId<{}>({:?})", E::TYPE_NAME, self.uuid)
    }
}

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
        EntityId::new(uuid).ok_or_else(|| serde::de::Error::custom("EntityId UUID must not be nil"))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
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

        fn field_set() -> &'static FieldSet<Self> {
            unimplemented!()
        }

        fn export(_: &Self::InternalData) -> Self::Data {
            MockData
        }

        fn validate(_: &Self::InternalData) -> Vec<ValidationError> {
            vec![]
        }
    }

    fn make_non_nil_uuid() -> NonNilUuid {
        // SAFETY: Uuid::now_v7() sets version bits to 7; result is never nil.
        unsafe { NonNilUuid::new_unchecked(Uuid::now_v7()) }
    }

    // ── NonNilUuid (uuid::NonNilUuid) ──

    #[test]
    fn test_non_nil_uuid_new_accepts_non_nil() {
        let u = Uuid::new_v4();
        assert!(NonNilUuid::new(u).is_some());
    }

    #[test]
    fn test_non_nil_uuid_new_rejects_nil() {
        assert!(NonNilUuid::new(Uuid::nil()).is_none());
    }

    #[test]
    fn test_non_nil_uuid_from_v7_is_non_nil() {
        let nnu = make_non_nil_uuid();
        assert!(!nnu.get().is_nil());
    }

    #[test]
    fn test_non_nil_uuid_get_roundtrip() {
        let raw = Uuid::new_v4();
        let nnu = NonNilUuid::new(raw).unwrap();
        assert_eq!(nnu.get(), raw);
    }

    #[test]
    fn test_non_nil_uuid_display() {
        let raw = Uuid::new_v4();
        let nnu = NonNilUuid::new(raw).unwrap();
        assert_eq!(nnu.to_string(), raw.to_string());
    }

    #[test]
    fn test_non_nil_uuid_serde_roundtrip() {
        let nnu = make_non_nil_uuid();
        let json = serde_json::to_string(&nnu).unwrap();
        let back: NonNilUuid = serde_json::from_str(&json).unwrap();
        assert_eq!(nnu, back);
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_non_nil_uuid_copy_clone() {
        let nnu = make_non_nil_uuid();
        let copy = nnu;
        let clone = nnu.clone();
        assert_eq!(nnu, copy);
        assert_eq!(nnu, clone);
    }

    // ── RuntimeEntityId ──

    #[test]
    fn test_runtime_entity_id_from_uuid() {
        let nnu = make_non_nil_uuid();
        // SAFETY: test-only; no real registry to verify against.
        let rid = unsafe { RuntimeEntityId::from_uuid(nnu, "TestEntity") };
        assert_eq!(rid.non_nil_uuid(), nnu);
        assert_eq!(rid.type_name(), "TestEntity");
    }

    #[test]
    fn test_runtime_entity_id_from_typed() {
        let nnu = make_non_nil_uuid();
        // SAFETY: test controls the type; nnu is for MockEntity.
        let typed_id = unsafe { EntityId::<MockEntity>::from_uuid(nnu) };
        let rid = RuntimeEntityId::from_typed(typed_id);
        assert_eq!(rid.non_nil_uuid(), nnu);
        assert_eq!(rid.type_name(), "mock");
    }

    #[test]
    fn test_runtime_entity_id_try_as_typed_matching() {
        let nnu = make_non_nil_uuid();
        // SAFETY: test controls the type; nnu is for MockEntity.
        let typed_id = unsafe { EntityId::<MockEntity>::from_uuid(nnu) };
        let rid = RuntimeEntityId::from_typed(typed_id);
        let back: Option<EntityId<MockEntity>> = rid.try_as_typed();
        assert!(back.is_some());
        assert_eq!(back.unwrap().non_nil_uuid(), nnu);
    }

    #[test]
    fn test_runtime_entity_id_try_as_typed_non_matching() {
        let nnu = make_non_nil_uuid();
        // SAFETY: test-only; deliberately mismatched type name.
        let rid = unsafe { RuntimeEntityId::from_uuid(nnu, "OtherEntity") };
        let back: Option<EntityId<MockEntity>> = rid.try_as_typed();
        assert!(back.is_none());
    }

    #[test]
    fn test_runtime_entity_id_display() {
        let nnu = make_non_nil_uuid();
        // SAFETY: test-only; no real registry to verify against.
        let rid = unsafe { RuntimeEntityId::from_uuid(nnu, "Panel") };
        let s = rid.to_string();
        assert!(s.starts_with("Panel:"));
        assert!(s.contains(&nnu.to_string()));
    }

    #[test]
    fn test_runtime_entity_id_serde_roundtrip() {
        let rid = unsafe {
            RuntimeEntityId::from_uuid(
                make_non_nil_uuid(),
                crate::presenter::PresenterEntityType::TYPE_NAME,
            )
        };
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
        let rid = unsafe {
            RuntimeEntityId::from_uuid(
                make_non_nil_uuid(),
                crate::event_room::EventRoomEntityType::TYPE_NAME,
            )
        };
        let clone = rid.clone();
        assert_eq!(rid, clone);
    }

    // ── EntityId::from_preference ──

    #[test]
    fn test_from_preference_generate_new_is_non_nil() {
        let id = EntityId::<MockEntity>::from_preference(UuidPreference::GenerateNew);
        assert!(!id.uuid().is_nil());
    }

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
        assert_eq!(id.non_nil_uuid(), nnu);
    }

    // ── EntityId ──

    #[test]
    fn test_entity_id_new_accepts_non_nil() {
        assert!(EntityId::<MockEntity>::new(Uuid::new_v4()).is_some());
    }

    #[test]
    fn test_entity_id_new_rejects_nil() {
        assert!(EntityId::<MockEntity>::new(Uuid::nil()).is_none());
    }

    #[test]
    fn test_entity_id_uuid_roundtrip() {
        let raw = Uuid::new_v4();
        let id = EntityId::<MockEntity>::new(raw).unwrap();
        assert_eq!(id.uuid(), raw);
    }

    #[test]
    fn test_entity_id_non_nil_uuid() {
        let raw = Uuid::new_v4();
        let id = EntityId::<MockEntity>::new(raw).unwrap();
        assert_eq!(id.non_nil_uuid().get(), raw);
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_entity_id_copy_clone() {
        let id = EntityId::<MockEntity>::new(Uuid::new_v4()).unwrap();
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
        let a = EntityId::<MockEntity>::new(raw).unwrap();
        let b = EntityId::<MockEntity>::new(raw).unwrap();
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn test_entity_id_display() {
        let raw = Uuid::new_v4();
        let id = EntityId::<MockEntity>::new(raw).unwrap();
        assert_eq!(id.to_string(), raw.to_string());
    }

    #[test]
    fn test_entity_id_debug() {
        let raw = Uuid::new_v4();
        let id = EntityId::<MockEntity>::new(raw).unwrap();
        let s = format!("{id:?}");
        assert!(s.contains("EntityId<mock>"));
    }

    #[test]
    fn test_entity_id_serde_roundtrip() {
        let raw = Uuid::new_v4();
        let id = EntityId::<MockEntity>::new(raw).unwrap();
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
        let id = EntityId::<MockEntity>::new(raw).unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"mock:{raw}\""));
    }

    #[test]
    fn test_runtime_entity_id_serde_format() {
        let nnu = make_non_nil_uuid();
        let rid = unsafe { RuntimeEntityId::from_uuid(nnu, "panel") };
        let json = serde_json::to_string(&rid).unwrap();
        assert_eq!(json, format!("\"panel:{}\"", nnu.get()));
    }

    #[test]
    fn test_registered_entity_types_contains_all_five() {
        let names: Vec<&'static str> = registered_entity_types().map(|r| r.type_name).collect();
        for expected in &[
            "panel",
            "presenter",
            "event_room",
            "hotel_room",
            "panel_type",
        ] {
            assert!(
                names.contains(expected),
                "registered_entity_types() missing \"{expected}\"; got {names:?}"
            );
        }
        assert_eq!(names.len(), 5, "expected exactly 5 registered entity types");
    }
}
