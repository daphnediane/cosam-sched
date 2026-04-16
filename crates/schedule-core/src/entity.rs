/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Entity type system — [`EntityType`] trait, [`EntityId`], [`EntityKind`],
//! [`RuntimeEntityId`], and [`UuidPreference`].
//!
//! Non-nil UUID identity uses [`uuid::NonNilUuid`] from the `uuid` crate
//! directly.

use crate::value::ValidationError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::marker::PhantomData;
use uuid::{NonNilUuid, Uuid};

// ── EntityKind ────────────────────────────────────────────────────────────────

/// Identifies which concrete entity type a UUID belongs to.
///
/// Used by [`RuntimeEntityId`] and by v5 UUID namespace selection in
/// [`UuidPreference`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityKind {
    Panel,
    Presenter,
    EventRoom,
    HotelRoom,
    PanelType,
}

impl EntityKind {
    /// The v5 UUID namespace for this entity kind.
    ///
    /// Each kind has a dedicated, fixed namespace so that deterministic IDs
    /// derived from natural keys (e.g. `"GP001"`) are unique across kinds.
    ///
    /// Namespaces were generated as v4 UUIDs and are stable for the lifetime
    /// of the project.
    #[must_use]
    pub fn uuid_namespace(self) -> Uuid {
        match self {
            Self::Panel => Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap(),
            Self::Presenter => Uuid::parse_str("b2c3d4e5-f6a7-8901-bcde-f12345678901").unwrap(),
            Self::EventRoom => Uuid::parse_str("c3d4e5f6-a7b8-9012-cdef-123456789012").unwrap(),
            Self::HotelRoom => Uuid::parse_str("d4e5f6a7-b8c9-0123-defa-234567890123").unwrap(),
            Self::PanelType => Uuid::parse_str("e5f6a7b8-c9d0-1234-efab-345678901234").unwrap(),
        }
    }
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Panel => "Panel",
            Self::Presenter => "Presenter",
            Self::EventRoom => "EventRoom",
            Self::HotelRoom => "HotelRoom",
            Self::PanelType => "PanelType",
        };
        write!(f, "{s}")
    }
}

// ── RuntimeEntityId ───────────────────────────────────────────────────────────

/// Dynamic (untyped) entity identifier — a non-nil UUID paired with its kind.
///
/// Use this when the entity type is not known at compile time, e.g. in
/// serialized change-log entries or mixed-kind search results.
/// For compile-time type safety use [`EntityId<E>`] instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeEntityId {
    pub kind: EntityKind,
    pub id: NonNilUuid,
}

impl RuntimeEntityId {
    /// Construct from a kind and a non-nil UUID.
    #[must_use]
    pub fn new(kind: EntityKind, id: NonNilUuid) -> Self {
        Self { kind, id }
    }
}

impl fmt::Display for RuntimeEntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.id)
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

impl UuidPreference {
    /// Resolve this preference into a concrete [`NonNilUuid`] given the
    /// `EntityKind` that owns the namespace for v5 derivation.
    #[must_use]
    pub fn resolve(self, kind: EntityKind) -> NonNilUuid {
        match self {
            Self::GenerateNew => {
                // SAFETY: Uuid::now_v7() sets version bits to 7; result is
                // never the nil UUID.
                unsafe { NonNilUuid::new_unchecked(Uuid::now_v7()) }
            }
            Self::FromV5 { name } => {
                let ns = kind.uuid_namespace();
                let uuid = Uuid::new_v5(&ns, name.as_bytes());
                // v5 UUIDs are never nil
                unsafe { NonNilUuid::new_unchecked(uuid) }
            }
            Self::Exact(id) => id,
        }
    }
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

    /// Return the static field registry for this entity type.
    fn field_set() -> &'static FieldSet<Self>;

    /// Produce the public export view from internal storage data.
    fn export(internal: &Self::InternalData) -> Self::Data;

    /// Validate internal data and return any constraint violations.
    fn validate(data: &Self::InternalData) -> Vec<ValidationError>;
}

// ── EntityId ──────────────────────────────────────────────────────────────────

/// Compile-time type-safe entity identifier.
///
/// Wraps a [`Uuid`] with a `PhantomData<fn() -> E>` so the type system
/// prevents mixing IDs from different entity types. The nil check in [`new`]
/// upholds the non-nil invariant that [`non_nil_uuid`] relies on.
///
/// `Clone` and `Copy` are implemented manually to avoid spurious
/// `E: Clone`/`E: Copy` bounds that derive macros would add.
///
/// [`new`]: EntityId::new
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
    /// This is the only public safe constructor — the nil check upholds the
    /// non-nil invariant that [`non_nil_uuid`](Self::non_nil_uuid) relies on.
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

    /// Return the underlying [`Uuid`].
    #[must_use]
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Return the UUID as a [`NonNilUuid`].
    ///
    /// Safe because [`EntityId::new`] rejects nil UUIDs, so `self.uuid` is
    /// guaranteed non-nil here.
    #[must_use]
    pub fn non_nil_uuid(&self) -> NonNilUuid {
        // SAFETY: EntityId::new rejects nil, so self.uuid is never nil.
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
        self.uuid.serialize(s)
    }
}

