/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! CRDT mapping spike for Phase 3 (META-004 / FEATURE-022).
//!
//! This crate is experimental and will be removed once the `schedule-core`
//! automerge integration lands. It proves that every [`FieldValue`] variant
//! can be stored in and retrieved from an automerge document using the
//! path layout and [`CrdtFieldType`] routing described in `docs/crdt-design.md`.
//!
//! Document layout:
//!
//! ```text
//! ROOT
//! └── entities (Map)
//!     └── {type_name} (Map)
//!         └── {uuid_string} (Map)
//!             ├── {field_name_A}   ← Scalar: ScalarValue
//!             ├── {field_name_B}   ← Text:   ObjType::Text
//!             ├── {field_name_C}   ← List:   ObjType::List of ScalarValue
//!             └── __deleted        ← soft delete flag (Scalar bool)
//! ```

use automerge::transaction::Transactable;
use automerge::{AutoCommit, ObjId, ObjType, ReadDoc, ScalarValue, Value, ROOT};
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use schedule_core::entity::RuntimeEntityId;
use schedule_core::value::{
    CrdtFieldType, FieldCardinality, FieldType, FieldTypeItem, FieldValue, FieldValueItem,
};
use std::fmt;
use uuid::{NonNilUuid, Uuid};

// ── Error ──────────────────────────────────────────────────────────────────

/// Errors from the spike mapping layer.
#[derive(Debug)]
pub enum SpikeError {
    /// Underlying automerge error.
    Automerge(automerge::AutomergeError),
    /// A value in the document did not match the expected shape.
    TypeMismatch(String),
    /// Load/save failed.
    Codec(String),
    /// `FieldValue` + `CrdtFieldType` combination is unsupported.
    Unsupported(String),
}

impl fmt::Display for SpikeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Automerge(e) => write!(f, "automerge error: {e}"),
            Self::TypeMismatch(s) => write!(f, "type mismatch: {s}"),
            Self::Codec(s) => write!(f, "codec error: {s}"),
            Self::Unsupported(s) => write!(f, "unsupported: {s}"),
        }
    }
}

impl std::error::Error for SpikeError {}

impl From<automerge::AutomergeError> for SpikeError {
    fn from(e: automerge::AutomergeError) -> Self {
        Self::Automerge(e)
    }
}

/// Shorthand result type for the spike.
pub type SpikeResult<T> = Result<T, SpikeError>;

// ── Document wrapper ───────────────────────────────────────────────────────

/// Thin wrapper around [`automerge::AutoCommit`] providing the
/// `FieldValue`-aware read/write API that the production code will eventually
/// provide via `schedule-core::crdt`.
pub struct CrdtDoc {
    /// The underlying automerge document.
    pub inner: AutoCommit,
}

impl Default for CrdtDoc {
    fn default() -> Self {
        Self::new()
    }
}