impl<'de, E: EntityType> Deserialize<'de> for EntityId<E> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let uuid = Uuid::deserialize(d)?;
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
    fn test_non_nil_uuid_copy_clone() {
        let nnu = make_non_nil_uuid();
        let copy = nnu;
        let clone = nnu.clone();
        assert_eq!(nnu, copy);
        assert_eq!(nnu, clone);
    }

    // ── EntityKind ──

    #[test]
    fn test_entity_kind_display() {
        assert_eq!(EntityKind::Panel.to_string(), "Panel");
        assert_eq!(EntityKind::Presenter.to_string(), "Presenter");
        assert_eq!(EntityKind::EventRoom.to_string(), "EventRoom");
        assert_eq!(EntityKind::HotelRoom.to_string(), "HotelRoom");
        assert_eq!(EntityKind::PanelType.to_string(), "PanelType");
    }

    #[test]
    fn test_entity_kind_serde_roundtrip() {
        let kinds = [
            EntityKind::Panel,
            EntityKind::Presenter,
            EntityKind::EventRoom,
            EntityKind::HotelRoom,
            EntityKind::PanelType,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let back: EntityKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn test_entity_kind_namespaces_are_distinct() {
        let ns: Vec<Uuid> = [
            EntityKind::Panel,
            EntityKind::Presenter,
            EntityKind::EventRoom,
            EntityKind::HotelRoom,
            EntityKind::PanelType,
        ]
        .iter()
        .map(|k| k.uuid_namespace())
        .collect();
        // All five namespaces must be distinct.
        for i in 0..ns.len() {
            for j in (i + 1)..ns.len() {
                assert_ne!(ns[i], ns[j], "namespaces {i} and {j} collide");
            }
        }
    }

    // ── RuntimeEntityId ──

    #[test]
    fn test_runtime_entity_id_display() {
        let nnu = make_non_nil_uuid();
        let rid = RuntimeEntityId::new(EntityKind::Panel, nnu);
        let s = rid.to_string();
        assert!(s.starts_with("Panel:"));
        assert!(s.contains(&nnu.to_string()));
    }

    #[test]
    fn test_runtime_entity_id_serde_roundtrip() {
        let rid = RuntimeEntityId::new(EntityKind::Presenter, make_non_nil_uuid());
        let json = serde_json::to_string(&rid).unwrap();
        let back: RuntimeEntityId = serde_json::from_str(&json).unwrap();
        assert_eq!(rid, back);
    }

    #[test]
    fn test_runtime_entity_id_copy_clone() {
        let rid = RuntimeEntityId::new(EntityKind::EventRoom, make_non_nil_uuid());
        let copy = rid;
        let clone = rid.clone();
        assert_eq!(rid, copy);
        assert_eq!(rid, clone);
    }

    // ── UuidPreference ──

    #[test]
    fn test_uuid_preference_generate_new_is_non_nil() {
        let id = UuidPreference::GenerateNew.resolve(EntityKind::Panel);
        assert!(!id.get().is_nil());
    }

    #[test]
    fn test_uuid_preference_from_v5_is_deterministic() {
        let pref1 = UuidPreference::FromV5 {
            name: "GP001".into(),
        };
        let pref2 = UuidPreference::FromV5 {
            name: "GP001".into(),
        };
        let id1 = pref1.resolve(EntityKind::Panel);
        let id2 = pref2.resolve(EntityKind::Panel);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_uuid_preference_from_v5_differs_by_kind() {
        let pref1 = UuidPreference::FromV5 {
            name: "same-name".into(),
        };
        let pref2 = UuidPreference::FromV5 {
            name: "same-name".into(),
        };
        let id_panel = pref1.resolve(EntityKind::Panel);
        let id_presenter = pref2.resolve(EntityKind::Presenter);
        assert_ne!(id_panel, id_presenter);
    }

    #[test]
    fn test_uuid_preference_exact_preserves_id() {
        let nnu = make_non_nil_uuid();
        let id = UuidPreference::Exact(nnu).resolve(EntityKind::Panel);
        assert_eq!(id, nnu);
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
        let nil_json = format!("\"{}\"", Uuid::nil());
        let result: Result<EntityId<MockEntity>, _> = serde_json::from_str(&nil_json);
        assert!(result.is_err());
    }
}