impl CrdtDoc {
    /// Create a new, empty document.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: AutoCommit::new(),
        }
    }

    /// Save to a byte vector.
    pub fn save(&mut self) -> Vec<u8> {
        self.inner.save()
    }

    /// Load from bytes.
    ///
    /// # Errors
    /// Returns `SpikeError::Codec` if the bytes do not decode.
    pub fn load(bytes: &[u8]) -> SpikeResult<Self> {
        let inner = AutoCommit::load(bytes).map_err(|e| SpikeError::Codec(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Fork into a divergent replica (deep copy + independent actor id).
    #[must_use]
    pub fn fork(&mut self) -> Self {
        Self {
            inner: self.inner.fork(),
        }
    }

    /// Merge another document's changes into this one.
    ///
    /// # Errors
    /// Forwards any automerge merge error.
    pub fn merge(&mut self, other: &mut Self) -> SpikeResult<()> {
        self.inner.merge(&mut other.inner)?;
        Ok(())
    }

    // ── Path helpers ────────────────────────────────────────────────────────

    /// Get-or-create a Map child on `parent` under `key`.
    fn ensure_map(&mut self, parent: &ObjId, key: &str) -> SpikeResult<ObjId> {
        match self.inner.get(parent, key)? {
            Some((Value::Object(ObjType::Map), id)) => Ok(id),
            Some((other, _)) => Err(SpikeError::TypeMismatch(format!(
                "{key}: expected Map, got {other:?}"
            ))),
            None => Ok(self.inner.put_object(parent, key, ObjType::Map)?),
        }
    }

    fn ensure_entity_map(&mut self, type_name: &str, uuid: Uuid) -> SpikeResult<ObjId> {
        let entities = self.ensure_map(&ROOT, "entities")?;
        let type_map = self.ensure_map(&entities, type_name)?;
        self.ensure_map(&type_map, &uuid.to_string())
    }

    fn get_entity_map(&self, type_name: &str, uuid: Uuid) -> Option<ObjId> {
        let entities = match self.inner.get(&ROOT, "entities").ok()?? {
            (Value::Object(ObjType::Map), id) => id,
            _ => return None,
        };
        let type_map = match self.inner.get(&entities, type_name).ok()?? {
            (Value::Object(ObjType::Map), id) => id,
            _ => return None,
        };
        match self.inner.get(&type_map, uuid.to_string()).ok()?? {
            (Value::Object(ObjType::Map), id) => Some(id),
            _ => None,
        }
    }

    // ── Entity lifecycle ────────────────────────────────────────────────────

    /// Ensure an entity exists in the document (creates an empty field map).
    ///
    /// # Errors
    /// Forwards any automerge error.
    pub fn create_entity(&mut self, type_name: &str, uuid: Uuid) -> SpikeResult<()> {
        self.ensure_entity_map(type_name, uuid)?;
        Ok(())
    }

    /// Soft-delete an entity (sets `__deleted = true`). The entity's field data
    /// is preserved so concurrent edits on other replicas can still observe it.
    ///
    /// # Errors
    /// Forwards any automerge error.
    pub fn soft_delete(&mut self, type_name: &str, uuid: Uuid, flag: bool) -> SpikeResult<()> {
        let entity = self.ensure_entity_map(type_name, uuid)?;
        self.inner
            .put(&entity, "__deleted", ScalarValue::Boolean(flag))?;
        Ok(())
    }

    /// Check soft-delete status. Returns `false` if entity absent or not flagged.
    #[must_use]
    pub fn is_deleted(&self, type_name: &str, uuid: Uuid) -> bool {
        let Some(entity) = self.get_entity_map(type_name, uuid) else {
            return false;
        };
        matches!(
            self.inner.get(&entity, "__deleted").ok().flatten(),
            Some((Value::Scalar(sv), _)) if matches!(sv.as_ref(), ScalarValue::Boolean(true))
        )
    }

    /// List all (non-deleted) entity UUIDs of a given type.
    #[must_use]
    pub fn list_entities(&self, type_name: &str) -> Vec<Uuid> {
        let Some((Value::Object(ObjType::Map), entities)) =
            self.inner.get(&ROOT, "entities").ok().flatten()
        else {
            return Vec::new();
        };
        let Some((Value::Object(ObjType::Map), type_map)) =
            self.inner.get(&entities, type_name).ok().flatten()
        else {
            return Vec::new();
        };
        self.inner
            .keys(&type_map)
            .filter_map(|k| Uuid::parse_str(&k).ok())
            .filter(|u| !self.is_deleted(type_name, *u))
            .collect()
    }

    // ── Field write ─────────────────────────────────────────────────────────

    /// Write a field value, routed by `crdt_type`.
    ///
    /// # Errors
    /// Returns `SpikeError::Unsupported` if the combination of `FieldValue`
    /// shape and `CrdtFieldType` is incoherent; forwards automerge errors.
    pub fn write_field(
        &mut self,
        type_name: &str,
        uuid: Uuid,
        field_name: &str,
        crdt_type: CrdtFieldType,
        value: &FieldValue,
    ) -> SpikeResult<()> {
        let entity = self.ensure_entity_map(type_name, uuid)?;
        match (crdt_type, value) {
            (CrdtFieldType::Derived, _) => Err(SpikeError::Unsupported(
                "Derived fields are not stored in CRDT".into(),
            )),
            (CrdtFieldType::Scalar, FieldValue::Single(item)) => {
                let sv = item_to_scalar(item)?;
                self.inner.put(&entity, field_name, sv)?;
                Ok(())
            }
            (CrdtFieldType::Scalar, FieldValue::List(_)) => Err(SpikeError::Unsupported(
                "Scalar CrdtFieldType requires FieldValue::Single".into(),
            )),
            (CrdtFieldType::Text, FieldValue::Single(FieldValueItem::Text(s))) => {
                write_text(&mut self.inner, &entity, field_name, s)
            }
            (CrdtFieldType::Text, _) => Err(SpikeError::Unsupported(
                "Text CrdtFieldType requires FieldValue::Single(Text)".into(),
            )),
            (CrdtFieldType::List, FieldValue::List(items)) => {
                write_list(&mut self.inner, &entity, field_name, items)
            }
            (CrdtFieldType::List, FieldValue::Single(_)) => Err(SpikeError::Unsupported(
                "List CrdtFieldType requires FieldValue::List".into(),
            )),
        }
    }

    /// Add a single element to a list-typed field (for concurrent-append
    /// scenarios; does not diff the whole list).
    ///
    /// # Errors
    /// Returns `SpikeError::Unsupported` if the field exists but isn't a list;
    /// forwards automerge errors.
    pub fn list_push(
        &mut self,
        type_name: &str,
        uuid: Uuid,
        field_name: &str,
        item: &FieldValueItem,
    ) -> SpikeResult<()> {
        let entity = self.ensure_entity_map(type_name, uuid)?;
        let list = match self.inner.get(&entity, field_name)? {
            Some((Value::Object(ObjType::List), id)) => id,
            Some((other, _)) => {
                return Err(SpikeError::TypeMismatch(format!(
                    "{field_name}: expected List, got {other:?}"
                )))
            }
            None => self.inner.put_object(&entity, field_name, ObjType::List)?,
        };
        let len = self.inner.length(&list);
        self.inner.insert(&list, len, item_to_scalar(item)?)?;
        Ok(())
    }

    /// Remove the first occurrence of an entity identifier from a list field.
    ///
    /// # Errors
    /// Returns `SpikeError::Unsupported` if the field isn't a list.
    pub fn list_remove_id(
        &mut self,
        type_name: &str,
        uuid: Uuid,
        field_name: &str,
        target: Uuid,
    ) -> SpikeResult<()> {
        let Some(entity) = self.get_entity_map(type_name, uuid) else {
            return Ok(());
        };
        let Some((Value::Object(ObjType::List), list)) = self.inner.get(&entity, field_name)?
        else {
            return Ok(());
        };
        let len = self.inner.length(&list);
        let needle = target.to_string();
        for i in 0..len {
            if let Some((Value::Scalar(sv), _)) = self.inner.get(&list, i)? {
                if let ScalarValue::Str(s) = sv.as_ref() {
                    if s.as_str() == needle {
                        self.inner.delete(&list, i)?;
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    // ── Field read ──────────────────────────────────────────────────────────

    /// Read a field back as a `FieldValue`, shaped by `field_type` and
    /// routed by `crdt_type`. Returns `None` if the field is absent.
    ///
    /// # Errors
    /// Returns an error if the stored shape does not match the requested type.
    pub fn read_field(
        &self,
        type_name: &str,
        uuid: Uuid,
        field_name: &str,
        field_type: FieldType,
        crdt_type: CrdtFieldType,
    ) -> SpikeResult<Option<FieldValue>> {
        let Some(entity) = self.get_entity_map(type_name, uuid) else {
            return Ok(None);
        };
        match crdt_type {
            CrdtFieldType::Derived => Ok(None),
            CrdtFieldType::Scalar => match self.inner.get(&entity, field_name)? {
                Some((Value::Scalar(sv), _)) => Ok(Some(FieldValue::Single(scalar_to_item(
                    sv.as_ref(),
                    field_type.item_type(),
                )?))),
                Some((other, _)) => Err(SpikeError::TypeMismatch(format!(
                    "{field_name}: expected Scalar, got {other:?}"
                ))),
                None => Ok(None),
            },
            CrdtFieldType::Text => match self.inner.get(&entity, field_name)? {
                Some((Value::Object(ObjType::Text), id)) => {
                    let s = self.inner.text(&id)?;
                    Ok(Some(FieldValue::Single(FieldValueItem::Text(s))))
                }
                Some((other, _)) => Err(SpikeError::TypeMismatch(format!(
                    "{field_name}: expected Text, got {other:?}"
                ))),
                None => Ok(None),
            },
            CrdtFieldType::List => match self.inner.get(&entity, field_name)? {
                Some((Value::Object(ObjType::List), list)) => {
                    let len = self.inner.length(&list);
                    let mut out = Vec::with_capacity(len);
                    for i in 0..len {
                        match self.inner.get(&list, i)? {
                            Some((Value::Scalar(sv), _)) => {
                                out.push(scalar_to_item(sv.as_ref(), field_type.item_type())?);
                            }
                            Some((other, _)) => {
                                return Err(SpikeError::TypeMismatch(format!(
                                    "{field_name}[{i}]: expected Scalar, got {other:?}"
                                )));
                            }
                            None => {}
                        }
                    }
                    Ok(Some(FieldValue::List(out)))
                }
                Some((other, _)) => Err(SpikeError::TypeMismatch(format!(
                    "{field_name}: expected List, got {other:?}"
                ))),
                None => Ok(None),
            },
        }
    }
}

// ── Scalar conversions ─────────────────────────────────────────────────────

fn item_to_scalar(item: &FieldValueItem) -> SpikeResult<ScalarValue> {
    Ok(match item {
        FieldValueItem::String(s) => ScalarValue::Str(s.clone().into()),
        FieldValueItem::Text(s) => ScalarValue::Str(s.clone().into()),
        FieldValueItem::Integer(n) => ScalarValue::Int(*n),
        FieldValueItem::Float(v) => ScalarValue::F64(*v),
        FieldValueItem::Boolean(b) => ScalarValue::Boolean(*b),
        FieldValueItem::DateTime(dt) => {
            let millis = Utc.from_utc_datetime(dt).timestamp_millis();
            ScalarValue::Timestamp(millis)
        }
        FieldValueItem::Duration(d) => ScalarValue::Int(d.num_milliseconds()),
        FieldValueItem::EntityIdentifier(rid) => {
            // Encode as "{type_name}:{uuid}" so reads can round-trip.
            ScalarValue::Str(format!("{}:{}", rid.type_name(), rid.uuid()).into())
        }
    })
}

fn scalar_to_item(sv: &ScalarValue, expected: FieldTypeItem) -> SpikeResult<FieldValueItem> {
    match (sv, expected) {
        (ScalarValue::Str(s), FieldTypeItem::String) => Ok(FieldValueItem::String(s.to_string())),
        (ScalarValue::Str(s), FieldTypeItem::Text) => Ok(FieldValueItem::Text(s.to_string())),
        (ScalarValue::Int(n), FieldTypeItem::Integer) => Ok(FieldValueItem::Integer(*n)),
        (ScalarValue::Uint(n), FieldTypeItem::Integer) => Ok(FieldValueItem::Integer(*n as i64)),
        (ScalarValue::F64(v), FieldTypeItem::Float) => Ok(FieldValueItem::Float(*v)),
        (ScalarValue::Boolean(b), FieldTypeItem::Boolean) => Ok(FieldValueItem::Boolean(*b)),
        (ScalarValue::Timestamp(ms), FieldTypeItem::DateTime) => {
            let dt: DateTime<Utc> = Utc
                .timestamp_millis_opt(*ms)
                .single()
                .ok_or_else(|| SpikeError::TypeMismatch(format!("bad timestamp: {ms}")))?;
            Ok(FieldValueItem::DateTime(dt.naive_utc()))
        }
        (ScalarValue::Int(ms), FieldTypeItem::Duration) => {
            Ok(FieldValueItem::Duration(Duration::milliseconds(*ms)))
        }
        (ScalarValue::Str(s), FieldTypeItem::EntityIdentifier(type_name)) => {
            let (got_type, uuid_part) = s
                .as_str()
                .split_once(':')
                .ok_or_else(|| SpikeError::TypeMismatch(format!("bad entity id: {s}")))?;
            if got_type != type_name {
                return Err(SpikeError::TypeMismatch(format!(
                    "entity id type mismatch: got {got_type}, expected {type_name}"
                )));
            }
            let u = Uuid::parse_str(uuid_part)
                .map_err(|e| SpikeError::TypeMismatch(format!("bad uuid in entity id: {e}")))?;
            let nn = NonNilUuid::new(u)
                .ok_or_else(|| SpikeError::TypeMismatch("entity id is nil UUID".into()))?;
            // SAFETY: caller promised the scalar stored at this field references
            // the given entity type_name; we just validated the UUID is non-nil.
            let rid = unsafe { RuntimeEntityId::from_uuid(nn, type_name) };
            Ok(FieldValueItem::EntityIdentifier(rid))
        }
        (sv, item) => Err(SpikeError::TypeMismatch(format!(
            "scalar {sv:?} does not match FieldTypeItem::{item}"
        ))),
    }
}

// ── Text + List write helpers ──────────────────────────────────────────────

fn write_text(
    doc: &mut AutoCommit,
    parent: &ObjId,
    field_name: &str,
    text: &str,
) -> SpikeResult<()> {
    // Replace-style write: create if absent, otherwise splice the delta as a
    // single full-range replace. Character-granular concurrent edits are
    // exercised directly via `doc.inner.splice_text` in the tests.
    let obj = match doc.get(parent, field_name)? {
        Some((Value::Object(ObjType::Text), id)) => id,
        Some((other, _)) => {
            return Err(SpikeError::TypeMismatch(format!(
                "{field_name}: expected Text, got {other:?}"
            )))
        }
        None => doc.put_object(parent, field_name, ObjType::Text)?,
    };
    let current_len = doc.length(&obj);
    if current_len > 0 {
        doc.splice_text(&obj, 0, current_len as isize, "")?;
    }
    doc.splice_text(&obj, 0, 0, text)?;
    Ok(())
}

fn write_list(
    doc: &mut AutoCommit,
    parent: &ObjId,
    field_name: &str,
    items: &[FieldValueItem],
) -> SpikeResult<()> {
    // Replace-style write: drop existing list if shape differs, recreate, and
    // insert each item. Concurrent per-element add/remove is exercised via
    // `list_push` / `list_remove_id`.
    if let Some((Value::Object(ObjType::List), id)) = doc.get(parent, field_name)? {
        let len = doc.length(&id);
        for i in (0..len).rev() {
            doc.delete(&id, i)?;
        }
        for (i, it) in items.iter().enumerate() {
            doc.insert(&id, i, item_to_scalar(it)?)?;
        }
        return Ok(());
    }
    let id = doc.put_object(parent, field_name, ObjType::List)?;
    for (i, it) in items.iter().enumerate() {
        doc.insert(&id, i, item_to_scalar(it)?)?;
    }
    Ok(())
}

// ── Convenience: known PanelType field shapes ──────────────────────────────

/// Descriptor of a single field for spike tests. Mirrors the pieces of
/// [`schedule_core::field::FieldDescriptor`] the CRDT layer cares about.
#[derive(Debug, Clone, Copy)]
pub struct SpikeField {
    pub name: &'static str,
    pub crdt: CrdtFieldType,
    pub ty: FieldType,
}

impl SpikeField {
    #[must_use]
    pub const fn scalar(name: &'static str, item: FieldTypeItem) -> Self {
        Self {
            name,
            crdt: CrdtFieldType::Scalar,
            ty: FieldType(FieldCardinality::Single, item),
        }
    }

    #[must_use]
    pub const fn text(name: &'static str) -> Self {
        Self {
            name,
            crdt: CrdtFieldType::Text,
            ty: FieldType(FieldCardinality::Single, FieldTypeItem::Text),
        }
    }

    #[must_use]
    pub const fn list(name: &'static str, item: FieldTypeItem) -> Self {
        Self {
            name,
            crdt: CrdtFieldType::List,
            ty: FieldType(FieldCardinality::List, item),
        }
    }
}

/// Subset of PanelType fields used by the spike round-trip test.
#[must_use]
pub fn panel_type_spike_fields() -> [SpikeField; 5] {
    [
        SpikeField::scalar("prefix", FieldTypeItem::String),
        SpikeField::scalar("panel_kind", FieldTypeItem::String),
        SpikeField::scalar("hidden", FieldTypeItem::Boolean),
        SpikeField::scalar("color", FieldTypeItem::String),
        SpikeField::scalar("sort_key", FieldTypeItem::Integer),
    ]
}

/// Return the logical entity `type_name` used by PanelType for the spike.
#[must_use]
pub const fn panel_type_type_name() -> &'static str {
    "panel_type"
}

/// Helper: parse a naive datetime for tests.
///
/// # Panics
/// Panics on an invalid literal (tests only).
#[must_use]
pub fn parse_ndt(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .expect("valid NaiveDateTime literal in tests")
}
